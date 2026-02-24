/// Library implementation of the Trivial File Transfer Protocol (RFC 1350)

const MAX_PACKET_SIZE: usize = 512;

///    TFTP supports five types of packets:
///
///           opcode  operation
///             1     Read request (RRQ)
///             2     Write request (WRQ)
///             3     Data (DATA)
///             4     Acknowledgment (ACK)
///             5     Error (ERROR)
///
///    The TFTP header of a packet contains the  opcode  associated  with
///    that packet.
#[repr(u16)]
enum OpCode {
    ReadRequest = 1,
    WriteRequest = 2,
    Data = 3,
    Acknowledgement = 4,
    Error = 5,
}

const TEXT_MODE: &str = "NETASCII";
const BINARY_MODE: &str = "OCTET";
const FIXED_REQUEST_BYTES: usize = 4;

/// The mode field contains the string "netascii", "octet", or "mail"
/// (or any combination of upper and lower case, such as "NETASCII", NetAscii", etc.)
/// in netascii indicating the three modes defined in the protocol.
#[derive(Debug, Copy, Clone)]
enum Mode {
    /// A host which receives netascii mode data must translate the data to its own format.
    Text,
    /// Octet mode is used to transfer a file that is in the 8-bit format of the machine from which the file is being transferred.
    Binary,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
enum RequestType {
    Read = OpCode::ReadRequest as u8,
    Write = OpCode::WriteRequest as u8,
}

#[repr(u8)]
enum ErrorCode {
    Undefined = 0,
    FileNotFound = 1,
    AccessViolation = 2,
    DiskFull = 3,
    IllegalOperation = 4,
    UnknownTransferId = 5,
    FileAlreadyExists = 6,
    NoSuchUser = 7,
}

trait Serial {
    fn serialize(&self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> usize;
}

/// Any transfer begins with a request to read or write a file, which also serves to request a connection.
/// Size of filename must not exceed `match mode { binary => 503, text => 500 }` bytes
/// (512 - 4 fixed - mode string)
#[derive(Debug, Clone)]
struct Request {
    // RRQ and WRQ packets (opcodes 1 and 2 respectively)
    request: RequestType,
    // The file name is a sequence of bytes in netascii.
    filename: String,
    // The mode field contains the string "netascii", "octet", or "mail" (or any combination of upper
    //    and lower case, such as "NETASCII", NetAscii", etc.) in netascii indicating the three modes defined in the protocol.
    mode: Mode,
}

impl Request {
    fn new(request: RequestType, filename: String, mode: Mode) -> Result<Self, TftprsError> {
        let mode_size = match mode {
            Mode::Text => TEXT_MODE.len(),
            Mode::Binary => BINARY_MODE.len(),
        };
        let max_filename_size = MAX_PACKET_SIZE - FIXED_REQUEST_BYTES - mode_size;
        if filename.len() > max_filename_size {
            return Err(TftprsError::BadRequest);
        }
        Ok(Self {
            request,
            filename,
            mode,
        })
    }
}

enum TftprsError {
    BadRequest,
}

impl Serial for Request {
    fn serialize(&self, buffer: &mut [u8; MAX_PACKET_SIZE]) -> usize {
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

struct Data {
    block: u16,
    data: Vec<u8>,
}

struct Acknowledgement {
    block: u16,
}

/// Most errors cause termination of the connection.
/// An error is signalled by sending an error packet.
struct Error {
    // The error code is an integer indicating the nature of the error.
    code: ErrorCode,
    // The error message is intended for human consumption.
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
