//#############################################################################################
//#                                 IMPORTANT INFORMATION                                     #
//#############################################################################################
//#   The codebase is at the moment synchronus. This should be amended when we have a working #
//#   prototype. at the moment, if i am not being too doom and gloom,                         #
//#   somewhere around 70%+ of the time used by this approach would likely                    #
//#   be on just waiting for things.                                                          #
//#############################################################################################

// These are integration tests
// Think of these tests as quality control inspectors, sent by coorperate, to sure
// everything works smoothly. If we step out of line or write faulty code, they will
// chastise us, and then report back where in our code we are failing.
// Super nice people, really...

#[cfg(test)]
pub mod integration_tests {

    // Import all our other modules
    use crate::the_gate::AVLData;
    use crate::the_gate::AVLPacket;
    use crate::the_gate::Connection;
    use crate::the_gate::GPSElement;
    use crate::the_gate::IOElement;
    use crate::the_gate::IOElement16;
    use crate::the_gate::IOElement8;
    use crate::the_gate::IOElement8Extended;
    use crate::the_gate::Parser;
    use crate::the_gate::ProcessingPipeline;
    use crate::the_gate::ProtocolAction;
    use crate::the_gate::ProtocolEvent;
    use crate::the_gate::ProtocolState;
    use crate::the_gate::StateMachine;
    use crate::the_gate::LARGEST_AVL_SIZE;
    use crate::the_gate::MAX_AVL_PACKET_SIZE_FM6XXX;
    use crate::the_gate::SMALLEST_AVL_SIZE;
    use std::io::{self, Cursor, Read, Write};
    use std::net::SocketAddr;
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    // creates test packets
    // to test if we are actually able to parse and store the protocol
    fn create_mock_avl_packet(data_count: u8) -> AVLPacket {
        let mut avl_data = Vec::with_capacity(data_count as usize);

        for i in 0..data_count {
            // Creates psudo GPS data, where our imaginary vehicle is located
            let gps = GPSElement {
                longitude: 25_00000,
                latitude: 54_00000,
                altitude: 100,
                angle: 90,
                satellites: 8,
                speed: 50,
            };

            // Create IO data, what our pretend vehicle is doing
            let io = IOElement::Codec8(IOElement8 {
                event_io_id: 1,
                n_total_io: 1,
                n1_of_one_byte: 1,
                one_byte_ios: vec![(1, i)],
                n2_of_two_bytes: 0,
                two_byte_ios: vec![],
                n4_of_four_bytes: 0,
                four_byte_ios: vec![],
                n8_of_eight_bytes: 0,
                eight_byte_ios: vec![],
            });

            // Packages all these lies up and stamps them with a timestamp
            avl_data.push(AVLData {
                timestamp: 1644238347000 + (i as u64 * 1000), // Sequential timestamps, one after the other
                priority: 1,
                gps,
                io,
            });
        }

        // Create the final packet
        AVLPacket {
            preamble: 0x00000000,
            data_length: 0, // is set to 0 because it will be calculated later
            codec_id: 0x08,
            number_of_data1: data_count,
            avl_data,
            number_of_data2: data_count,
            crc16: 0, // Is set to 0 for the same reason as the data_length
        }
    }

    // The PacketSerializer helps us convert our packets into raw bytes
    // Translating our message into a uniform language devices understand
    struct PacketSerializer {
        buffer: Vec<u8>,
    }

    impl PacketSerializer {
        fn new() -> Self {
            Self {
                buffer: Vec::with_capacity(LARGEST_AVL_SIZE),
            }
        }

        fn serialize_packet(&mut self, packet: &AVLPacket) -> io::Result<Vec<u8>> {
            self.buffer.clear();

            // Write preamble
            self.write_u32(packet.preamble)?;

            // Calculate and write data length
            let data_start_pos = self.buffer.len() + 4; // Position after data_length
            self.write_u32(0)?; // Placeholder for data_length

            // Write codec ID and number of data
            self.write_u8(packet.codec_id)?;
            self.write_u8(packet.number_of_data1)?;

            // Write AVL data records
            for data in &packet.avl_data {
                self.serialize_avl_data(data)?;
            }

            self.write_u8(packet.number_of_data2)?;

            // Calculate actual data length
            let data_length = (self.buffer.len() - data_start_pos) as u32;

            // Go back and write the actual data length
            let data_length_bytes = data_length.to_be_bytes();
            self.buffer[data_start_pos - 4..data_start_pos].copy_from_slice(&data_length_bytes);

            // Calculate and write CRC16
            let crc = self.calculate_crc(&self.buffer[4..]);
            self.write_u32(crc)?;

            Ok(self.buffer.clone())
        }

        fn serialize_avl_data(&mut self, data: &AVLData) -> io::Result<()> {
            // Write timestamp (8 bytes)
            self.write_u64(data.timestamp)?;

            // Write priority
            self.write_u8(data.priority)?;

            // Write GPS data
            self.write_i32(data.gps.longitude)?;
            self.write_i32(data.gps.latitude)?;
            self.write_i16(data.gps.altitude)?;
            self.write_i16(data.gps.angle)?;
            self.write_u8(data.gps.satellites)?;
            self.write_i16(data.gps.speed)?;

            // Write IO data based on type
            match &data.io {
                IOElement::Codec8(io) => self.serialize_codec8_io(io)?,
                IOElement::Codec8Extended(io) => self.serialize_codec8_extended_io(io)?,
                IOElement::Codec16(io) => self.serialize_codec16_io(io)?,
            }

            Ok(())
        }

        fn serialize_codec8_io(&mut self, io: &IOElement8) -> io::Result<()> {
            self.write_u8(io.event_io_id)?;
            self.write_u8(io.n_total_io)?;

            // 1-byte elements
            self.write_u8(io.n1_of_one_byte)?;
            for (id, value) in &io.one_byte_ios {
                self.write_u8(*id)?;
                self.write_u8(*value)?;
            }

            // 2-byte elements
            self.write_u8(io.n2_of_two_bytes)?;
            for (id, value) in &io.two_byte_ios {
                self.write_u8(*id)?;
                self.write_u16(*value)?;
            }

            // 4-byte elements
            self.write_u8(io.n4_of_four_bytes)?;
            for (id, value) in &io.four_byte_ios {
                self.write_u8(*id)?;
                self.write_u32(*value)?;
            }

            // 8-byte elements
            self.write_u8(io.n8_of_eight_bytes)?;
            for (id, value) in &io.eight_byte_ios {
                self.write_u8(*id)?;
                self.write_u64(*value)?;
            }

            Ok(())
        }

        fn serialize_codec8_extended_io(&mut self, io: &IOElement8Extended) -> io::Result<()> {
            self.write_u16(io.event_io_id)?;
            self.write_u16(io.n_total_io)?;

            // 1-byte elements
            self.write_u16(io.n1_of_one_byte)?;
            for (id, value) in &io.one_byte_ios {
                self.write_u16(*id)?;
                self.write_u8(*value)?;
            }

            // 2-byte elements
            self.write_u16(io.n2_of_two_bytes)?;
            for (id, value) in &io.two_byte_ios {
                self.write_u16(*id)?;
                self.write_u16(*value)?;
            }

            // 4-byte elements
            self.write_u16(io.n4_of_four_bytes)?;
            for (id, value) in &io.four_byte_ios {
                self.write_u16(*id)?;
                self.write_u32(*value)?;
            }

            // 8-byte elements
            self.write_u16(io.n8_of_eight_bytes)?;
            for (id, value) in &io.eight_byte_ios {
                self.write_u16(*id)?;
                self.write_u64(*value)?;
            }

            // Variable length elements
            self.write_u16(io.nx_of_var_bytes)?;
            for (id, length, value) in &io.var_byte_ios {
                self.write_u16(*id)?;
                self.write_u16(*length)?;
                self.buffer.extend_from_slice(value);
            }

            Ok(())
        }

        fn serialize_codec16_io(&mut self, io: &IOElement16) -> io::Result<()> {
            self.write_u16(io.event_io_id)?;
            self.write_u8(io.generation_type)?;
            self.write_u8(io.n_total_io)?;

            // 1-byte elements
            self.write_u8(io.n1_of_one_byte)?;
            for (id, value) in &io.one_byte_ios {
                self.write_u16(*id)?;
                self.write_u8(*value)?;
            }

            // 2-byte elements
            self.write_u8(io.n2_of_two_bytes)?;
            for (id, value) in &io.two_byte_ios {
                self.write_u16(*id)?;
                self.write_u16(*value)?;
            }

            // 4-byte elements
            self.write_u8(io.n4_of_four_bytes)?;
            for (id, value) in &io.four_byte_ios {
                self.write_u16(*id)?;
                self.write_u32(*value)?;
            }

            // 8-byte elements
            self.write_u8(io.n8_of_eight_bytes)?;
            for (id, value) in &io.eight_byte_ios {
                self.write_u16(*id)?;
                self.write_u64(*value)?;
            }

            Ok(())
        }

        // Helper methods for writing different integer types
        fn write_u8(&mut self, value: u8) -> io::Result<()> {
            self.buffer.push(value);
            Ok(())
        }

        fn write_u16(&mut self, value: u16) -> io::Result<()> {
            self.buffer.extend_from_slice(&value.to_be_bytes());
            Ok(())
        }

        fn write_u32(&mut self, value: u32) -> io::Result<()> {
            self.buffer.extend_from_slice(&value.to_be_bytes());
            Ok(())
        }

        fn write_u64(&mut self, value: u64) -> io::Result<()> {
            self.buffer.extend_from_slice(&value.to_be_bytes());
            Ok(())
        }

        fn write_i16(&mut self, value: i16) -> io::Result<()> {
            self.buffer.extend_from_slice(&value.to_be_bytes());
            Ok(())
        }

        fn write_i32(&mut self, value: i32) -> io::Result<()> {
            self.buffer.extend_from_slice(&value.to_be_bytes());
            Ok(())
        }

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

    // MockDevice pretends to be a real tracking device
    // it should help us test our system without needing actual hardware
    // by lying to us, like telling us we're pretty even though we're clearly a wreck.
    struct MockDevice {
        listener: TcpListener,
    }

    impl MockDevice {
        fn new() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            Self { listener }
        }

        // Get the device's address (we pretend we believe them and get their phone number)
        fn addr(&self) -> SocketAddr {
            self.listener.local_addr().unwrap()
        }

        // Start listening for connections
        // having the fake device wait for us to call
        fn accept_connection(self) -> thread::JoinHandle<()> {
            thread::spawn(move || {
                let (mut socket, _) = self.listener.accept().unwrap();
                let mut buffer = [0u8; 1024];

                // Simple echoing for testing, it sends back what it recieves.
                loop {
                    match socket.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Err(_) = socket.write_all(&buffer[..n]) {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })
        }
    }

    #[test]
    // Test that everything works together in a working scenario
    fn test_full_connection_flow() {
        // Set up the dishonest fake device
        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();
        let device_thread = mock_device.accept_connection();

        // Create our system components
        let mut connection = Connection::new(device_addr);
        let mut state_machine = StateMachine::new(Duration::from_secs(5));
        let mut pipeline = ProcessingPipeline::new(10);
        let mut parser = Parser::new();

        // Try connecting the two
        assert!(connection.connect().is_ok());
        assert!(connection.is_connected());

        // check each state machine transition and so on.
        let result = state_machine.handle_event(ProtocolEvent::Connect);
        assert_eq!(result.state, ProtocolState::Connected);

        let result = state_machine.handle_event(ProtocolEvent::Authenticate(
            "123456789".to_string(),
            "".to_string(),
        ));
        assert_eq!(result.state, ProtocolState::Authenticating);

        let result = state_machine.handle_event(ProtocolEvent::AuthSuccess);
        assert_eq!(result.state, ProtocolState::Ready);

        // Test packet processing
        let test_packet = create_mock_avl_packet(2);
        assert!(pipeline.process_incoming(test_packet.clone(), None).is_ok());

        let (incoming, outgoing) = pipeline.queue_stats();
        assert!(incoming > 0 || outgoing > 0, "Packet should be queued");

        // Clean up
        connection.shutdown().unwrap();
        device_thread.join().unwrap();
    }

    #[test]
    // Test if we are able to convert reciveved packets into bytes and back
    fn test_packet_serialization_and_parsing() {
        // Set up mock device that will echo our serialized packet
        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();

        // Spawn the mock device thread with custom handler
        let device_thread = thread::spawn(move || {
            let (mut socket, _) = mock_device.listener.accept().unwrap();
            let mut buf = [0u8; LARGEST_AVL_SIZE];

            // Read the incoming packet
            let n = socket.read(&mut buf).unwrap();
            // Echo it back, showing us it is listning.
            socket.write_all(&buf[..n]).unwrap();
        });

        // Create client connection
        let mut connection = Connection::new(device_addr);
        assert!(connection.connect().is_ok());

        let mut parser = Parser::new();
        let mut serializer = PacketSerializer::new();

        // Create and serialize test packet
        let original_packet = create_mock_avl_packet(2);
        let serialized_data = serializer.serialize_packet(&original_packet).unwrap();

        // Get the TcpStream from connection
        if let Some(stream) =
            unsafe { &mut *(&mut connection as *mut Connection).cast::<Connection>() }
                .stream
                .as_mut()
        {
            // Send the serialized packet
            stream.write_all(&serialized_data).unwrap();

            // Parse the response using the actual Parser implementation
            let parsed_result = parser.parse_stream(stream);
            assert!(parsed_result.is_ok());

            if let Ok(Some(parsed_packet)) = parsed_result {
                // Verify packet structure
                assert_eq!(parsed_packet.codec_id, original_packet.codec_id);
                assert_eq!(
                    parsed_packet.number_of_data1,
                    original_packet.number_of_data1
                );
                assert_eq!(
                    parsed_packet.number_of_data2,
                    original_packet.number_of_data2
                );
                assert_eq!(parsed_packet.avl_data.len(), original_packet.avl_data.len());

                // Compare each AVL data record in detail
                for (original_data, parsed_data) in original_packet
                    .avl_data
                    .iter()
                    .zip(parsed_packet.avl_data.iter())
                {
                    // Compare basic fields
                    assert_eq!(parsed_data.timestamp, original_data.timestamp);
                    assert_eq!(parsed_data.priority, original_data.priority);

                    // Compare GPS data
                    assert_eq!(parsed_data.gps.longitude, original_data.gps.longitude);
                    assert_eq!(parsed_data.gps.latitude, original_data.gps.latitude);
                    assert_eq!(parsed_data.gps.altitude, original_data.gps.altitude);
                    assert_eq!(parsed_data.gps.angle, original_data.gps.angle);
                    assert_eq!(parsed_data.gps.satellites, original_data.gps.satellites);
                    assert_eq!(parsed_data.gps.speed, original_data.gps.speed);

                    // Compare IO elements
                    match (&original_data.io, &parsed_data.io) {
                        (IOElement::Codec8(original_io), IOElement::Codec8(parsed_io)) => {
                            assert_eq!(parsed_io.event_io_id, original_io.event_io_id);
                            assert_eq!(parsed_io.n_total_io, original_io.n_total_io);
                            assert_eq!(parsed_io.n1_of_one_byte, original_io.n1_of_one_byte);
                            assert_eq!(parsed_io.one_byte_ios, original_io.one_byte_ios);
                            assert_eq!(parsed_io.n2_of_two_bytes, original_io.n2_of_two_bytes);
                            assert_eq!(parsed_io.two_byte_ios, original_io.two_byte_ios);
                            assert_eq!(parsed_io.n4_of_four_bytes, original_io.n4_of_four_bytes);
                            assert_eq!(parsed_io.four_byte_ios, original_io.four_byte_ios);
                            assert_eq!(parsed_io.n8_of_eight_bytes, original_io.n8_of_eight_bytes);
                            assert_eq!(parsed_io.eight_byte_ios, original_io.eight_byte_ios);
                        }
                        _ => panic!("Unexpected IO element type in parsed packet"),
                    }
                }
            }
        }
    }

    #[test]
    fn test_error_handling() {
        let mut state_machine = StateMachine::new(Duration::from_secs(1));
        let mut pipeline = ProcessingPipeline::new(10);

        // Test timeout handling
        let result = state_machine.handle_event(ProtocolEvent::Timeout);
        assert_eq!(result.state, ProtocolState::Error);
        assert!(result
            .actions
            .iter()
            .any(|action| matches!(action, ProtocolAction::DisconnectClient)));

        // Test invalid packet handling
        let result = state_machine.handle_event(ProtocolEvent::InvalidPacket);
        assert_eq!(result.state, ProtocolState::Error);

        // Test connection loss handling
        let result = state_machine.handle_event(ProtocolEvent::ConnectionLost);
        assert_eq!(result.state, ProtocolState::Disconnected);
        assert!(result
            .actions
            .iter()
            .any(|action| matches!(action, ProtocolAction::ResetConnection)));
    }

    #[test]
    fn test_pipeline_processing() {
        let mut pipeline = ProcessingPipeline::new(2);

        // Create test packets with different priorities
        let mut packet1 = create_mock_avl_packet(1);
        packet1.avl_data[0].priority = 1;

        let mut packet2 = create_mock_avl_packet(1);
        packet2.avl_data[0].priority = 8; // High priority

        // Test priority handling
        pipeline.process_incoming(packet1.clone(), None).unwrap();
        pipeline.process_incoming(packet2.clone(), None).unwrap();

        let (incoming, outgoing) = pipeline.queue_stats();
        assert_eq!(incoming + outgoing, 2, "Both packets should be queued");

        // Test batch processing
        let processed = pipeline.flush().unwrap();
        assert_eq!(processed.len(), 2, "All packets should be processed");

        // Verify high priority packet is processed first
        assert_eq!(processed[0].avl_data[0].priority, 8);
        assert_eq!(processed[1].avl_data[0].priority, 1);
    }

    #[test]
    fn test_extended_codec_handling() {
        let mut parser = Parser::new();
        let mut serializer = PacketSerializer::new();

        // Create a packet with extended codec (8E)
        let mut packet = create_mock_avl_packet(1);
        packet.codec_id = 0x8E;
        packet.avl_data[0].io = IOElement::Codec8Extended(IOElement8Extended {
            event_io_id: 0x1234,
            n_total_io: 1,
            n1_of_one_byte: 1,
            one_byte_ios: vec![(0x5678, 42)],
            n2_of_two_bytes: 0,
            two_byte_ios: vec![],
            n4_of_four_bytes: 0,
            four_byte_ios: vec![],
            n8_of_eight_bytes: 0,
            eight_byte_ios: vec![],
            nx_of_var_bytes: 1,
            var_byte_ios: vec![(0x9ABC, 3, vec![1, 2, 3])],
        });

        // Set up mock device
        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();
        let serialized_data = serializer.serialize_packet(&packet).unwrap();

        let device_thread = thread::spawn(move || {
            let (mut socket, _) = mock_device.listener.accept().unwrap();
            socket.write_all(&serialized_data).unwrap();
        });

        let mut connection = Connection::new(device_addr);
        assert!(connection.connect().is_ok());

        if let Some(stream) =
            unsafe { &mut *(&mut connection as *mut Connection).cast::<Connection>() }
                .stream
                .as_mut()
        {
            let parsed_result = parser.parse_stream(stream);

            assert!(parsed_result.is_ok());
            if let Ok(Some(parsed_packet)) = parsed_result {
                assert_eq!(parsed_packet.codec_id, 0x8E);

                // Verify extended codec data
                if let IOElement::Codec8Extended(io) = &parsed_packet.avl_data[0].io {
                    assert_eq!(io.event_io_id, 0x1234);
                    assert_eq!(io.n_total_io, 1);
                    assert_eq!(io.one_byte_ios, vec![(0x5678, 42)]);
                    assert_eq!(io.var_byte_ios, vec![(0x9ABC, 3, vec![1, 2, 3])]);
                } else {
                    panic!("Expected Codec8Extended IO element");
                }
            }
        }
    }
    #[test]
    fn test_codec_edge_cases() {
        let mut parser = Parser::new();
        let mut serializer = PacketSerializer::new();

        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();
        let mut empty_io_packet = create_mock_avl_packet(1);
        empty_io_packet.avl_data[0].io = IOElement::Codec8(IOElement8 {
            event_io_id: 0,
            n_total_io: 0,
            n1_of_one_byte: 0,
            one_byte_ios: vec![],
            n2_of_two_bytes: 0,
            two_byte_ios: vec![],
            n4_of_four_bytes: 0,
            four_byte_ios: vec![],
            n8_of_eight_bytes: 0,
            eight_byte_ios: vec![],
        });

        let serialized = serializer.serialize_packet(&empty_io_packet).unwrap();

        let device_thread = thread::spawn(move || {
            let (mut socket, _) = mock_device.listener.accept().unwrap();
            socket.write_all(&serialized).unwrap();
        });

        let mut connection = Connection::new(device_addr);
        assert!(connection.connect().is_ok());

        if let Some(stream) =
            unsafe { &mut *(&mut connection as *mut Connection).cast::<Connection>() }
                .stream
                .as_mut()
        {
            let result = parser.parse_stream(stream);
            assert!(result.is_ok());
        }
        device_thread.join().unwrap();

        // Test maximum IO elements
        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();
        let mut max_io_packet = create_mock_avl_packet(1);
        max_io_packet.avl_data[0].io = IOElement::Codec8(IOElement8 {
            event_io_id: 0xFF,
            n_total_io: 0xFF,
            n1_of_one_byte: 0xFF,
            one_byte_ios: (0..0xFF).map(|i| (i as u8, i as u8)).collect(),
            n2_of_two_bytes: 0,
            two_byte_ios: vec![],
            n4_of_four_bytes: 0,
            four_byte_ios: vec![],
            n8_of_eight_bytes: 0,
            eight_byte_ios: vec![],
        });

        let serialized = serializer.serialize_packet(&max_io_packet).unwrap();

        let device_thread = thread::spawn(move || {
            let (mut socket, _) = mock_device.listener.accept().unwrap();
            socket.write_all(&serialized).unwrap();
        });

        let mut connection = Connection::new(device_addr);
        assert!(connection.connect().is_ok());

        if let Some(stream) =
            unsafe { &mut *(&mut connection as *mut Connection).cast::<Connection>() }
                .stream
                .as_mut()
        {
            let result = parser.parse_stream(stream);
            assert!(result.is_ok());
        }
        device_thread.join().unwrap();

        // Test invalid codec ID
        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();
        let mut invalid_codec_packet = create_mock_avl_packet(1);
        invalid_codec_packet.codec_id = 0xFF; // Invalid codec ID

        let serialized = serializer.serialize_packet(&invalid_codec_packet).unwrap();

        let device_thread = thread::spawn(move || {
            let (mut socket, _) = mock_device.listener.accept().unwrap();
            socket.write_all(&serialized).unwrap();
        });

        let mut connection = Connection::new(device_addr);
        assert!(connection.connect().is_ok());

        if let Some(stream) =
            unsafe { &mut *(&mut connection as *mut Connection).cast::<Connection>() }
                .stream
                .as_mut()
        {
            let result = parser.parse_stream(stream);
            assert!(result.is_err());
        }
        device_thread.join().unwrap();

        // Test boundary timestamps
        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();
        let mut boundary_timestamp_packet = create_mock_avl_packet(1);
        boundary_timestamp_packet.avl_data[0].timestamp = 0;

        let serialized = serializer
            .serialize_packet(&boundary_timestamp_packet)
            .unwrap();

        let device_thread = thread::spawn(move || {
            let (mut socket, _) = mock_device.listener.accept().unwrap();
            socket.write_all(&serialized).unwrap();
        });

        let mut connection = Connection::new(device_addr);
        assert!(connection.connect().is_ok());

        if let Some(stream) =
            unsafe { &mut *(&mut connection as *mut Connection).cast::<Connection>() }
                .stream
                .as_mut()
        {
            let result = parser.parse_stream(stream);
            assert!(result.is_err());
        }
        device_thread.join().unwrap();
    }
    #[test]
    fn test_packet_size_limits() {
        let mut parser = Parser::new();
        let mut serializer = PacketSerializer::new();

        // Test minimum size packet
        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();
        let min_packet = create_mock_avl_packet(1);
        let min_serialized = serializer.serialize_packet(&min_packet).unwrap();

        assert!(
            min_serialized.len() >= SMALLEST_AVL_SIZE,
            "Minimum packet size should be at least {} bytes, got {}",
            SMALLEST_AVL_SIZE,
            min_serialized.len()
        );

        let device_thread = thread::spawn(move || {
            let (mut socket, _) = mock_device.listener.accept().unwrap();
            socket.write_all(&min_serialized).unwrap();
        });

        let mut connection = Connection::new(device_addr);
        assert!(connection.connect().is_ok());

        if let Some(stream) =
            unsafe { &mut *(&mut connection as *mut Connection).cast::<Connection>() }
                .stream
                .as_mut()
        {
            let result = parser.parse_stream(stream);
            assert!(result.is_ok());
        }
        device_thread.join().unwrap();

        // Test maximum size packet
        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();
        let max_packet = create_mock_avl_packet(255); // Maximum records for FM6XXX
        let max_serialized = serializer.serialize_packet(&max_packet).unwrap();

        assert!(
            max_serialized.len() <= MAX_AVL_PACKET_SIZE_FM6XXX,
            "Maximum packet size should not exceed {} bytes, got {}",
            MAX_AVL_PACKET_SIZE_FM6XXX,
            max_serialized.len()
        );

        let device_thread = thread::spawn(move || {
            let (mut socket, _) = mock_device.listener.accept().unwrap();
            socket.write_all(&max_serialized).unwrap();
        });

        let mut connection = Connection::new(device_addr);
        assert!(connection.connect().is_ok());

        if let Some(stream) =
            unsafe { &mut *(&mut connection as *mut Connection).cast::<Connection>() }
                .stream
                .as_mut()
        {
            let result = parser.parse_stream(stream);
            assert!(result.is_ok());
        }
        device_thread.join().unwrap();

        let mock_device = MockDevice::new();
        let device_addr = mock_device.addr();
        let oversize_packet = create_mock_avl_packet(0xFF); // Trying to exceed maximum
        let oversize_serialized = serializer.serialize_packet(&oversize_packet).unwrap();

        let device_thread = thread::spawn(move || {
            let (mut socket, _) = mock_device.listener.accept().unwrap();
            socket.write_all(&oversize_serialized).unwrap();
        });

        let mut connection = Connection::new(device_addr);
        assert!(connection.connect().is_ok());

        if let Some(stream) =
            unsafe { &mut *(&mut connection as *mut Connection).cast::<Connection>() }
                .stream
                .as_mut()
        {
            let result = parser.parse_stream(stream);
            assert!(result.is_err() || result.unwrap().is_none());
        }
        device_thread.join().unwrap();
    }
    #[cfg(test)]
    mod stress_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier};
        use std::time::Instant;

        // Helper to create variable-sized packets for stress testing
        fn create_variable_packet(size: u8, seed: u64) -> AVLPacket {
            let mut avl_data = Vec::with_capacity(size as usize);
            for i in 0..size {
                let gps = GPSElement {
                    longitude: (((seed + i as u64) % 360) - 180) as i32 * 10000,
                    latitude: (((seed + i as u64) % 180) - 90) as i32 * 10000,
                    altitude: ((seed + i as u64) % 1000) as i16,
                    angle: ((seed + i as u64) % 360) as i16,
                    satellites: ((seed + i as u64) % 12) as u8 + 1,
                    speed: ((seed + i as u64) % 120) as i16,
                };

                let io = IOElement::Codec8(IOElement8 {
                    event_io_id: 1,
                    n_total_io: 1,
                    n1_of_one_byte: 1,
                    one_byte_ios: vec![(1, i)],
                    n2_of_two_bytes: 0,
                    two_byte_ios: vec![],
                    n4_of_four_bytes: 0,
                    four_byte_ios: vec![],
                    n8_of_eight_bytes: 0,
                    eight_byte_ios: vec![],
                });

                avl_data.push(AVLData {
                    timestamp: seed + i as u64 * 1000,
                    priority: (i % 8) as u8,
                    gps,
                    io,
                });
            }

            AVLPacket {
                preamble: 0x00000000,
                data_length: 0,
                codec_id: 0x08,
                number_of_data1: size,
                avl_data,
                number_of_data2: size,
                crc16: 0,
            }
        }

        struct StressDevice {
            listener: TcpListener,
            packet_count: Arc<AtomicUsize>,
            error_count: Arc<AtomicUsize>,
        }

        impl StressDevice {
            fn new() -> Self {
                Self {
                    listener: TcpListener::bind("127.0.0.1:0").unwrap(),
                    packet_count: Arc::new(AtomicUsize::new(0)),
                    error_count: Arc::new(AtomicUsize::new(0)),
                }
            }

            fn addr(&self) -> SocketAddr {
                self.listener.local_addr().unwrap()
            }

            fn stats(&self) -> (usize, usize) {
                (
                    self.packet_count.load(Ordering::Relaxed),
                    self.error_count.load(Ordering::Relaxed),
                )
            }

            fn run_stress_server(
                self,
                duration: Duration,
                packet_interval: Duration,
            ) -> thread::JoinHandle<()> {
                thread::spawn(move || {
                    let (mut socket, _) = self.listener.accept().unwrap();
                    let mut serializer = PacketSerializer::new();
                    let start = Instant::now();
                    let mut sequence = 0u64;

                    while start.elapsed() < duration {
                        // Create and send packet
                        let packet_size = (sequence % 10 + 1) as u8; // Vary packet size
                        let packet = create_variable_packet(packet_size, sequence);

                        match serializer.serialize_packet(&packet) {
                            Ok(data) => {
                                if socket.write_all(&data).is_ok() {
                                    self.packet_count.fetch_add(1, Ordering::Relaxed);
                                } else {
                                    self.error_count.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            Err(_) => {
                                self.error_count.fetch_add(1, Ordering::Relaxed);
                            }
                        }

                        sequence += 1;
                        thread::sleep(packet_interval);
                    }
                })
            }
        }

        #[test]
        fn test_concurrent_connections() {
            const NUM_CONNECTIONS: usize = 50;
            const TEST_DURATION: Duration = Duration::from_secs(30);
            const PACKET_INTERVAL: Duration = Duration::from_millis(100);

            let devices: Vec<_> = (0..NUM_CONNECTIONS).map(|_| StressDevice::new()).collect();

            let barrier = Arc::new(Barrier::new(NUM_CONNECTIONS + 1));

            let handles: Vec<_> = devices
                .into_iter()
                .map(|device| {
                    let addr = device.addr();
                    let barrier = Arc::clone(&barrier);

                    let client_handle = thread::spawn(move || {
                        let mut connection = Connection::new(addr);
                        let mut parser = Parser::new();
                        let mut pipeline = ProcessingPipeline::new(100);
                        let mut state_machine = StateMachine::new(Duration::from_secs(5));
                        let mut total_packets = 0;
                        let mut errors = 0;

                        barrier.wait();

                        if connection.connect().is_ok() {
                            let start = Instant::now();

                            while start.elapsed() < TEST_DURATION {
                                if let Some(stream) = connection.get_stream_mut() {
                                    match parser.parse_stream(stream) {
                                        Ok(Some(packet)) => {
                                            if pipeline
                                                .process_incoming(
                                                    packet,
                                                    Some(Duration::from_millis(50)),
                                                )
                                                .is_ok()
                                            {
                                                total_packets += 1;
                                            } else {
                                                errors += 1;
                                            }
                                        }
                                        Ok(None) => continue,
                                        Err(_) => {
                                            errors += 1;
                                            // Attempt reconnection
                                            if connection.connect().is_ok() {
                                                state_machine.handle_event(ProtocolEvent::Connect);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        (total_packets, errors)
                    });

                    let server = device.run_stress_server(TEST_DURATION, PACKET_INTERVAL);
                    (client_handle, server)
                })
                .collect();

            barrier.wait();

            let mut total_processed = 0;
            let mut total_errors = 0;

            for (client, server) in handles {
                let (processed, errors) = client.join().unwrap();
                total_processed += processed;
                total_errors += errors;
                server.join().unwrap();
            }

            println!("Stress Test Results:");
            println!("Total Connections: {}", NUM_CONNECTIONS);
            println!("Total Packets Processed: {}", total_processed);
            println!("Total Errors: {}", total_errors);
            println!(
                "Error Rate: {:.2}%",
                (total_errors as f64 / total_processed as f64) * 100.0
            );

            assert!(total_processed > 0, "No packets were processed");
            assert!(
                (total_errors as f64 / total_processed as f64) < 0.05,
                "Error rate exceeded 5%"
            );
        }

        #[test]
        fn test_pipeline_throughput() {
            const BATCH_SIZE: usize = 1000;
            const NUM_BATCHES: usize = 100;
            let mut pipeline = ProcessingPipeline::new(BATCH_SIZE);

            let start = Instant::now();
            let mut total_packets = 0;

            for batch in 0..NUM_BATCHES {
                for i in 0..BATCH_SIZE {
                    let packet = create_variable_packet(
                        ((i % 10) + 1) as u8,
                        batch as u64 * BATCH_SIZE as u64 + i as u64,
                    );
                    pipeline.process_incoming(packet, None).unwrap();
                    total_packets += 1;
                }

                // Process accumulated packets
                let processed = pipeline.flush().unwrap();
                assert_eq!(processed.len(), BATCH_SIZE);
            }

            let duration = start.elapsed();
            let throughput = total_packets as f64 / duration.as_secs_f64();

            println!("Pipeline Throughput Test Results:");
            println!("Total Packets: {}", total_packets);
            println!("Processing Time: {:.2?}", duration);
            println!("Throughput: {:.2} packets/second", throughput);

            // Assert minimum throughput requirements
            assert!(throughput > 1000.0, "Throughput below minimum requirement");
        }
    }
}
