use super::*;
use std::{
    collections::BinaryHeap,
    fs::{read, File},
    io::{self, stdin, BufRead, BufReader, Read},
};
//  Check the mod.rs file for custom macros, dependencies and constants
pub fn binary_parser(binary_file: String) -> io::Result<()> {
    // Open the selected file
    let file = File::open(binary_file).expect("failed to open file");

    // Creating a reader with the capacity of the larges possible package.
    let mut reader = BufReader::with_capacity(LARGEST_AVL_SIZE, file);

    // Creating a buffer to contain the AVL Packages
    let mut buffer = [0u8; LARGEST_AVL_SIZE];

    // Reads the bytes into the buffer
    loop {
        let bytes_read = reader.read(&mut buffer).expect("no bytes to read");
        // Breaks the loop when there is no more data to read
        if bytes_read == 0 {
            break;
        }
        // Prints the new bytes. It will print escape characters, if this is a problem, filter them out.
        for byte in &buffer[..bytes_read] {
            println!("{}", byte);
        }
    }

    Ok(())
}
