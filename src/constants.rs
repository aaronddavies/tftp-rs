/// Constants

pub(crate) const MAX_PACKET_SIZE: usize = 512;

/// TFTP supports five types of packets. The TFTP header of a packet contains the opcode associated with that packet.
#[repr(u16)]
pub(crate) enum OpCode {
    ReadRequest = 1,
    WriteRequest = 2,
    Data = 3,
    Acknowledgement = 4,
    Error = 5,
}

pub(crate) const TEXT_MODE: &str = "NETASCII";
pub(crate) const BINARY_MODE: &str = "OCTET";
pub(crate) const FIXED_REQUEST_BYTES: usize = 4;

pub(crate) const DEFAULT_DESTINATION_TID: u16 = 69;

/// The mode field contains the string "netascii", "octet", or "mail"
/// (or any combination of upper and lower case, such as "NETASCII", NetAscii", etc.)
/// in netascii indicating the three modes defined in the protocol.
#[derive(Debug, Copy, Clone)]
pub(crate) enum Mode {
    /// A host which receives netascii mode data must translate the data to its own format.
    Text,
    /// Octet mode is used to transfer a file that is in the 8-bit format of the machine from which the file is being transferred.
    Binary,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub(crate) enum RequestType {
    Read = OpCode::ReadRequest as u8,
    Write = OpCode::WriteRequest as u8,
}

#[derive(Debug, Copy, Clone)]
#[repr(u16)]
pub(crate) enum ErrorCode {
    Undefined = 0,
    FileNotFound = 1,
    AccessViolation = 2,
    DiskFull = 3,
    IllegalOperation = 4,
    UnknownTransferId = 5,
    FileAlreadyExists = 6,
    NoSuchUser = 7,
}