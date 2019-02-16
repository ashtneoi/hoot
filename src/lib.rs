use http::header::{
    HeaderName,
    HeaderValue,
    InvalidHeaderName,
    InvalidHeaderValue,
};
use lazy_static::lazy_static;
use regex::bytes::Regex;

#[derive(Debug)]
pub enum InvalidHeaderField {
    Syntax,
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
    lazy_static! {
        static ref R: Regex = Regex::new(concat!(
            // token ":" OWS *field-content OWS
            r"(?-u)^([!#$%&'*+.^_`|~0-9A-Za-z-]+):",
            r"[\t ]*([!-~\x80-\xFF]([\t !-~\x80-\xFF]*[!-~\x80-\xFF])?)[\t ]*$",
        )).unwrap();
    }
    let cap = R.captures(s).ok_or(InvalidHeaderField::Syntax)?;
    Ok((
        HeaderName::from_bytes(&cap[1])?,
        HeaderValue::from_bytes(&cap[2])?,
    ))
}

#[cfg(test)]
mod test {
    use crate::parse_header_field;
    use http::{Method, Uri, Version};
    use http::header::HeaderValue;

    #[test]
    fn test_parse_header_field() {
        // doesn't support obs-fold e.g. within message/http yet
        // (see rfc7230 section 3.2.4)
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
