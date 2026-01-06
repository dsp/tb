//! Error types for the TigerBeetle client.
//!
//! All error types implement `std::error::Error` for compatibility
//! with error handling frameworks like `anyhow` and `thiserror`.

use crate::protocol::header::EvictionReason;
use std::error::Error;
use std::fmt;

/// Result type for client operations.
pub type Result<T> = std::result::Result<T, ClientError>;

/// Main error type for client operations.
#[derive(Debug)]
pub enum ClientError {
    /// Connection error (connect, send, recv failures).
    Connection(String),
    /// Protocol error (invalid message, checksum failure, etc.).
    Protocol(ProtocolError),
    /// Client was evicted by the server.
    Evicted(EvictionReason),
    /// Operation timed out.
    Timeout,
    /// Client is not registered.
    NotRegistered,
    /// Client is shutting down.
    Shutdown,
    /// Request was too large for the server's batch size limit.
    RequestTooLarge {
        /// The size of the request body in bytes.
        size: u32,
        /// The server's batch size limit in bytes.
        limit: u32,
    },
    /// Invalid operation for current state.
    InvalidOperation,
    /// Transport-level error (I/O, network, etc.).
    /// Deprecated: Use Connection instead.
    Transport(Box<dyn Error + Send + Sync>),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::Connection(msg) => write!(f, "connection error: {}", msg),
            ClientError::Protocol(e) => write!(f, "protocol error: {}", e),
            ClientError::Evicted(reason) => write!(f, "client evicted: {:?}", reason),
            ClientError::Timeout => write!(f, "operation timed out"),
            ClientError::NotRegistered => write!(f, "client not registered"),
            ClientError::Shutdown => write!(f, "client is shutting down"),
            ClientError::RequestTooLarge { size, limit } => {
                write!(f, "request too large: {} bytes exceeds limit of {} bytes", size, limit)
            }
            ClientError::InvalidOperation => write!(f, "invalid operation for current state"),
            ClientError::Transport(e) => write!(f, "transport error: {}", e),
        }
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ClientError::Transport(e) => Some(e.as_ref()),
            ClientError::Protocol(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ProtocolError> for ClientError {
    fn from(err: ProtocolError) -> Self {
        ClientError::Protocol(err)
    }
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        ClientError::Transport(Box::new(err))
    }
}

/// Protocol-level errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    /// Invalid header checksum.
    InvalidHeaderChecksum,
    /// Invalid body checksum.
    InvalidBodyChecksum,
    /// Invalid header structure.
    InvalidHeader,
    /// Invalid operation.
    InvalidOperation,
    /// Unexpected reply (wrong request number or parent).
    UnexpectedReply,
    /// Version mismatch.
    VersionMismatch,
    /// Invalid message size.
    InvalidSize,
    /// Invalid command.
    InvalidCommand,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::InvalidHeaderChecksum => write!(f, "invalid header checksum"),
            ProtocolError::InvalidBodyChecksum => write!(f, "invalid body checksum"),
            ProtocolError::InvalidHeader => write!(f, "invalid header structure"),
            ProtocolError::InvalidOperation => write!(f, "invalid operation"),
            ProtocolError::UnexpectedReply => write!(f, "unexpected reply"),
            ProtocolError::VersionMismatch => write!(f, "version mismatch"),
            ProtocolError::InvalidSize => write!(f, "invalid message size"),
            ProtocolError::InvalidCommand => write!(f, "invalid command"),
        }
    }
}

impl Error for ProtocolError {}

/// Packet-level status codes (from C client API).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PacketStatus {
    /// Operation completed successfully.
    Ok,
    /// Request data was too large.
    TooMuchData,
    /// Client was evicted.
    ClientEvicted,
    /// Client release is too old.
    ClientReleaseTooLow,
    /// Client release is too new.
    ClientReleaseTooHigh,
    /// Client was shut down.
    ClientShutdown,
    /// Invalid operation.
    InvalidOperation,
    /// Invalid data size.
    InvalidDataSize,
}

impl fmt::Display for PacketStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PacketStatus::Ok => write!(f, "ok"),
            PacketStatus::TooMuchData => write!(f, "too much data"),
            PacketStatus::ClientEvicted => write!(f, "client evicted"),
            PacketStatus::ClientReleaseTooLow => write!(f, "client release too low"),
            PacketStatus::ClientReleaseTooHigh => write!(f, "client release too high"),
            PacketStatus::ClientShutdown => write!(f, "client shutdown"),
            PacketStatus::InvalidOperation => write!(f, "invalid operation"),
            PacketStatus::InvalidDataSize => write!(f, "invalid data size"),
        }
    }
}

impl Error for PacketStatus {}

/// Initialization status codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitStatus {
    /// Initialization succeeded.
    Success,
    /// Unexpected error.
    Unexpected,
    /// Out of memory.
    OutOfMemory,
    /// Invalid address.
    AddressInvalid,
    /// Too many addresses.
    AddressLimitExceeded,
    /// System resource error.
    SystemResources,
    /// Network subsystem error.
    NetworkSubsystem,
}

impl fmt::Display for InitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InitStatus::Success => write!(f, "success"),
            InitStatus::Unexpected => write!(f, "unexpected error"),
            InitStatus::OutOfMemory => write!(f, "out of memory"),
            InitStatus::AddressInvalid => write!(f, "invalid address"),
            InitStatus::AddressLimitExceeded => write!(f, "address limit exceeded"),
            InitStatus::SystemResources => write!(f, "system resources error"),
            InitStatus::NetworkSubsystem => write!(f, "network subsystem error"),
        }
    }
}

impl Error for InitStatus {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_error_display() {
        let err = ClientError::Timeout;
        assert_eq!(format!("{}", err), "operation timed out");
    }

    #[test]
    fn test_protocol_error_display() {
        let err = ProtocolError::InvalidHeaderChecksum;
        assert_eq!(format!("{}", err), "invalid header checksum");
    }

    #[test]
    fn test_client_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let client_err: ClientError = io_err.into();
        assert!(matches!(client_err, ClientError::Transport(_)));
    }

    #[test]
    fn test_error_source_chain() {
        let protocol_err = ProtocolError::InvalidHeaderChecksum;
        let client_err = ClientError::Protocol(protocol_err);

        // Can get source
        let source = client_err.source().unwrap();
        assert!(source.is::<ProtocolError>());
    }
}
