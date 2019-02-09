#[cfg(test)]
mod test {
    use http::{Method, Uri, Version};
    use regex::Regex;

    #[test]
    fn test_parse_header_field() {
        // doesn't support obs-fold e.g. within message/http yet
        // (see rfc7230 section 3.2.4)
        let s = r#"Content-Type: application/json; charset="utf-8""#;
        let r = Regex::new(concat!(
            // token ":" OWS *field-content OWS
            r"^([!#$%&'*+.^_`|~0-9A-Za-z-]+):",
            r"[\t ]*([!-~]([\t -~]*[!-~])?)[\t ]*$",
        )).unwrap();
        let cap = r.captures(s).unwrap();
        assert_eq!(&cap[1], "Content-Type");
        assert_eq!(&cap[2], r#"application/json; charset="utf-8""#);
    }
}
