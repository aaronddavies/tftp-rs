//! Serialization of messages

use crate::constants::BINARY_MODE;
use crate::constants::FIXED_REQUEST_BYTES;
use crate::constants::MAX_DATA_SIZE;
use crate::constants::MAX_PACKET_SIZE;
use crate::constants::TEXT_MODE;

use crate::constants::ErrorCode;
use crate::constants::Mode;
use crate::constants::OpCode;
use crate::constants::RequestType;

use crate::errors::TftprsError;

pub(crate) trait Serial {
    fn serialize(&self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> usize;
}

/// Any transfer begins with a request to read or write a file, which also serves to request a connection.
///
/// The size of filename must not exceed `match mode { binary => 503, text => 500 }` bytes
/// `(512 - 4 fixed - mode string)`
///
/// The request will take ownership of the filename.
#[derive(Debug, Clone)]
pub(crate) struct Request {
    // RRQ and WRQ packets (opcodes 1 and 2 respectively)
    request: RequestType,
    // The file name is a sequence of bytes in netascii.
    filename: String,
    // The mode field contains the string "netascii", "octet", or "mail" (or any combination of upper
    //    and lower case, such as "NETASCII", NetAscii", etc.) in netascii indicating the three modes defined in the protocol.
    mode: Mode,
}

impl Request {
    fn filename_fits(mode: Mode, filename: &str) -> bool {
        let mode_size = match mode {
            Mode::Text => TEXT_MODE.len(),
            Mode::Binary => BINARY_MODE.len(),
        };
        let max_filename_size = MAX_PACKET_SIZE - FIXED_REQUEST_BYTES - mode_size;
        filename.len() <= max_filename_size
    }

    pub(crate) fn new(
        request: RequestType,
        mode: Mode,
        filename: String,
    ) -> Result<Self, TftprsError> {
        if Request::filename_fits(mode, &filename) {
            Ok(Self {
                request,
                filename,
                mode,
            })
        } else {
            Err(TftprsError::BadRequestAttempted)
        }
    }
}

impl Serial for Request {
    fn serialize(&self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> usize {
        if !Request::filename_fits(self.mode, &self.filename) {
            return 0;
        }
        let mut head = 0;
        buffer[head..].copy_from_slice(&(self.request as u16).to_be_bytes());
        head += 2;
        buffer[head..].copy_from_slice(self.filename.as_bytes());
        head += self.filename.len();
        buffer[head] = 0;
        head += 1;
        let mode_string = match self.mode {
            Mode::Text => TEXT_MODE,
            Mode::Binary => BINARY_MODE,
        };
        buffer[head..].copy_from_slice(mode_string.as_bytes());
        head += mode_string.len();
        buffer[head] = 0;
        head += 1;
        head
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Data<'a> {
    block: u16,
    data: &'a Vec<u8>,
    length: usize,
}

impl<'a> Data<'a> {
    pub(crate) fn new(block: u16, data: &'a Vec<u8>, length: usize) -> Self {
        Self {
            block,
            data,
            length,
        }
    }

    pub(crate) fn offset(&self) -> usize {
        self.block as usize * MAX_DATA_SIZE
    }
}

impl<'a> Serial for Data<'a> {
    fn serialize(&self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> usize {
        let mut head = 0;
        buffer[head..].copy_from_slice(&(OpCode::Data as u16).to_be_bytes());
        head += 2;
        buffer[head..].copy_from_slice(&self.block.to_be_bytes());
        head += 2;
        let offset = self.offset();
        buffer[head..head + self.length].copy_from_slice(&self.data[offset..offset + self.length]);
        head += self.length;
        head
    }
}

pub(crate) struct Ack {
    block: u16,
}

impl Ack {
    pub fn new(block: u16) -> Self {
        Self { block }
    }
}

impl Serial for Ack {
    fn serialize(&self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> usize {
        let mut head = 0;
        buffer[head..].copy_from_slice(&(OpCode::Acknowledgement as u16).to_be_bytes());
        head += 2;
        buffer[head..].copy_from_slice(&self.block.to_be_bytes());
        head += 2;
        head
    }
}

/// Most errors cause termination of the connection.
/// An error is signalled by sending an error packet.
#[derive(Debug, Clone)]
pub(crate) struct Error {
    // The error code is an integer indicating the nature of the error.
    code: ErrorCode,
    // The error message is intended for human consumption.
    message: String,
}

impl Error {
    pub fn new(error_code: ErrorCode, message: String) -> Self {
        Self {
            code: error_code,
            message,
        }
    }
}

impl Serial for Error {
    fn serialize(&self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> usize {
        let mut head = 0;
        buffer[head..].copy_from_slice(&(OpCode::Error as u16).to_be_bytes());
        head += 2;
        buffer[head..].copy_from_slice(&(self.code as u16).to_be_bytes());
        head += 2;
        buffer[head..].copy_from_slice(self.message.as_bytes());
        head += self.message.len();
        head
    }
}

mod test {
    use super::*;
    #[test]
    fn test_request() {

    }
}
