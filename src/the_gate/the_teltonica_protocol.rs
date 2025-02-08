use super::*;

// The size of a packet can't be less than 45 bytes, and not more than 1280 bytes
// This is the size limitations.

// Look at the mod file, for constansts, custom macros and crates.

// Imagine the AVL packages are letters sent by a firend.
// Then the:
// Preamble        - The beginning of each letter
// Data_Length     - How long each letter will be
// Codec_ID        - Which codec is used, or if it is easier, how the letter is written.
//                   Starting from the codec id and ending at number of data 2.
// Number_Of_Data1 - Is the amount of AVL data that are (should) be in the package
// AVL_Data -        Is the actual content of the letter, the information we want to know
// Number_Of_Data2 - Is the amount of AVL data that are (should) be in the package
//                   This and the first one should match, they are the count of
//                   records this 'letter' should contain.
// CRC16           - Check that no mistakes has been intruduced to the letter, while it
//                   was being sent, spelling mistakes if you will, like the word
//                   'friend' being mispelt as 'firend' in the beginning of this letter.

// AVL Data record
pub struct AVLData {
    pub timestamp: u64,  // 8 bytes
    pub priority: u8,    // 1 byte
    pub gps: GPSElement, // 15 bytes
    pub io: IOElement,   // Variable size
}

// Core AVL packet (Codec 8, 8E, 16)
pub struct AVLPacket {
    pub preamble: u32,       // Always 0x00000000 (4 bytes)
    pub data_length: u32,    // 4 bytes
    pub codec_id: u8,        // 1 byte (0x08 for Codec8, 0x8E for Codec8E, 0x10 for Codec16)
    pub number_of_data1: u8, // 1 byte
    pub avl_data: Vec<AVLData>,
    pub number_of_data2: u8, // 1 byte (must match number_of_data1)
    pub crc16: u32,          // 4 bytes
}

// GPS Element (15 bytes total)
pub struct GPSElement {
    pub longitude: i32, // 4 bytes
    pub latitude: i32,  // 4 bytes
    pub altitude: i16,  // 2 bytes
    pub angle: i16,     // 2 bytes
    pub satellites: u8, // 1 byte
    pub speed: i16,     // 2 bytes
}

// IO Elements for different codecs
pub enum IOElement {
    Codec8(IOElement8),
    Codec8Extended(IOElement8Extended),
    Codec16(IOElement16),
}

// Codec 8 IO Element
pub struct IOElement8 {
    pub event_io_id: u8,                // 1 byte
    pub n_total_io: u8,                 // 1 byte
    pub n1_of_one_byte: u8,             // 1 byte
    pub one_byte_ios: Vec<(u8, u8)>,    // (id, value)
    pub n2_of_two_bytes: u8,            // 1 byte
    pub two_byte_ios: Vec<(u8, u16)>,   // (id, value)
    pub n4_of_four_bytes: u8,           // 1 byte
    pub four_byte_ios: Vec<(u8, u32)>,  // (id, value)
    pub n8_of_eight_bytes: u8,          // 1 byte
    pub eight_byte_ios: Vec<(u8, u64)>, // (id, value)
}

// Codec 8 Extended IO Element
pub struct IOElement8Extended {
    // (id, value) are tuples
    pub event_io_id: u16,                       // 2 bytes
    pub n_total_io: u16,                        // 2 bytes
    pub n1_of_one_byte: u16,                    // 2 bytes
    pub one_byte_ios: Vec<(u16, u8)>,           // (id, value)
    pub n2_of_two_bytes: u16,                   // 2 bytes
    pub two_byte_ios: Vec<(u16, u16)>,          // (id, value)
    pub n4_of_four_bytes: u16,                  // 2 bytes
    pub four_byte_ios: Vec<(u16, u32)>,         // (id, value)
    pub n8_of_eight_bytes: u16,                 // 2 bytes
    pub eight_byte_ios: Vec<(u16, u64)>,        // (id, value)
    pub nx_of_var_bytes: u16,                   // 2 bytes
    pub var_byte_ios: Vec<(u16, u16, Vec<u8>)>, // (id, length, value)
}

// Codec 16 IO Element
pub struct IOElement16 {
    // (id, value) are tuples
    pub event_io_id: u16,                // 2 bytes
    pub generation_type: u8,             // 1 byte
    pub n_total_io: u8,                  // 1 byte
    pub n1_of_one_byte: u8,              // 1 byte
    pub one_byte_ios: Vec<(u16, u8)>,    // (id, value)
    pub n2_of_two_bytes: u8,             // 1 byte
    pub two_byte_ios: Vec<(u16, u16)>,   // (id, value)
    pub n4_of_four_bytes: u8,            // 1 byte
    pub four_byte_ios: Vec<(u16, u32)>,  // (id, value)
    pub n8_of_eight_bytes: u8,           // 1 byte
    pub eight_byte_ios: Vec<(u16, u64)>, // (id, value)
}

// Codec 12 Command Packet
pub struct Codec12CommandPacket {
    pub preamble: u32,     // Always 0x00000000 (4 bytes)
    pub data_length: u32,  // 4 bytes
    pub codec_id: u8,      // 1 byte (0x0C for Codec12)
    pub command_qty1: u8,  // 1 byte
    pub command_type: u8,  // 1 byte (0x05 for command)
    pub command_size: u32, // 4 bytes
    pub command: Vec<u8>,  // Variable size - command in HEX
    pub command_qty2: u8,  // 1 byte (should match command_qty1)
    pub crc16: u32,        // 4 bytes
}

// Codec 12 Response Packet
pub struct Codec12ResponsePacket {
    pub preamble: u32,      // Always 0x00000000 (4 bytes)
    pub data_length: u32,   // 4 bytes
    pub codec_id: u8,       // 1 byte (0x0C for Codec12)
    pub response_qty1: u8,  // 1 byte
    pub response_type: u8,  // 1 byte (0x06 for response)
    pub response_size: u32, // 4 bytes
    pub response: Vec<u8>,  // Variable size - response in HEX
    pub response_qty2: u8,  // 1 byte (should match response_qty1)
    pub crc16: u32,         // 4 bytes
}
