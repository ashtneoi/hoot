use hoot::{
    parse_header,
};
use std::io::stdin;

pub fn main() {
    let s = stdin();
    let sl = s.lock();
    println!("{:?}", parse_header(sl).unwrap());
}
