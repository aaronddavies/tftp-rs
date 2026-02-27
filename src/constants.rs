//! Constants

use crate::errors::TftprsError;

pub const MAX_PACKET_SIZE: usize = 512;

/// TFTP supports five types of packets. The TFTP header of a packet contains the opcode associated with that packet.
#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum OpCode {
    ReadRequest = 1,
    WriteRequest = 2,
    Data = 3,
    Acknowledgement = 4,
    Error = 5,
}

impl TryFrom<u16> for OpCode {
    type Error = TftprsError;

    fn try_from(value: u16) -> Result<Self, TftprsError> {
        match value {
            1 => Ok(OpCode::ReadRequest),
            2 => Ok(OpCode::WriteRequest),
            3 => Ok(OpCode::Data),
            4 => Ok(OpCode::Acknowledgement),
            5 => Ok(OpCode::Error),
            _ => Err(TftprsError::BadPacketReceived),
        }
    }
}

pub(crate) const TEXT_MODE: &str = "NETASCII";
pub(crate) const BINARY_MODE: &str = "OCTET";
pub(crate) const FIXED_REQUEST_BYTES: usize = 4;
pub(crate) const FIXED_DATA_BYTES: usize = 4;
pub(crate) const MAX_DATA_SIZE: usize = MAX_PACKET_SIZE - FIXED_DATA_BYTES;

/// The mode field contains the string "netascii", "octet", or "mail"
/// (or any combination of upper and lower case, such as "NETASCII", NetAscii", etc.)
/// in netascii indicating the three modes defined in the protocol.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum Mode {
    /// A host which receives netascii mode data must translate the data to its own format.
    Text,
    /// Octet mode is used to transfer a file that is in the 8-bit format of the machine from which the file is being transferred.
    #[default]
    Binary,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum RequestType {
    Read = OpCode::ReadRequest as u8,
    Write = OpCode::WriteRequest as u8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u16)]
pub enum ErrorCode {
    Undefined = 0,
    FileNotFound = 1,
    AccessViolation = 2,
    DiskFull = 3,
    IllegalOperation = 4,
    UnknownTransferId = 5,
    FileAlreadyExists = 6,
    NoSuchUser = 7,
}

impl TryFrom<u16> for ErrorCode {
    type Error = TftprsError;

    fn try_from(value: u16) -> Result<Self, TftprsError> {
        match value {
            1 => Ok(ErrorCode::FileNotFound),
            2 => Ok(ErrorCode::AccessViolation),
            3 => Ok(ErrorCode::DiskFull),
            4 => Ok(ErrorCode::IllegalOperation),
            5 => Ok(ErrorCode::UnknownTransferId),
            6 => Ok(ErrorCode::FileAlreadyExists),
            7 => Ok(ErrorCode::NoSuchUser),
            _ => Ok(ErrorCode::Undefined),
        }
    }
}

