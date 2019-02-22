use http::{
    Method,
    Uri,
    Version,
};
use http::header::{
    HeaderName,
    HeaderValue,
    InvalidHeaderName,
    InvalidHeaderValue,
};
use http::method::InvalidMethod;
use http::uri::InvalidUriBytes;
use lazy_static::lazy_static;
use regex::bytes::Regex;

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
    // doesn't support obs-fold e.g. within message/http yet
    // (see rfc7230 section 3.2.4)
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
        parse_request_line,
        parse_header_field,
    };
    use http::header::HeaderValue;
    use http::{
        Method,
        Version,
    };

    #[test]
    fn test_parse_request_line() {
        let s = b"OPTIONS * HTTP/1.1";
        let (m, u, v) = parse_request_line(s).unwrap();
        assert_eq!(m, Method::OPTIONS);
        assert_eq!(u.path(), "*");
        assert_eq!(v, Version::HTTP_11);
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
