#[cfg(test)]
mod test {
    use http::{Method, Uri, Version};
    use http::header::{HeaderName, HeaderValue};
    use regex::bytes::Regex;

    #[test]
    fn test_parse_header_field() {
        // doesn't support obs-fold e.g. within message/http yet
        // (see rfc7230 section 3.2.4)
        let s = b"Content-Type: application/json; charset=\"\xAA\xBB\xCC\"";
        let r = Regex::new(concat!(
            // token ":" OWS *field-content OWS
            r"(?-u)^([!#$%&'*+.^_`|~0-9A-Za-z-]+):",
            r"[\t ]*([!-~\x80-\xFF]([\t -~\x80-\xFF]*[!-~\x80-\xFF])?)[\t ]*$",
        )).unwrap();
        let cap = r.captures(s).unwrap();
        assert_eq!(
            HeaderName::from_bytes(&cap[1]).unwrap(),
            http::header::CONTENT_TYPE,
        );
        assert_eq!(
            HeaderValue::from_bytes(&cap[2]).unwrap(),
            HeaderValue::from_bytes(
                &b"application/json; charset=\"\xAA\xBB\xCC\""[..]
            ).unwrap(),
        );
    }
}
