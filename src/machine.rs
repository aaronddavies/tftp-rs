use std::cmp::min;
use crate::constants::{ErrorCode, Mode, OpCode, MAX_DATA_SIZE, FIXED_DATA_BYTES};
use crate::constants::RequestType;
use crate::constants::MAX_PACKET_SIZE;
use crate::constants::DEFAULT_DESTINATION_TID;

use crate::errors::TftprsError;

use rand::rngs::SmallRng;
use rand::Rng;
use crate::serial::Ack;
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

    pub fn write_file(&mut self, filename: String, file: &'a mut Vec<u8>, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {
        self.block = 0;
        self.send_request(RequestType::Write, filename, file, outgoing)
    }

    pub fn read_file(&mut self, filename: String, file: &'a mut Vec<u8>, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {
        self.block = 1;
        self.send_request(RequestType::Read, filename, file, outgoing)
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

    fn send_error(&mut self, code: ErrorCode, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {

    }

    pub fn process_reply(&mut self, received: &mut [u8; MAX_PACKET_SIZE], length: usize, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {
        if length > MAX_PACKET_SIZE {
            return Err(TftprsError::BadPacketReceived)
        }
        if let Ok(opcode_bytes) = received[0..2].try_into() {
            let opcode: u16 = u16::from_be_bytes(opcode_bytes);
            if let Ok(opcode_match) = OpCode::try_from(opcode) {
                match opcode_match {
                    OpCode::Acknowledgement => {
                        if let Some(RequestType::Write) = self.request_type {
                            self.handle_ack(received, outgoing)
                        } else {
                            Err(TftprsError::BadPacketReceived)
                        }
                    },
                    OpCode::Data => {
                        if let Some(RequestType::Read) = self.request_type {
                            self.handle_data(received, length - FIXED_DATA_BYTES, outgoing)
                        } else {
                            Err(TftprsError::BadPacketReceived)
                        }
                    },
                    _ => {
                        self.send_error(ErrorCode::IllegalOperation, outgoing)
                    },
                }
            } else {
                self.send_error(ErrorCode::IllegalOperation, outgoing)
            }
        } else {
            Err(TftprsError::BadPacketReceived)
        }
    }

    fn check_block(&self, received: &mut [u8; MAX_PACKET_SIZE]) -> Result<(), TftprsError> {
        if let Ok(block_bytes) = received[2..4].try_into() {
            let block = u16::from_be_bytes(block_bytes);
            if block != self.block {
                return Err(TftprsError::BadPacketReceived)
            }
        }
        Ok(())
    }

    fn handle_ack(&mut self, received: &mut [u8; MAX_PACKET_SIZE], outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {
        // Verify the header.
        self.check_block(received)?;
        // Advance the block for the next write.
        self.block += 1;
        let offset = self.block as usize * MAX_DATA_SIZE;
        if let Some(file) = &self.file {
            if offset >= file.len() {
                // If there is no more data to write, then terminate.
                self.terminated = true;
                Ok(Header {
                    length: 0,
                    source: self.source,
                    destination: self.destination,
                })
            } else {
                // Send the next packet.
                let packet_size = min(file.len() - offset, MAX_DATA_SIZE);
                let data = Data::new(self.block, file, packet_size);
                let count = data.serialize(outgoing);
                Ok(Header {
                    length: count,
                    source: self.source,
                    destination: self.destination,
                })
            }
        } else {
            Err(TftprsError::NoFile)
        }
    }

    fn handle_data(&mut self, received: &mut [u8; MAX_PACKET_SIZE], length: usize, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Header, TftprsError> {
        // Verify the header.
        self.check_block(received)?;
        if let Some(file) = &mut self.file {
            // Write the received data.
            file[self.block as usize..self.block as usize + length].copy_from_slice(&received[0..length]);
        } else {
            return Err(TftprsError::NoFile)
        }
        if length < MAX_DATA_SIZE {
            // If there is no more data coming, then terminate.
            self.terminated = true;
        } else {
            // Otherwise, advance for the next data packet.
            self.block += 1;
        }
        // Acknowledge the received data.
        let ack = Ack::new(self.block);
        let count = ack.serialize(outgoing);
        Ok(Header {
            length: count,
            source: self.source,
            destination: self.destination,
        })
    }
}
