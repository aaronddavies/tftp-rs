use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum TftprsError {
    #[error("Request is badly formed")]
    BadRequestAttempted,
    #[error("Packet received failed to parse")]
    BadPacketReceived,
    #[error("Connection or transaction is already active")]
    Busy,
    #[error("No connection")]
    NoConnection,
    #[error("No file")]
    NoFile,
    #[error("Error {0} received: {1}")]
    ErrorResponse(u16, String),
}
