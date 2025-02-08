mod the_gate;
use std::io::{stdout, Read, Stdin};

use the_gate::binary_parser::binary_parser;
fn main() {
    let mut buf = String::new();

    binary_parser("/home/charon/Projects/deviaq_gateway/e".to_string()).expect("thoeu");

    let mut a = Stdin::read_line(&std::io::stdin(), &mut buf);
    println!("{:?}", a);
}
