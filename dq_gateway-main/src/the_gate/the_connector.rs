//#############################################################################################
//#                                 IMPORTANT INFORMATION                                     #
//#############################################################################################
//#   The codebase is at the moment synchronus. This should be amended when we have a working #
//#   prototype. at the moment, if i am not being too doom and gloom,                         #
//#   somewhere around 70%+ of the time used by this approach would likely                    #
//#   be on just waiting for things.                                                          #
//#############################################################################################

use super::*;
use std::thread;

//   The Connection struct is very much like a phone line between
//   our system and a device:
// - stream: The actual phone line between us (TCP connection)
// - addr: The phone number we're calling (IP address and the port)
// - reconnect_attempts: How many times we've tried to call back
// - session_id: A unique ID for this conversation (like a call reference number in a log)
pub struct Connection {
    pub stream: Option<TcpStream>,
    addr: SocketAddr,
    reconnect_attempts: u32,
    session_id: Option<u32>,
}

impl Connection {
    // Start with a fresh connection, making a new phonecall
    // putting in the number we want to call
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            stream: None,
            addr,
            reconnect_attempts: 0,
            session_id: None,
        }
    }
    // Get access to our phone line if it's connected
    pub fn get_stream_mut(&mut self) -> Option<&mut TcpStream> {
        self.stream.as_mut()
    }

    // Try to establish a connection.
    // We'll try to call up to 3 times with short pauses between attempts,
    // becasue we don't want to seem needy
    pub fn connect(&mut self) -> io::Result<()> {
        if self.is_connected() {
            return Ok(()); // Already on a call.
        }

        let mut last_error = None;

        // Try calling three times with increasing delays.
        // The key is to give them time to calm down, or
        // finish whatever they are doing.
        for attempt in 0..3 {
            match TcpStream::connect_timeout(&self.addr, Duration::from_secs(5)) {
                Ok(stream) => {
                    Self::configure_stream(&stream)?;
                    self.stream = Some(stream);
                    self.reconnect_attempts = 0;
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);

                    // Wait a bit longer between each retry (Should be looked into being made asynch)
                    thread::sleep(Duration::from_millis(100 * (attempt + 1)));
                    println!("{:?}", last_error);
                }
            }
        }

        // One final attempt before succumbing to the fact she doesn't want to talk to us.
        let stream = match TcpStream::connect_timeout(&self.addr, Duration::from_secs(5)) {
            Ok(stream) => stream,
            Err(e) => {
                self.reconnect_attempts += 1;
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    format!(
                        "Connection attempt {} failed: {}",
                        self.reconnect_attempts, e
                    ),
                ));
            }
        };

        // Set up the connection quality settings
        Self::configure_stream(&stream)?;

        self.stream = Some(stream);
        self.reconnect_attempts = 0;
        Ok(())
    }

    // Check if we're currently connected
    // Like asking if they are still there
    pub fn is_connected(&self) -> bool {
        self.stream
            .as_ref()
            .map(|s| !s.peer_addr().is_err())
            .unwrap_or(false)
    }

    // End the connection properly.
    // Like saying goodbye before hanging up the phone
    pub fn shutdown(&mut self) -> io::Result<()> {
        if let Some(stream) = self.stream.take() {
            stream.shutdown(std::net::Shutdown::Both)?;
        }
        Ok(())
    }

    // Configure the connection settings
    // Things like adjusting the quality of the call and the timeout settings
    fn configure_stream(stream: &TcpStream) -> io::Result<()> {
        stream.set_nodelay(true)?; // eagerly send data immediately
        stream.set_read_timeout(Some(Duration::from_secs(30)))?; // How long are we willing to wait for them to respond?
        stream.set_write_timeout(Some(Duration::from_secs(5)))?; // How long are we willing to talk before we shut up?
        Ok(())
    }
}
