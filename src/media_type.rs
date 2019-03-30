use crate::{QUOTED_STRING_1G, TOKEN};
use lazy_static::lazy_static;
use regex::bytes::Regex;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaType {
    pub type_: String,
    pub subtype: String,
    // TODO: A hash map is abysmally ineffecient for this.
    pub parameters: HashMap<String, Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidMediaType;

pub fn parse_media_type(mut s: &[u8]) -> Result<MediaType, InvalidMediaType> {
    lazy_static! {
        // type "/" subtype *( OWS ";" OWS parameter )

        static ref R1: Regex = Regex::new(&(String::new()
            + r"(?-u)^(" + TOKEN + r")/(" + TOKEN + r")"
        )).unwrap();

        static ref R2: Regex = Regex::new(&(String::new()
            + r"(?-u)^[\t ]*;[\t ]*(" + TOKEN + r")=("
            + TOKEN + r"|" + QUOTED_STRING_1G + r")"
        )).unwrap();
    }

    let cap = R1.captures(s).ok_or(InvalidMediaType)?;
    let mut m = MediaType {
        type_: String::from_utf8(cap[1].to_vec()).unwrap(),
        subtype: String::from_utf8(cap[2].to_vec()).unwrap(),
        parameters: HashMap::new(),
    };
    s = &s[cap.get(0).unwrap().end()..];

    while s.len() > 0 {
        let cap = R2.captures(s).ok_or(InvalidMediaType)?;
        let quoted_value = &cap[2];
        let mut value;
        if quoted_value[0] == b'"' {
            assert_eq!(quoted_value[quoted_value.len()-1], b'"');
            value = Vec::new();
            for &c in &quoted_value[1..=quoted_value.len()-2] {
                if c != b'\\' {
                    value.push(c);
                }
            }
        } else {
            value = cap[2].to_vec();
        }
        m.parameters.insert(
            String::from_utf8(cap[1].to_vec()).unwrap(),
            value,
        );
        s = &s[cap.get(0).unwrap().end()..];
    }

    Ok(m)
}

#[cfg(test)]
mod test {
    use crate::media_type::{
        MediaType,
        parse_media_type,
    };
    use std::collections::HashMap;

    #[test]
    fn test_parse_media_type() {
        assert_eq!(
            parse_media_type(
                b"application/json"
            ).unwrap(),
            MediaType {
                type_: "application".to_string(),
                subtype: "json".to_string(),
                parameters: HashMap::new(),
            }
        );

        let mut p = HashMap::new();
        p.insert("charset".to_string(), b"utf-8".to_vec());

        assert_eq!(
            parse_media_type(
                b"text/plain; charset=utf-8"
            ).unwrap(),
            MediaType {
                type_: "text".to_string(),
                subtype: "plain".to_string(),
                parameters: p.clone(),
            },
        );
        assert_eq!(
            parse_media_type(
                br#"text/plain ;charset="utf-8""#
            ).unwrap(),
            MediaType {
                type_: "text".to_string(),
                subtype: "plain".to_string(),
                parameters: p.clone(),
            },
        );
    }
}
