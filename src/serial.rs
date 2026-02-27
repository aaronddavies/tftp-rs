//! Serialization of messages

use crate::constants::BINARY_MODE;
use crate::constants::FIXED_REQUEST_BYTES;
use crate::constants::MAX_DATA_SIZE;
use crate::constants::MAX_PACKET_SIZE;
use crate::constants::TEXT_MODE;
use std::cmp::min;

use crate::constants::ErrorCode;
use crate::constants::Mode;
use crate::constants::OpCode;
use crate::constants::TransferType;

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
    request: TransferType,
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
        request: TransferType,
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
        write_bytes(buffer, &mut head, &(self.request as u16).to_be_bytes());
        write_bytes(buffer, &mut head, self.filename.as_bytes());
        write_bytes(buffer, &mut head, &[0x0]);
        let mode_string = match self.mode {
            Mode::Text => TEXT_MODE,
            Mode::Binary => BINARY_MODE,
        };
        write_bytes(buffer, &mut head, mode_string.as_bytes());
        write_bytes(buffer, &mut head, &[0x0]);
        head
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Data<'a> {
    block: u16,
    data: &'a Vec<u8>,
}

impl<'a> Data<'a> {
    pub(crate) fn new(block: u16, data: &'a Vec<u8>) -> Option<Self> {
        if block == 0 {
            return None;
        }
        if (block - 1) as usize * MAX_DATA_SIZE > data.len() {
            return None;
        }
        Some(Self { block, data })
    }

    fn offset(&self) -> usize {
        (self.block - 1) as usize * MAX_DATA_SIZE
    }
}

impl<'a> Serial for Data<'a> {
    fn serialize(&self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> usize {
        let mut head = 0;
        write_bytes(buffer, &mut head, &(OpCode::Data as u16).to_be_bytes());
        write_bytes(buffer, &mut head, &self.block.to_be_bytes());
        let count = min(MAX_DATA_SIZE, self.data.len() - self.offset());
        write_bytes(
            buffer,
            &mut head,
            &self.data[self.offset()..self.offset() + count],
        );
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
        write_bytes(
            buffer,
            &mut head,
            &(OpCode::Acknowledgement as u16).to_be_bytes(),
        );
        write_bytes(buffer, &mut head, &self.block.to_be_bytes());
        head
    }
}

/// Most errors cause termination of the connection.
/// An error is signalled by sending an error packet.
#[derive(Debug, Clone)]
pub(crate) struct ErrorResponse {
    // The error code is an integer indicating the nature of the error.
    code: ErrorCode,
    // The error message is intended for human consumption.
    message: String,
}

impl ErrorResponse {
    pub fn new(error_code: ErrorCode, message: String) -> Self {
        Self {
            code: error_code,
            message,
        }
    }
}

impl Serial for ErrorResponse {
    fn serialize(&self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> usize {
        let mut head = 0;
        write_bytes(buffer, &mut head, &(OpCode::Error as u16).to_be_bytes());
        write_bytes(buffer, &mut head, &(self.code as u16).to_be_bytes());
        write_bytes(buffer, &mut head, self.message.as_bytes());
        head
    }
}

/// Helper to write bytes from source to buffer and advance the head pointer.
fn write_bytes(buffer: &mut [u8; MAX_PACKET_SIZE], head: &mut usize, source: &[u8]) {
    let count = source.len();
    buffer[*head..*head + count].copy_from_slice(source);
    *head += count;
}

mod test {
    #[cfg(test)]
    use super::*;
    #[test]
    fn test_read_request() {
        let request = Request::new(TransferType::Read, Mode::Binary, String::from("ABCDE"));
        let mut tx_buffer = [0u8; MAX_PACKET_SIZE];
        request.unwrap().serialize(&mut tx_buffer);
        let expected: [u8; 14] = [
            0x0, 0x1, 0x41, 0x42, 0x43, 0x44, 0x45, 0x0, 0x4F, 0x43, 0x54, 0x45, 0x54, 0x0,
        ];
        assert_eq!(expected, tx_buffer[0..14]);
    }

    #[test]
    fn test_write_request() {
        let request = Request::new(TransferType::Write, Mode::Text, String::from("ABCDE"));
        let mut tx_buffer = [0u8; MAX_PACKET_SIZE];
        request.unwrap().serialize(&mut tx_buffer);
        let expected: [u8; 17] = [
            0x0, 0x2, 0x41, 0x42, 0x43, 0x44, 0x45, 0x0, 0x4E, 0x45, 0x54, 0x41, 0x53, 0x43, 0x49,
            0x49, 0x0,
        ];
        assert_eq!(expected, tx_buffer[0..17]);
    }

    #[test]
    fn test_bad_request() {
        let request = Request::new(
            TransferType::Write,
            Mode::Binary,
            String::from(['H'; 512].iter().collect::<String>()),
        );
        assert!(request.is_err());
    }

    #[test]
    fn test_one_small_gram_data() {
        let my_datagram: Vec<u8> = vec![0x5a, 0xa5];
        let data = Data::new(1, &my_datagram);
        let mut tx_buffer = [0u8; MAX_PACKET_SIZE];
        data.unwrap().serialize(&mut tx_buffer);
        let expected: [u8; 6] = [0x0, 0x3, 0x0, 0x1, 0x5a, 0xa5];
        assert_eq!(expected, tx_buffer[0..6]);

        // cannot send a second one
        let data = Data::new(2, &my_datagram);
        assert!(data.is_none());
    }

    #[test]
    fn test_full_packet_data() {
        let my_datagram: Vec<u8> = vec![0x5A; MAX_DATA_SIZE];
        let data = Data::new(1, &my_datagram);
        let mut tx_buffer = [0u8; MAX_PACKET_SIZE];
        data.unwrap().serialize(&mut tx_buffer);
        let mut expected: [u8; MAX_DATA_SIZE] = [0x5A; MAX_DATA_SIZE];
        expected[0] = 0x0;
        expected[1] = 0x3;
        expected[2] = 0x0;
        expected[3] = 0x1;
        assert_eq!(expected, tx_buffer[0..MAX_DATA_SIZE]);
    }

    #[test]
    fn test_full_packet_data_and_one() {
        let mut my_datagram: Vec<u8> = vec![0x5A; MAX_DATA_SIZE + 1];
        my_datagram[MAX_DATA_SIZE] = 0xA5;
        let mut tx_buffer = [0u8; MAX_PACKET_SIZE];

        // first datagram
        let data = Data::new(1, &my_datagram);
        data.unwrap().serialize(&mut tx_buffer);
        let mut expected: [u8; MAX_DATA_SIZE] = [0x5A; MAX_DATA_SIZE];
        expected[0] = 0x0;
        expected[1] = 0x3;
        expected[2] = 0x0;
        expected[3] = 0x1;
        assert_eq!(expected, tx_buffer[0..MAX_DATA_SIZE]);

        // second datagram
        let data = Data::new(2, &my_datagram);
        data.unwrap().serialize(&mut tx_buffer);
        let expected: [u8; 5] = [0x0, 0x3, 0x0, 0x2, 0xA5];
        assert_eq!(expected, tx_buffer[0..5]);
    }

    #[test]
    fn test_three_packets() {
        let mut my_datagram: Vec<u8> = vec![0x5A; MAX_DATA_SIZE * 2 + 1];
        my_datagram[MAX_DATA_SIZE * 2] = 0xA5;
        let mut tx_buffer = [0u8; MAX_PACKET_SIZE];

        let data = Data::new(3, &my_datagram);
        data.unwrap().serialize(&mut tx_buffer);
        let expected: [u8; 5] = [0x0, 0x3, 0x0, 0x3, 0xA5];
        assert_eq!(expected, tx_buffer[0..5]);
    }

    #[test]
    fn test_ack() {
        let my_ack = Ack::new(0);
        let mut tx_buffer = [0u8; MAX_PACKET_SIZE];
        my_ack.serialize(&mut tx_buffer);
        let expected: [u8; 4] = [0x0, 0x4, 0x0, 0x0];
        assert_eq!(expected, tx_buffer[0..4]);

        let my_ack = Ack::new(257);
        let mut tx_buffer = [0u8; MAX_PACKET_SIZE];
        my_ack.serialize(&mut tx_buffer);
        let expected: [u8; 4] = [0x0, 0x4, 0x1, 0x1];
        assert_eq!(expected, tx_buffer[0..4]);
    }

    #[test]
    fn test_error() {
        let my_error = ErrorResponse::new(ErrorCode::DiskFull, String::from("WRONG"));
        let mut tx_buffer = [0u8; MAX_PACKET_SIZE];
        my_error.serialize(&mut tx_buffer);
        let expected: [u8; 10] = [0x0, 0x5, 0x0, 0x3, 0x57, 0x52, 0x4F, 0x4E, 0x47, 0x0];
        assert_eq!(expected, tx_buffer[0..10]);
    }
}
