use http::{
    Method,
    StatusCode,
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
use std::io::{BufRead, BufWriter, Read, Write};

#[derive(Debug)]
pub struct RequestHeader {
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub fields: HeaderMap,
}

#[derive(Debug)]
pub struct ResponseHeader {
    pub status_code: StatusCode,
    pub version: Version,
    pub fields: HeaderMap,
}

#[derive(Debug)]
pub enum InvalidRequestHeader {
    Format,
    RequestLine(InvalidRequestLine),
    HeaderField(InvalidHeaderField),
    Io(io::Error),
}

impl From<InvalidRequestLine> for InvalidRequestHeader {
    fn from(e: InvalidRequestLine) -> Self {
        InvalidRequestHeader::RequestLine(e)
    }
}

impl From<InvalidHeaderField> for InvalidRequestHeader {
    fn from(e: InvalidHeaderField) -> Self {
        InvalidRequestHeader::HeaderField(e)
    }
}

impl From<io::Error> for InvalidRequestHeader {
    fn from(e: io::Error) -> Self {
        InvalidRequestHeader::Io(e)
    }
}

const LINE_CAP: usize = 16384;

pub fn parse_request_header<B: BufRead>(mut stream: B)
    -> Result<RequestHeader, InvalidRequestHeader>
{
    // TODO: Why does removing the type from `line` here cause errors?
    let next_line = |stream: &mut B, line: &mut Vec<u8>| {
        line.clear();
        let count = stream
            .take(LINE_CAP as u64)
            .read_until('\n' as u8, line)?;
        match count {
            0 => Err(InvalidRequestHeader::Format), // FIXME?
            LINE_CAP => Err(InvalidRequestHeader::Format), // FIXME
            _ => Ok(()),
        }
    };

    let mut line = Vec::with_capacity(LINE_CAP);

    next_line(&mut stream, &mut line)?;
    if !line.ends_with(b"\r\n") {
        return Err(InvalidRequestHeader::Format);
    }
    line.truncate(line.len() - 2);
    let (method, uri, version) = parse_request_line(&line[..])?;
    let mut fields = HeaderMap::new();
    loop {
        next_line(&mut stream, &mut line)?;
        if !line.ends_with(b"\r\n") {
            return Err(InvalidRequestHeader::Format);
        }
        line.truncate(line.len() - 2);
        if line == b"" {
            return Ok(RequestHeader { method, uri, version, fields });
        }
        let (name, value) = parse_header_field(&line)?;
        fields.insert(name, value); // TODO: we should care about result, right?
    }
}

pub fn write_response_header<W: Write>(header: &ResponseHeader, stream: W)
    -> io::Result<()>
{
    let mut stream = BufWriter::new(stream);

    // TODO: Is this the way you're supposed to format bytes?
    stream.write_all(
        match header.version {
            Version::HTTP_10 => b"HTTP/1.0",
            Version::HTTP_11 => b"HTTP/1.1",
            _ => panic!("Unsupported version"), // FIXME: Err? Or really panic?
        }
    )?;
    stream.write_all(b" ")?;
    stream.write_all(header.status_code.as_str().as_bytes())?;
    stream.write_all(b" ")?;
    stream.write_all(
        header
        .status_code
        .canonical_reason()
        .unwrap_or("Unknown Reason")
        .as_bytes()
    )?;
    // TODO: Write header fields.
    Ok(())
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
            // rfc 7230 section A: "Any server that implements name-based
            // virtual hosts ought to disable support for HTTP/0.9."
            b"HTTP/1.0" => Version::HTTP_10,
            b"HTTP/1.1" => Version::HTTP_11,
            // We don't support HTTP 0.9 or 2.0. 2.0 support may be added later.
            // FIXME: Can we respond to an invalid version with 505 HTTP
            // Version Not Supported? If not, unsupported major versions need a
            // different error than invalid versions.
            // FIXME: We should probably accept requests with version 1.2 and
            // higher. Check the spec.
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
        parse_request_header,
        parse_request_line,
        parse_header_field,
        ResponseHeader,
        write_response_header,
    };
    use http::header::{
        HeaderMap,
        HeaderValue,
    };
    use http::{
        Method,
        StatusCode,
        Version,
    };

    #[test]
    fn test_parse_request_header() {
        let mut s = Vec::new();
        // TODO: There's a better way to do this, right?
        s.extend(
            &b"POST http://foo.example.com/bar?qux=19&qux=xyz HTTP/1.1\r\n"[..]
        );
        s.extend(&b"Host: foo.example.com\r\n"[..]);
        s.extend(&b"Content-Type: application/json\r\n"[..]);
        s.extend(&b"\r\n"[..]);

        let h = parse_request_header(&s[..]).unwrap();
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
    fn test_write_response_header() {
        let mut s = Vec::new();
        let h = ResponseHeader {
            status_code: StatusCode::from_u16(404).unwrap(),
            version: Version::HTTP_11,
            fields: HeaderMap::new(),
        };
        write_response_header(&h, &mut s).unwrap();
        assert_eq!(s, b"HTTP/1.1 404 Not Found");
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
