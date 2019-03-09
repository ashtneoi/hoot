use hoot::{
    parse_request_header,
};
use std::io::stdin;

pub fn main() {
    let s = stdin();
    let sl = s.lock();
    println!("{:?}", parse_request_header(sl).unwrap());
}
