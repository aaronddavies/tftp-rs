use crate::constants::{Mode, OpCode, MAX_DATA_SIZE};
use crate::constants::RequestType;
use crate::constants::MAX_PACKET_SIZE;
use crate::constants::DEFAULT_DESTINATION_TID;

use crate::errors::TftprsError;

use crate::states::{State, TerminalState};
use crate::states::IdleState;

use rand::rngs::SmallRng;
use rand::Rng;
use crate::serial::{Data, Request};
use crate::serial::Serial;

/// This represents a message header for the caller to use when transmitting the message over an
///   enveloping protocol.
pub struct Header {
    length: usize,
    source: u16,
    destination: u16,
}

/// This implements the state machine for the protocol.
/// This machine will process received messages and provide the caller formatted outgoing messages.
/// It is up to the caller to perform actual network send and receive operations.
pub struct Machine<'a> {
    request_type: Option<RequestType>,
    terminated: bool,
    source: u16,
    destination: u16,
    file: Option<&'a mut Vec<u8>>,
    mode: Mode,
    block: u16,
}

impl<'a> Machine<'a> {
    pub fn new() -> Machine<'a> {
        let mut rng :SmallRng = rand::make_rng();
        let source: u16 = rng.next_u32() as u16;
        Self {
            request_type: None,
            terminated: false,
            source,
            destination: DEFAULT_DESTINATION_TID,
            file: None,
            mode: Mode::Binary,
            block: 0,
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if !self.is_busy() {
            self.mode = mode;
        }
    }

    pub fn is_terminated(&self) -> bool {
        self.terminated
    }

    pub fn is_busy(&self) -> bool {
        self.request_type.is_some() && !self.terminated
    }

    /// Make a write request to a remote server.
    /// # Arguments:
    /// * `filename`: The name of the file for the server to write.
    /// * `file`: The file to send
    /// * `outgoing`: The outgoing TX buffer provided by the caller to collect the outgoing message.
    /// # Returns:
    /// * `Ok(Header)`: The source ID, destination ID, and number of bytes of the outgoing message.
    fn send_request(&mut self, request_type: RequestType, filename: String, file: &'a mut Vec<u8>, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {
        if self.is_busy() {
            return Err(TftprsError::Busy)
        }
        if let Ok(request) = Request::new(request_type, self.mode, filename) {
            let count = request.serialize(outgoing);
            if request.serialize(outgoing) > 0 {
                self.file = Some(file);
                self.request_type = Some(RequestType::Write);
                self.terminated = false;
                Ok(Header {
                    length: count,
                    source: self.source,
                    destination: self.destination,
                })
            } else {
                Err(TftprsError::BadRequestAttempted)
            }
        } else {
            Err(TftprsError::BadRequestAttempted)
        }
    }

    pub fn write_file(&mut self, filename: String, file: &'a mut Vec<u8>, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {
        self.send_request(RequestType::Write, filename, file, outgoing)
    }

    pub fn read_file(&mut self, filename: String, file: &'a mut Vec<u8>, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {
        self.send_request(RequestType::Read, filename, file, outgoing)
    }

    pub fn process_message(&mut self, received: &mut [u8; MAX_PACKET_SIZE], outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {
        if let Ok(opcode_bytes) = received[0..2].try_into() {
            let opcode: u16 = u16::from_be_bytes(opcode_bytes);
            match opcode {
                OpCode::Acknowledgement => {
                    self.handle_ack(received, outgoing)
                },
                OpCode::Data => {
                    self.handle_data(received, outgoing)
                },
                 => {
                    self.handle_read_request(received, outgoing)
                },
                OpCode::WriteRequest => {
                    self.handle_write_request(received, outgoing)
                }

            }
            if opcode != OpCode::Acknowledgement as u16 {
                return Err(TftprsError::BadPacketReceived)
            }
        } else {
            return Err(TftprsError::BadPacketReceived)
        }
        if let Ok(block_bytes) = received[2..4].try_into() {
            let block = u16::from_be_bytes(block_bytes);
            if block != self.block {
                return Err(TftprsError::BadPacketReceived)
            }
        }
        let new_block = self.block + 1;
        let offset = new_block as usize * MAX_DATA_SIZE;
        if offset >= self.source.len() {
            return Ok(Box::new(TerminalState::default()))
        }
        let data = Data::new(new_block, self.source);
        data.serialize(outgoing);
    }
}
