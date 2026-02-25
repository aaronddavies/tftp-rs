use std::ptr::addr_of_mut;
use crate::constants::{Mode, MAX_DATA_SIZE};
use crate::constants::OpCode;
use crate::constants::FIXED_DATA_OFFSET;
use crate::constants::RequestType;
use crate::constants::MAX_PACKET_SIZE;
use crate::serial::Data;
use crate::errors::TftprsError;
use crate::serial::Request;
use crate::serial::Serial;

/// The generic interface for state in the state machine.
pub(crate) trait State<'a> {
    // By default, a new request cannot be made unless the state implements it.
    fn request(
        &mut self,
        _request_type: RequestType,
        _mode: Mode,
        _location: String,
        _storage: &'a mut Vec<u8>,
        _buffer: &mut [u8; MAX_PACKET_SIZE],
        _length: &mut usize,
    ) -> Result<Box<dyn State>, TftprsError> {
        Err(TftprsError::Busy)
    }

    // All states must be able to receive a message.
    fn receive(&mut self, received: &[u8; MAX_PACKET_SIZE], outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Box<dyn State>, TftprsError>;

    fn terminated(&self) -> bool {
        false
    }
    
    fn idle(&self) -> bool {
        false
    }
}

/// This state can either listen for requests or submit a new request.
#[derive(Debug, Default)]
pub(crate) struct IdleState {}

impl<'a> State<'a> for IdleState {
    fn request(
        &mut self,
        request_type: RequestType,
        mode: Mode,
        location: String,
        storage: &'a mut Vec<u8>,
        buffer: &mut [u8; MAX_PACKET_SIZE],
        length: &mut usize,
    ) -> Result<Box<dyn State<'a> + 'a>, TftprsError > {
        if let Ok(request) = Request::new(request_type, mode, location) {
            *length = request.serialize(buffer);
            if length > &mut 0 {
                match request_type {
                    RequestType::Read => Ok(Box::new(IdleState{})),
                    RequestType::Write => Ok(Box::new(WriteState::new(
                        0,
                        storage
                    ))),
                }
            } else {
                Err(TftprsError::BadRequestAttempted)
            }
        } else {
            Err(TftprsError::BadRequestAttempted)
        }
    }

    fn receive(&mut self, _received: &[u8; MAX_PACKET_SIZE], _outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Box<dyn State>, TftprsError> {
        // TODO - handle requests
        Ok(Box::new(IdleState::default()))
    }
    
    fn idle(&self) -> bool {
        true
    }
}


#[derive(Debug)]
pub(crate) struct WriteState<'a> {
    block: u16,
    source: &'a Vec<u8>,
}

impl<'a> WriteState<'a> {
    pub fn new(block: u16, source: &'a Vec<u8>) -> Self {
        Self {
            block,
            source
        }
    }
}

impl<'a> State<'a> for WriteState<'a> {
    fn receive(&mut self, received: &[u8; MAX_PACKET_SIZE], outgoing: &mut [u8; MAX_PACKET_SIZE]) -> Result<Box<dyn State>, TftprsError> {

        Ok(Box::new(WriteState::new(self.block + 1, self.source)))
    }
}


#[derive(Debug, Default)]
pub(crate) struct TerminalState {}

impl State for TerminalState {
    fn receive(&mut self, message: &[u8; MAX_PACKET_SIZE]) -> Result<Box<dyn State>, TftprsError> {
        Err(TftprsError::Terminated)
    }
    fn terminated(&self) -> bool {
        true
    }
}
