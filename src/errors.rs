use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum TftprsError {
    #[error("Request is badly formed")]
    /// The user attempted to form a bad request for the protocol.
    BadRequestAttempted,
    #[error("Packet received failed to parse")]
    /// A packet arrived that the machine cannot parse.
    BadPacketReceived,
    #[error("Connection or transaction is already active")]
    /// Machine is busy on an active connection, and the user attempted to start a new connection.
    Busy,
    #[error("No connection")]
    /// The machine is not currently on an active connection, and the user attempted to process a message as if it were.
    NoConnection,
    #[error("No file")]
    /// The user provided a bad file.
    NoFile,
    #[error("Error {0} received: {1}")]
    /// An error was parsed from the remote peer.
    ErrorResponse(u16, String),
}
