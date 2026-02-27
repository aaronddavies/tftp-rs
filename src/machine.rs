//! Definition of the TFTP protocol state machine / message engine

use crate::constants::BINARY_MODE;
use crate::constants::MAX_PACKET_SIZE;
use crate::constants::RequestType;
use crate::constants::TEXT_MODE;
use crate::constants::{ErrorCode, FIXED_DATA_BYTES, MAX_DATA_SIZE, Mode, OpCode};

use crate::errors::TftprsError;

use crate::serial::Serial;
use crate::serial::{Ack, Error};
use crate::serial::{Data, Request};

const TERMINATOR_BYTE: u8 = 0x0;

/// This machine operates as the transfer engine for the protocol. It provides an interface for
/// initiating transfers and for handling transfer requests.
///
/// It will process incoming messages and provide the caller formatted outgoing messages in reply.
///
/// It is completely synchronous and network-agnostic. Therefore, it is up to the caller to:
///  * Perform actual network send and receive operations.
///  * Handle timing in between messages per the advice in the RFC.
///  * Respond to requests with a file for reading or writing.
///  * Provide a reference to the requested file that lives as long as this machine does.
///  * Manage byte buffers for receiving and transmitting messages.
#[derive(Debug, Default)]
pub struct Machine<'a> {
    request_type: Option<RequestType>,
    incoming_file: Option<&'a mut Vec<u8>>,
    outgoing_file: Option<&'a Vec<u8>>,
    mode: Mode,
    block: u16,
}

impl<'a> Machine<'a> {
    pub fn new() -> Machine<'a> {
        let mut me = Self::default();
        me.reset();
        me
    }

    /// Resets the machine to an idle state.
    pub fn reset(&mut self) {
        self.request_type = None;
        self.incoming_file = None;
        self.outgoing_file = None;
        self.block = 0;
    }

    /// Sets the file mode. This can only be done when no transfer is being performed.
    pub fn set_mode(&mut self, mode: Mode) -> Result<(), TftprsError> {
        if self.is_busy() {
            return Err(TftprsError::Busy);
        }
        self.mode = mode;
        Ok(())
    }

    /// Indicates whether a transfer is being performed.
    pub fn is_busy(&self) -> bool {
        self.request_type.is_some()
    }

    /// Informs the caller what kind of request, from the perspective of the caller, is being performed.
    /// For example, if the caller initiates a write request, this type will reflect it. If the remote peer
    /// initiates a write request, this will be reflected as a read request. If the caller receives a request,
    /// it should check this type to determine whether to send a file or receive a file in reply.
    pub fn request_type(&self) -> Option<RequestType> {
        self.request_type
    }

    /// Indicates what format of file is being transferred. THe default is binary.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Sends a request to the remote peer to send / write a file out to that peer.
    pub fn request_send_file(
        &mut self,
        filename: String,
        file: &'a Vec<u8>,
        outgoing: &mut [u8; MAX_PACKET_SIZE],
    ) -> Result<usize, TftprsError> {
        // Do not send a request if a transaction is already taking place.
        if self.is_busy() {
            return Err(TftprsError::Busy);
        }
        // Do not allow files that are too large for the block field.
        if file.len() > u16::MAX as usize * MAX_DATA_SIZE {
            return Err(TftprsError::BadRequestAttempted);
        }
        // Expect an ack at block 0
        self.block = 0;
        if let Ok(request) = Request::new(RequestType::Write, self.mode, filename) {
            let count = request.serialize(outgoing);
            if request.serialize(outgoing) > 0 {
                self.outgoing_file = Some(file);
                self.request_type = Some(RequestType::Write);
                Ok(count)
            } else {
                Err(TftprsError::BadRequestAttempted)
            }
        } else {
            Err(TftprsError::BadRequestAttempted)
        }
    }

    /// Sends a request to the remote peer to receive / read a file from that peer.
    pub fn request_receive_file(
        &mut self,
        filename: String,
        file: &'a mut Vec<u8>,
        outgoing: &mut [u8; MAX_PACKET_SIZE],
    ) -> Result<usize, TftprsError> {
        // Do not send a request if a transaction is already taking place.
        if self.is_busy() {
            return Err(TftprsError::Busy);
        }
        // Do not allow files that are too large for the block field.
        if file.len() > u16::MAX as usize * MAX_DATA_SIZE {
            return Err(TftprsError::BadRequestAttempted);
        }
        // Expect first block of data in response
        self.block = 1;
        if let Ok(request) = Request::new(RequestType::Read, self.mode, filename) {
            let count = request.serialize(outgoing);
            if request.serialize(outgoing) > 0 {
                self.incoming_file = Some(file);
                self.request_type = Some(RequestType::Read);
                Ok(count)
            } else {
                Err(TftprsError::BadRequestAttempted)
            }
        } else {
            Err(TftprsError::BadRequestAttempted)
        }
    }

    /// Responds to a request from a remote peer to read / receive a file from the caller. This is
    /// a write request from the caller's perspective.
    pub fn reply_send_file(
        &mut self,
        file: &'a Vec<u8>,
        outgoing: &mut [u8; MAX_PACKET_SIZE],
    ) -> Result<usize, TftprsError> {
        if !self.is_busy() {
            return Err(TftprsError::NoConnection);
        }
        self.outgoing_file = Some(file);
        self.block = 1;
        self.send_block(outgoing)
    }

    /// Responds to a request from a remote peer to write / send a file to the caller. This is a
    /// read request from the caller's perspective.
    pub fn reply_receive_file(
        &mut self,
        file: &'a mut Vec<u8>,
        outgoing: &mut [u8; MAX_PACKET_SIZE],
    ) -> Result<usize, TftprsError> {
        if !self.is_busy() {
            return Err(TftprsError::NoConnection);
        }
        self.incoming_file = Some(file);
        self.block = 0;
        self.send_ack(outgoing)
    }

    /// Listens for (i.e., parses an incoming message) to check for a request from a remote peer.
    pub fn listen_for_request(
        &mut self,
        received: &[u8; MAX_PACKET_SIZE],
    ) -> Result<String, TftprsError> {
        if self.is_busy() {
            return Err(TftprsError::Busy);
        }
        if let Ok(opcode_bytes) = received[0..2].try_into() {
            // Determine dispatch based on op code.
            let opcode: u16 = u16::from_be_bytes(opcode_bytes);
            if let Ok(opcode_match) = OpCode::try_from(opcode) {
                match opcode_match {
                    // Handle incoming write request (read).
                    OpCode::WriteRequest => {
                        let filename = self.parse_request(received)?;
                        self.request_type = Some(RequestType::Read);
                        Ok(filename)
                    }
                    // Handle incoming read request (write).
                    OpCode::ReadRequest => {
                        let filename = self.parse_request(received)?;
                        self.request_type = Some(RequestType::Write);
                        Ok(filename)
                    }
                    // This was an attempt to send us transfer messages when there is no connection,
                    //  or it was an unexpected error packet
                    _ => Err(TftprsError::NoConnection),
                }
            } else {
                Err(TftprsError::BadPacketReceived)
            }
        } else {
            Err(TftprsError::BadPacketReceived)
        }
    }

    /// Processes incoming messages while a transfer is active. It does not matter who initiated the transfer,
    /// whether it was the caller or the remote peer.
    pub fn process(
        &mut self,
        received: &[u8; MAX_PACKET_SIZE],
        length: usize,
        outgoing: &mut [u8; MAX_PACKET_SIZE],
    ) -> Result<usize, TftprsError> {
        // Drop unexpected packets.
        if !self.is_busy() {
            return Err(TftprsError::NoConnection);
        }
        // Sanity check.
        if length > MAX_PACKET_SIZE {
            return self.send_error(ErrorCode::IllegalOperation, outgoing, None);
        }
        if let Ok(opcode_bytes) = received[0..2].try_into() {
            // Determine dispatch based on op code.
            let opcode: u16 = u16::from_be_bytes(opcode_bytes);
            if let Ok(opcode_match) = OpCode::try_from(opcode) {
                match opcode_match {
                    // Handle ack if we are writing.
                    OpCode::Acknowledgement => {
                        if let Some(RequestType::Write) = self.request_type {
                            self.handle_ack_and_send_next_block(received, outgoing)
                        } else {
                            Err(TftprsError::BadPacketReceived)
                        }
                    }
                    // Handle data if we are reading.
                    OpCode::Data => {
                        if let Some(RequestType::Read) = self.request_type {
                            self.handle_data_and_send_ack(
                                received,
                                length - FIXED_DATA_BYTES,
                                outgoing,
                            )
                        } else {
                            Err(TftprsError::BadPacketReceived)
                        }
                    }
                    // Terminate on error.
                    OpCode::Error => {
                        self.reset();
                        Ok(0)
                    }
                    // This was an attempt to send us a request when we already busy.
                    _ => Err(TftprsError::Busy),
                }
            } else {
                self.send_error(ErrorCode::IllegalOperation, outgoing, None)
            }
        } else {
            Err(TftprsError::BadPacketReceived)
        }
    }

    /// Formulate an error and write it to the transmit buffer. THe caller can do this at any time.
    /// This oeration automatically resets the machine.
    pub fn send_error(
        &mut self,
        code: ErrorCode,
        outgoing: &mut [u8; MAX_PACKET_SIZE],
        message: Option<String>,
    ) -> Result<usize, TftprsError> {
        let error_message =
            Error::new(code, message.unwrap_or_else(|| "Unknown error".to_string()));
        let count = error_message.serialize(outgoing);
        self.reset();
        Ok(count)
    }

    /// Helper to parse a variable length string in a message.
    fn parse_string(
        &mut self,
        received: &[u8; MAX_PACKET_SIZE],
        cursor: &mut usize,
        cursor_limit: usize,
    ) -> Result<String, TftprsError> {
        let mut result = String::new();
        while received[*cursor] != TERMINATOR_BYTE {
            result.push(char::from(received[*cursor]));
            *cursor += 1;
            if *cursor >= cursor_limit {
                return Err(TftprsError::BadPacketReceived);
            }
        }
        // Step past terminator byte
        *cursor += 1;
        Ok(result)
    }

    /// Helper to parse an incoming request from a peer.
    fn parse_request(
        &mut self,
        received: &[u8; MAX_PACKET_SIZE],
    ) -> Result<String, TftprsError> {
        let mut cursor: usize = 2;
        let filename = self.parse_string(
            received,
            &mut cursor,
            MAX_PACKET_SIZE - BINARY_MODE.len() - 2,
        )?;
        let mode = self.parse_string(received, &mut cursor, MAX_PACKET_SIZE - 1)?;
        if mode.eq(TEXT_MODE) {
            self.mode = Mode::Text;
        } else if mode.eq(BINARY_MODE) {
            self.mode = Mode::Binary;
        } else {
            return Err(TftprsError::BadPacketReceived);
        }
        Ok(filename)
    }

    /// Verifies that the block specified in the incoming message is as expected.
    fn check_block_on_message(
        &self,
        received: &[u8; MAX_PACKET_SIZE],
    ) -> Result<(), TftprsError> {
        if let Ok(block_bytes) = received[2..4].try_into() {
            let block = u16::from_be_bytes(block_bytes);
            if block != self.block {
                return Err(TftprsError::BadPacketReceived);
            }
        }
        Ok(())
    }

    /// Writes out the current block of the file.
    fn send_block(&mut self, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<usize, TftprsError> {
        if let Some(file) = &self.outgoing_file {
            if let Some(data) = Data::new(self.block, file) {
                let count = data.serialize(outgoing);
                Ok(count)
            } else {
                self.reset();
                Ok(0)
            }
        } else {
            Err(TftprsError::NoFile)
        }
    }

    /// Checks the last ack, and then sends the next block.
    fn handle_ack_and_send_next_block(
        &mut self,
        received: &[u8; MAX_PACKET_SIZE],
        outgoing: &mut [u8; MAX_PACKET_SIZE],
    ) -> Result<usize, TftprsError> {
        // Verify the header.
        self.check_block_on_message(received)?;
        if self.block == u16::MAX {
            // For safety, automatically terminate.
            self.reset();
            Ok(0)
        } else {
            // Advance the block for the next write.
            self.block += 1;
            self.send_block(outgoing)
        }
    }

    /// Send an ack.
    fn send_ack(&mut self, outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<usize, TftprsError> {
        let ack = Ack::new(self.block);
        let count = ack.serialize(outgoing);
        Ok(count)
    }

    /// Receives the last datagram, and then sends an ack.
    fn handle_data_and_send_ack(
        &mut self,
        received: &[u8; MAX_PACKET_SIZE],
        length: usize,
        outgoing: &mut [u8; MAX_PACKET_SIZE],
    ) -> Result<usize, TftprsError> {
        // Verify the header.
        self.check_block_on_message(received)?;
        if let Some(file) = &mut self.incoming_file {
            // Write the received data.
            for i in 0 .. length {
                let idx = FIXED_DATA_BYTES + i;
                file.push(received[idx]);
            }
        } else {
            return Err(TftprsError::NoFile);
        }
        if length < MAX_DATA_SIZE || self.block == u16::MAX {
            // If there is no more data coming, then terminate.
            self.reset();
        }
        // Acknowledge the received data and advance the block.
        let response = self.send_ack(outgoing);
        self.block += 1;
        response
    }
}
