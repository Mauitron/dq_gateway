//#############################################################################################
//#                                 IMPORTANT INFORMATION                                     #
//#############################################################################################
//#   The codebase is at the moment synchronus. This should be amended when we have a working #
//#   prototype. at the moment, if i am not being too doom and gloom,                         #
//#   somewhere around 70%+ of the time used by this approach would likely                    #
//#   be on just waiting for things.                                                          #
//#############################################################################################

mod the_gate;

// use the_gate::binary_parser::binary_parser;
fn main() {
    let mut buf = String::new();

    // binary_parser("/home/charon/Projects/deviaq_gateway/e".to_string()).expect("thoeu");

    let mut a = std::io::Stdin::read_line(&std::io::stdin(), &mut buf);
    println!("{:?}", a);
}
