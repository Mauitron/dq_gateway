//#############################################################################################
//#                                 IMPORTANT INFORMATION                                     #
//#############################################################################################
//#   The codebase is at the moment synchronus. This should be amended when we have a working #
//#   prototype. at the moment, if i am not being too doom and gloom,                         #
//#   somewhere around 70%+ of the time used by this approach would likely                    #
//#   be on just waiting for things.                                                          #
//#############################################################################################

use super::*;

// Think of ProtocolEvent as different things that can happen during a conversation
// between our system and a vehicle's device. Similar to a phone call, we can
// connect, disconnect, receive messages, or encounter problems with the transmission.
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolEvent {
    // When devices want to start or stop talking to us
    Connect,
    Disconnect,
    ConnectionLost,

    // When we're talking to eachother and exchanging actual information
    PacketReceived(AVLPacket),    // Device sent us a message
    PacketSent(u32),              // We sent a message (with an ID)
    AcknowledgementReceived(u32), // Device confirmed they got our message

    // When devices need to prove who they are, it tells us its name
    Authenticate(String, String), // Device says "Hey, I'm device Fjordor"
    AuthSuccess, // We confirmed we know a Fjordor, and that they are who they say they are
    AuthFailure, // 'Fjordor' is a silly name, Something's fishy with their ID

    // When things don't go as planned
    Timeout,       // The Device is not respecting our time by taking too long to respond.
    InvalidPacket, // Got a message we couldn't understand. Is the device drunk?
    ProtocolError(String), // Something else went wrong, this thing is speaking in tongues.
}

// ProtocolState is the different stages of our conversation
// You can think of it as the relationship status between our system and the device
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtocolState {
    Disconnected,   // We're not talking, don't they like us?
    Connected,      // We established a connection but haven't verified their identity
    Authenticating, // We're checking if the device is who it claims to be
    Ready,          // Everything's good, we can exchange some spicy data
    Error,          // Something went wrong, it is not me, it is you.
}

// Not because we are insecure, but when something happens, we need to know two things:
// 1. What state we should be in now
// 2. What actions we should take
#[derive(Debug)]
pub struct ProtocolResult {
    pub state: ProtocolState,
    pub actions: Vec<ProtocolAction>,
}

// These are the different things our system can decide to do
// Like our responses in the conversation
#[derive(Debug)]
pub enum ProtocolAction {
    SendAcknowledgement(u32),   // "Yay! Got your message!"
    RequestRetransmission(u32), // "Could you say that again?"
    DisconnectClient,           // "Yeah, We need to end this conversation"
    SendAuthResponse(bool),     // "Yes, I know you" or "No, who are you?"
    ResetConnection,            // "Let's start over"
}

// The StateMachine is like a evesdropping receptionist who keeps track of:
// - What stage each conversation is in
// - What messages we're waiting for
// - How long we've been waiting
// - Who we're talking to
pub struct StateMachine {
    state: ProtocolState,
    sequence: u32,                        // Helps us keep messages in order
    last_ack: u32,                        // Last message we confirmed
    pending_packets: Vec<(u32, Instant)>, // Messages we're waiting for
    timeout_duration: Duration,           // How long we're prepared to wait
    imei: Option<String>,                 // The device's ID
}

impl StateMachine {
    // Start fresh with a new receptionist,
    // the last one knew too much and had to go.
    pub fn new(timeout_duration: Duration) -> Self {
        Self {
            state: ProtocolState::Disconnected,
            sequence: 0,
            last_ack: 0,
            pending_packets: Vec::new(),
            timeout_duration,
            imei: None,
        }
    }

    // This is the main brain of our system, it decides what to say when things happen
    // Like a controlling partner that tells us how to responses to different situations.
    pub fn handle_event(&mut self, event: ProtocolEvent) -> ProtocolResult {
        let mut actions = Vec::new();

        let new_state = match (self.state, event) {
            // Handling new connections, telling us to pick up the phone.
            (ProtocolState::Disconnected, ProtocolEvent::Connect) => ProtocolState::Connected,
            // Device introduces itself - like saying "Hi, I'm Bertil"
            (ProtocolState::Connected, ProtocolEvent::Authenticate(imei, _)) => {
                self.imei = Some(imei);
                ProtocolState::Authenticating
            }
            // Device passed the identity check. Even though we don't want to, we know Bertil
            (ProtocolState::Authenticating, ProtocolEvent::AuthSuccess) => ProtocolState::Ready,

            // When we're ready and receiving data, when to litsen to Bertils endless moaning.
            (ProtocolState::Ready, ProtocolEvent::PacketReceived(packet)) => {
                actions.push(ProtocolAction::SendAcknowledgement(packet.crc16));
                self.handle_packet(packet, &mut actions);
                ProtocolState::Ready
            }

            // Device confirmed they got our message, Bertil starts yapping.
            (ProtocolState::Ready, ProtocolEvent::AcknowledgementReceived(packet_id)) => {
                self.handle_acknowledgement(packet_id);
                ProtocolState::Ready
            }

            // Oh no, we lost connection. Totally by accident, such an unfortunate turn of events...
            (current_state, ProtocolEvent::ConnectionLost) => {
                actions.push(ProtocolAction::ResetConnection);
                ProtocolState::Disconnected
            }

            // We waited too long for a response, is he dead?
            (current_state, ProtocolEvent::Timeout) => {
                self.handle_timeout(&mut actions);
                if current_state != ProtocolState::Error {
                    ProtocolState::Error
                } else {
                    current_state
                }
            }

            // Something unexpected happened
            (current_state, _) => {
                actions.push(ProtocolAction::DisconnectClient);
                ProtocolState::Error
            }
        };

        self.state = new_state;
        ProtocolResult {
            state: new_state,
            actions,
        }
    }

    // When we receive a packet, we need to:
    // 1. Keep track of it
    // 2. Make sure we're not missing any earlier packets
    // 3. Let the device know we got it
    fn handle_packet(&mut self, packet: AVLPacket, actions: &mut Vec<ProtocolAction>) {
        // Remember this packet if we haven't seen it before
        if !self
            .pending_packets
            .iter()
            .any(|(id, _)| *id == packet.crc16)
        {
            self.pending_packets.push((packet.crc16, Instant::now()));
        }

        // Check if we missed any packets
        if packet.crc16 != self.last_ack + 1 {
            actions.push(ProtocolAction::RequestRetransmission(self.last_ack + 1));
        } else {
            self.last_ack = packet.crc16;
        }
    }

    // When device confirms they got our message, we can stop waiting for it
    fn handle_acknowledgement(&mut self, packet_id: u32) {
        self.pending_packets.retain(|(id, _)| *id != packet_id);
    }

    // Check if we've been waiting too long for any messages
    // going over all the unanswered texts
    fn handle_timeout(&mut self, actions: &mut Vec<ProtocolAction>) {
        // Check for timed out packets
        let now = Instant::now();
        let timed_out: Vec<_> = self
            .pending_packets
            .iter()
            .filter(|(_, timestamp)| now.duration_since(*timestamp) > self.timeout_duration)
            .map(|(id, _)| *id)
            .collect();

        // We promised ourselves we wouldn't, but we text back for any messages we've waited too long for
        for packet_id in timed_out {
            actions.push(ProtocolAction::RequestRetransmission(packet_id));
        }

        // We clean up our waiting list and move on
        self.pending_packets
            .retain(|(_, timestamp)| now.duration_since(*timestamp) <= self.timeout_duration);
    }
}
