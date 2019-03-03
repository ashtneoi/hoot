use http::{
    Method,
    Uri,
    Version,
};
use http::header::{
    HeaderMap,
    HeaderName,
    HeaderValue,
    InvalidHeaderName,
    InvalidHeaderValue,
};
use http::method::InvalidMethod;
use http::uri::InvalidUriBytes;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use std::io;
use std::io::{BufRead, Read};

#[derive(Debug)]
pub struct Header {
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub fields: HeaderMap,
}

#[derive(Debug)]
pub enum InvalidHeader {
    Format,
    RequestLine(InvalidRequestLine),
    HeaderField(InvalidHeaderField),
    Io(io::Error),
}

impl From<InvalidRequestLine> for InvalidHeader {
    fn from(e: InvalidRequestLine) -> Self {
        InvalidHeader::RequestLine(e)
    }
}

impl From<InvalidHeaderField> for InvalidHeader {
    fn from(e: InvalidHeaderField) -> Self {
        InvalidHeader::HeaderField(e)
    }
}

impl From<io::Error> for InvalidHeader {
    fn from(e: io::Error) -> Self {
        InvalidHeader::Io(e)
    }
}

const LINE_CAP: usize = 16384;

pub fn parse_header<B: BufRead>(mut stream: B)
    -> Result<Header, InvalidHeader>
{
    // TODO: Why does removing the type from `line` here cause errors?
    let next_line = |stream: &mut B, line: &mut Vec<u8>| {
        line.clear();
        let count = stream
            .take(LINE_CAP as u64)
            .read_until('\n' as u8, line)?;
        match count {
            0 => Err(InvalidHeader::Format), // FIXME?
            LINE_CAP => Err(InvalidHeader::Format), // FIXME
            _ => Ok(()),
        }
    };

    let mut line = Vec::with_capacity(LINE_CAP);

    next_line(&mut stream, &mut line)?;
    if !line.ends_with(b"\r\n") {
        return Err(InvalidHeader::Format);
    }
    line.truncate(line.len() - 2);
    let (method, uri, version) = parse_request_line(&line[..])?;
    let mut fields = HeaderMap::new();
    loop {
        next_line(&mut stream, &mut line)?;
        if !line.ends_with(b"\r\n") {
            return Err(InvalidHeader::Format);
        }
        line.truncate(line.len() - 2);
        if line == b"" {
            return Ok(Header { method, uri, version, fields });
        }
        let (name, value) = parse_header_field(&line)?;
        fields.insert(name, value); // TODO: we should care about result, right?
    }
}

#[derive(Debug)]
pub enum InvalidRequestLine {
    Format,
    Method(InvalidMethod),
    Uri(InvalidUriBytes),
    Version,
}

impl From<InvalidMethod> for InvalidRequestLine {
    fn from(e: InvalidMethod) -> Self {
        InvalidRequestLine::Method(e)
    }
}

impl From<InvalidUriBytes> for InvalidRequestLine {
    fn from(e: InvalidUriBytes) -> Self {
        InvalidRequestLine::Uri(e)
    }
}

pub fn parse_request_line(s: &[u8])
    -> Result<(Method, Uri, Version), InvalidRequestLine>
{
    lazy_static! {
        static ref R: Regex = Regex::new(
            // method SP request-target SP HTTP-version
            r"(?-u)^(\S+) (\S+) (\S+)$"
        ).unwrap();
    }
    let cap = R.captures(s).ok_or(InvalidRequestLine::Format)?;
    Ok((
        Method::from_bytes(&cap[1])?,
        Uri::from_shared(cap[2].into())?,
        match &cap[3] {
            b"HTTP/0.9" => Version::HTTP_09,
            b"HTTP/1.0" => Version::HTTP_10,
            b"HTTP/1.1" => Version::HTTP_11,
            b"HTTP/2.0" => Version::HTTP_2,
            _ => return Err(InvalidRequestLine::Version),
        },
    ))
}

#[derive(Debug)]
pub enum InvalidHeaderField {
    Format,
    Name(InvalidHeaderName),
    Value(InvalidHeaderValue),
}

impl From<InvalidHeaderName> for InvalidHeaderField {
    fn from(e: InvalidHeaderName) -> Self {
        InvalidHeaderField::Name(e)
    }
}

impl From<InvalidHeaderValue> for InvalidHeaderField {
    fn from(e: InvalidHeaderValue) -> Self {
        InvalidHeaderField::Value(e)
    }
}

pub fn parse_header_field(s: &[u8])
    -> Result<(HeaderName, HeaderValue), InvalidHeaderField>
{
    // TODO: support obs-fold e.g. within message/http
    // (see rfc7230 section 3.2.4)

    // rfc7230 section 3.2.4: Server MUST return 400 if there's whitespace
    // between field name and colon.
    // rfc7230 section 3.2.4: If obs-fold is used outside a message/http body,
    // server MUST either return 400 or replace each such obs-fold with one or
    // more SP chars.
    lazy_static! {
        static ref R: Regex = Regex::new(concat!(
            // token ":" OWS *field-content OWS
            r"(?-u)^([!#$%&'*+.^_`|~0-9A-Za-z-]+):",
            r"[\t ]*([!-~\x80-\xFF]([\t !-~\x80-\xFF]*[!-~\x80-\xFF])?)[\t ]*$",
        )).unwrap();
    }
    let cap = R.captures(s).ok_or(InvalidHeaderField::Format)?;
    Ok((
        HeaderName::from_bytes(&cap[1])?,
        HeaderValue::from_bytes(&cap[2])?,
    ))
}

#[cfg(test)]
mod test {
    use crate::{
        parse_header,
        parse_request_line,
        parse_header_field,
    };
    use http::header::HeaderValue;
    use http::{
        Method,
        Version,
    };

    #[test]
    fn test_parse_header() {
        let mut s = Vec::new();
        // TODO: There's a better way to do this, right?
        s.extend(
            &b"POST http://foo.example.com/bar?qux=19&qux=xyz HTTP/1.1\r\n"[..]
        );
        s.extend(&b"Host: foo.example.com\r\n"[..]);
        s.extend(&b"Content-Type: application/json\r\n"[..]);
        s.extend(&b"\r\n"[..]);

        let h = parse_header(&s[..]).unwrap();
        assert_eq!(h.method, Method::POST);
        assert_eq!(h.uri.scheme_str().unwrap(), "http");
        assert_eq!(h.uri.host().unwrap(), "foo.example.com");
        assert_eq!(h.uri.port_part(), None);
        assert_eq!(h.uri.path(), "/bar");
        assert_eq!(h.uri.query().unwrap(), "qux=19&qux=xyz");
        assert_eq!(h.version, Version::HTTP_11);
        assert_eq!(h.fields["host"], "foo.example.com");
        assert_eq!(h.fields["content-type"], "application/json");
    }

    #[test]
    fn test_parse_request_line() {
        let s = b"OPTIONS * HTTP/1.1";
        let (m, u, v) = parse_request_line(s).unwrap();
        assert_eq!(m, Method::OPTIONS);
        assert_eq!(u.path(), "*");
        assert_eq!(v, Version::HTTP_11);

        let s = b"POST http://foo.example.com/bar?qux=19&qux=xyz HTTP/1.0";
        let (m, u, v) = parse_request_line(s).unwrap();
        assert_eq!(m, Method::POST);
        assert_eq!(u.scheme_str().unwrap(), "http");
        assert_eq!(u.host().unwrap(), "foo.example.com");
        assert_eq!(u.port_part(), None);
        assert_eq!(u.path(), "/bar");
        assert_eq!(u.query().unwrap(), "qux=19&qux=xyz");
        assert_eq!(v, Version::HTTP_10);
    }

    #[test]
    fn test_parse_header_field() {
        let s = b"Content-Type: application/json; charset=\"\xAA\xBB\xCC\"";
        let (h, v) = parse_header_field(s).unwrap();
        assert_eq!(
            h,
            http::header::CONTENT_TYPE,
        );
        assert_eq!(
            v,
            HeaderValue::from_bytes(
                &b"application/json; charset=\"\xAA\xBB\xCC\""[..]
            ).unwrap(),
        );
    }
}
