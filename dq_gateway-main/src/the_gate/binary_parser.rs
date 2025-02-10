//#############################################################################################
//#                                 IMPORTANT INFORMATION                                     #
//#############################################################################################
//#   The codebase is at the moment synchronus. This should be amended when we have a working #
//#   prototype. at the moment, if i am not being too doom and gloom,                         #
//#   somewhere around 70%+ of the time used by this approach would likely                    #
//#   be on just waiting for things.                                                          #
//#############################################################################################

use super::*;
// The size of a packet can't be less than 45 bytes, and not more than 1280 bytes
// This is the size limitations. We should make sure this information should be used.

// Look at the mod file for constansts, custom macros and crates.

// Imagine you're receiving a stream of bytes, like reading a book page by page.
// This parser helps us understand what each page means and how to turn those bytes
// into meaningful information about where vehicles are and what they're doing.

// The Parser struct is like a librarian who knows how to read these special books
// buffer: Where we store the bytes we've received but haven't processed yet
// position: Which byte we're currently looking at (like keeping a finger on the line we're reading)

pub struct Parser {
    buffer: Vec<u8>,
    position: usize,
}

impl Parser {
    // We start with a fresh parser, the opening of a new book if you will.
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(LARGEST_AVL_SIZE),
            position: 0,
        }
    }

    // We then starat reading the pages from the book as they come in.
    // stream: The source of our data (like someone handing us pages)
    // Returns: A complete packet if we have enough data, or None if we need more
    pub fn parse_stream(&mut self, stream: &mut TcpStream) -> io::Result<Option<AVLPacket>> {
        let mut temp_buffer = [0u8; 4096];
        let bytes_read = stream.read(&mut temp_buffer)?;

        // If we got no new data, the book is finished
        if bytes_read == 0 {
            return Ok(None);
        }

        // Add the new pages to our book
        self.buffer.extend_from_slice(&temp_buffer[..bytes_read]);

        // We then try to make sense of what we have read so far
        match self.try_parse_packet() {
            Ok(Some(packet)) => {
                // We have successfully read a complete section of the book, remove those pages
                self.buffer.drain(..self.position);
                self.position = 0;
                Ok(Some(packet))
            }
            // We need more pages to complete the section, I mean, it doesn't even end on a cliffhanger
            Ok(None) => Ok(None),
            Err(e) => {
                // Something went wrong, the writer is clearly drunk, start fresh.
                self.buffer.clear();
                self.position = 0;
                Err(e)
            }
        }
    }

    // This would be like reading the extended footnotes in a fancy academic book
    // The extendet protocol contains extra details that normal one does't have
    fn parse_codec8_extended_io(&mut self) -> io::Result<IOElement> {
        // Checks if we have enough bytes for the header
        if self.buffer.len() < self.position + 4 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "The parsing buffer is too short for extended IO header",
            ));
        }
        // Read the event ID (like a chapter number in our book) and total IO count (words)
        let event_io_id =
            u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;

        let n_total_io =
            u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;

        if self.buffer.len() < self.position + 2 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for one byte elements count",
            ));
        }

        let n1_of_one_byte =
            u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;

        if self.buffer.len() < self.position + (n1_of_one_byte as usize * 3) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for one-byte IO elements",
            ));
        }

        let mut one_byte_ios = Vec::with_capacity(n1_of_one_byte as usize);
        for _ in 0..n1_of_one_byte {
            let id =
                u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
            self.position += 2;
            let value = self.buffer[self.position];
            self.position += 1;
            one_byte_ios.push((id, value));
        }

        if self.buffer.len() < self.position + 2 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for two byte elements count",
            ));
        }

        let n2_of_two_bytes =
            u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;

        if self.buffer.len() < self.position + (n2_of_two_bytes as usize * 4) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for two-byte IO elements",
            ));
        }

        let mut two_byte_ios = Vec::with_capacity(n2_of_two_bytes as usize);
        for _ in 0..n2_of_two_bytes {
            let id =
                u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
            self.position += 2;
            let value =
                u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
            self.position += 2;
            two_byte_ios.push((id, value));
        }

        if self.buffer.len() < self.position + 2 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for four byte elements count",
            ));
        }

        let n4_of_four_bytes =
            u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;

        if self.buffer.len() < self.position + (n4_of_four_bytes as usize * 6) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for four-byte IO elements",
            ));
        }

        let mut four_byte_ios = Vec::with_capacity(n4_of_four_bytes as usize);
        for _ in 0..n4_of_four_bytes {
            let id =
                u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
            self.position += 2;
            let value = u32::from_be_bytes([
                self.buffer[self.position],
                self.buffer[self.position + 1],
                self.buffer[self.position + 2],
                self.buffer[self.position + 3],
            ]);
            self.position += 4;
            four_byte_ios.push((id, value));
        }

        if self.buffer.len() < self.position + 2 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for eight byte elements count",
            ));
        }

        let n8_of_eight_bytes =
            u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;

        if self.buffer.len() < self.position + (n8_of_eight_bytes as usize * 10) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for eight-byte IO elements",
            ));
        }

        let mut eight_byte_ios = Vec::with_capacity(n8_of_eight_bytes as usize);
        for _ in 0..n8_of_eight_bytes {
            let id =
                u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
            self.position += 2;
            let value = u64::from_be_bytes([
                self.buffer[self.position],
                self.buffer[self.position + 1],
                self.buffer[self.position + 2],
                self.buffer[self.position + 3],
                self.buffer[self.position + 4],
                self.buffer[self.position + 5],
                self.buffer[self.position + 6],
                self.buffer[self.position + 7],
            ]);
            self.position += 8;
            eight_byte_ios.push((id, value));
        }

        if self.buffer.len() < self.position + 2 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for variable length elements count",
            ));
        }

        let nx_of_var_bytes =
            u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;

        let mut var_byte_ios = Vec::with_capacity(nx_of_var_bytes as usize);
        for _ in 0..nx_of_var_bytes {
            if self.buffer.len() < self.position + 4 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Buffer too short for variable length IO element header",
                ));
            }

            let id =
                u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
            self.position += 2;

            let length =
                u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
            self.position += 2;

            // Validate we have enough bytes for the variable length value
            if self.buffer.len() < self.position + length as usize {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Buffer too short for variable length IO element value",
                ));
            }

            let mut value = Vec::with_capacity(length as usize);
            for _ in 0..length {
                value.push(self.buffer[self.position]);
                self.position += 1;
            }

            var_byte_ios.push((id, length, value));
        }

        Ok(IOElement::Codec8Extended(IOElement8Extended {
            event_io_id,
            n_total_io,
            n1_of_one_byte,
            one_byte_ios,
            n2_of_two_bytes,
            two_byte_ios,
            n4_of_four_bytes,
            four_byte_ios,
            n8_of_eight_bytes,
            eight_byte_ios,
            nx_of_var_bytes,
            var_byte_ios,
        }))
    }

    // This is where we try to read one complete section of our book
    // Returns: A complete packet if we have enough data, or None if we need more
    fn try_parse_packet(&mut self) -> io::Result<Option<AVLPacket>> {
        // First, make sure the section we are reading have at least the minimum amount of pages
        if self.buffer.len() < SMALLEST_AVL_SIZE {
            return Ok(None);
        }
        let codec_id = self.buffer[self.position];
        self.position += 1;

        let number_of_data1 = self.buffer[self.position];
        self.position += 1;

        let mut avl_data = Vec::with_capacity(number_of_data1 as usize);
        for _ in 0..number_of_data1 {
            let data = self.parse_avl_data(codec_id)?; // Pass the codec_id here
            avl_data.push(data);
        }

        // We then check that the book starts with the right sequence (like "Chapter 1")
        let preamble = u32::from_be_bytes([
            self.buffer[0],
            self.buffer[1],
            self.buffer[2],
            self.buffer[3],
        ]);
        if preamble != 0x00000000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid preamble: expected 0x00000000, got {:#010x}",
                    preamble
                ),
            ));
        }

        self.position = 4;

        if self.buffer.len() < self.position + 4 {
            return Ok(None);
        }

        let data_length = u32::from_be_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
        ]);

        self.position += 4;

        let total_length = 8 + data_length as usize + 4; // preamble + length + data + crc
        if self.buffer.len() < total_length {
            return Ok(None);
        }

        let codec_id = self.buffer[self.position];
        self.position += 1;

        let number_of_data1 = self.buffer[self.position];
        self.position += 1;

        let mut avl_data = Vec::with_capacity(number_of_data1 as usize);
        for _ in 0..number_of_data1 {
            let data = self.parse_avl_data(codec_id)?;
            avl_data.push(data);
        }

        let number_of_data2 = self.buffer[self.position];
        self.position += 1;

        if number_of_data1 != number_of_data2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Data record count mismatch",
            ));
        }

        let crc = u32::from_be_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
        ]);
        self.position += 4;

        let calculated_crc = self.calculate_crc(&self.buffer[4..self.position - 4]);
        if crc != calculated_crc {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CRC validation failed",
            ));
        }

        Ok(Some(AVLPacket {
            preamble,
            data_length,
            codec_id,
            number_of_data1,
            avl_data,
            number_of_data2,
            crc16: crc,
        }))
    }

    // Each book contains the the story of our vehicle(The AVL-data), about the advetures it has been on.
    // This means that each entry contains information about where a vehicle was and what it was doing.
    // This is what we are reading hear.
    fn parse_avl_data(&mut self, codec_id: u8) -> io::Result<AVLData> {
        // First 8 bytes tell us when this happened
        if self.buffer.len() < self.position + 8 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for timestamp",
            ));
        }
        let timestamp = u64::from_be_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
            self.buffer[self.position + 4],
            self.buffer[self.position + 5],
            self.buffer[self.position + 6],
            self.buffer[self.position + 7],
        ]);
        self.position += 8;

        if timestamp == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid timestamp",
            ));
        }

        if self.buffer.len() < self.position + 1 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for priority",
            ));
        }
        let priority = self.buffer[self.position];
        self.position += 1;

        if self.buffer.len() < self.position + 15 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for GPS data",
            ));
        }

        let longitude = i32::from_be_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
        ]);
        self.position += 4;

        let latitude = i32::from_be_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
        ]);
        self.position += 4;

        if longitude < -180_00000 || longitude > 180_00000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Longitude out of range",
            ));
        }
        if latitude < -90_00000 || latitude > 90_00000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Latitude out of range",
            ));
        }

        let altitude =
            i16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;

        let angle =
            i16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;

        let satellites = self.buffer[self.position];
        self.position += 1;

        let speed =
            i16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;
        let io = match codec_id {
            0x08 => self.parse_codec8_io()?,
            0x8E => self.parse_codec8_extended_io()?,
            unknown_codec => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unsupported codec: {:#04x}", unknown_codec),
                ))
            }
        };

        Ok(AVLData {
            timestamp,
            priority,
            gps: GPSElement {
                longitude,
                latitude,
                altitude,
                angle,
                satellites,
                speed,
            },
            io,
        })
    }

    fn parse_codec8_io(&mut self) -> io::Result<IOElement> {
        if self.buffer.len() < self.position + 3 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for IO header",
            ));
        }

        let event_io_id = self.buffer[self.position];
        self.position += 1;

        let n_total_io = self.buffer[self.position];
        self.position += 1;

        let n1_of_one_byte = self.buffer[self.position];
        self.position += 1;

        if self.buffer.len() < self.position + (n1_of_one_byte as usize * 2) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for one-byte IO elements",
            ));
        }

        let mut one_byte_ios = Vec::with_capacity(n1_of_one_byte as usize);
        for _ in 0..n1_of_one_byte {
            let id = self.buffer[self.position];
            self.position += 1;
            let value = self.buffer[self.position];
            self.position += 1;
            one_byte_ios.push((id, value));
        }

        let n2_of_two_bytes = self.buffer[self.position];
        self.position += 1;

        if self.buffer.len() < self.position + (n2_of_two_bytes as usize * 3) {
            // id(1) + value(2)
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for two-byte IO elements",
            ));
        }

        let mut two_byte_ios = Vec::with_capacity(n2_of_two_bytes as usize);
        for _ in 0..n2_of_two_bytes {
            let id = self.buffer[self.position];
            self.position += 1;
            let value =
                u16::from_be_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
            self.position += 2;
            two_byte_ios.push((id, value));
        }

        let n4_of_four_bytes = self.buffer[self.position];
        self.position += 1;

        if self.buffer.len() < self.position + (n4_of_four_bytes as usize * 5) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for four-byte IO elements",
            ));
        }

        let mut four_byte_ios = Vec::with_capacity(n4_of_four_bytes as usize);
        for _ in 0..n4_of_four_bytes {
            let id = self.buffer[self.position];
            self.position += 1;
            let value = u32::from_be_bytes([
                self.buffer[self.position],
                self.buffer[self.position + 1],
                self.buffer[self.position + 2],
                self.buffer[self.position + 3],
            ]);
            self.position += 4;
            four_byte_ios.push((id, value));
        }

        let n8_of_eight_bytes = self.buffer[self.position];
        self.position += 1;

        if self.buffer.len() < self.position + (n8_of_eight_bytes as usize * 9) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too short for eight-byte IO elements",
            ));
        }

        let mut eight_byte_ios = Vec::with_capacity(n8_of_eight_bytes as usize);
        for _ in 0..n8_of_eight_bytes {
            let id = self.buffer[self.position];
            self.position += 1;
            let value = u64::from_be_bytes([
                self.buffer[self.position],
                self.buffer[self.position + 1],
                self.buffer[self.position + 2],
                self.buffer[self.position + 3],
                self.buffer[self.position + 4],
                self.buffer[self.position + 5],
                self.buffer[self.position + 6],
                self.buffer[self.position + 7],
            ]);
            self.position += 8;
            eight_byte_ios.push((id, value));
        }

        Ok(IOElement::Codec8(IOElement8 {
            event_io_id,
            n_total_io,
            n1_of_one_byte,
            one_byte_ios,
            n2_of_two_bytes,
            two_byte_ios,
            n4_of_four_bytes,
            four_byte_ios,
            n8_of_eight_bytes,
            eight_byte_ios,
        }))
    }

    // This calculates a special number that helps us verify nothing got corrupted
    // Like checking if any pages have grammatical mistakes, spelling errors
    // or got coffee stains on them during delivery. Maybe it's dog ate his homework?
    fn calculate_crc(&self, data: &[u8]) -> u32 {
        let mut crc: u16 = 0xFFFF;
        for &byte in data {
            crc ^= (byte as u16) & 0xFF;
            for _ in 0..8 {
                if (crc & 0x0001) != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc as u32
    }
}
