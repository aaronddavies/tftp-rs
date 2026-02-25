use thiserror::Error;

#[derive(Debug, Error)]
pub enum TftprsError {
    #[error("Request is badly formed")]
    BadRequestAttempted,
    #[error("Packet received failed to parse")]
    BadPacketReceived,
    #[error("Connection or transaction is already active")]
    Busy,
    #[error("Connection terminated")]
    Terminated,
    #[error("No file")]
    NoFile,
}
