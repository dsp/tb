//! TigerBeetle message header (256 bytes).
//!
//! The header is the fixed-size prefix of all TigerBeetle network messages.
//! It contains checksums, routing information, and command-specific fields.

use super::checksum;
use super::operation::{Command, Operation};

/// Protocol version (must match server).
pub const PROTOCOL_VERSION: u16 = 0;

/// Size of the message header in bytes.
pub const HEADER_SIZE: u32 = 256;

/// Header size as usize for array indexing (Rust requires usize for array sizes).
const HEADER_SIZE_USIZE: usize = HEADER_SIZE as usize;

/// TigerBeetle wire protocol header (256 bytes, little-endian).
///
/// This struct matches the exact byte layout of the TigerBeetle protocol header.
/// All padding fields must be zero.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Header {
    /// Checksum covering bytes 16-255 of this header.
    pub checksum: u128,
    /// Padding for future u256 support (must be zero).
    pub checksum_padding: u128,
    /// Checksum covering the message body after this header.
    pub checksum_body: u128,
    /// Padding for future u256 support (must be zero).
    pub checksum_body_padding: u128,
    /// Reserved for future AEAD nonce (must be zero).
    pub nonce_reserved: u128,
    /// Cluster identifier.
    pub cluster: u128,
    /// Total message size (header + body).
    pub size: u32,
    /// Cluster reconfiguration epoch (must be zero).
    pub epoch: u32,
    /// Current view number.
    pub view: u32,
    /// Release version (encoded as major.minor.patch).
    pub release: u32,
    /// Protocol version.
    pub protocol: u16,
    /// VSR command type.
    pub command: u8,
    /// Replica index that authored this message (0 for clients).
    pub replica: u8,
    /// Reserved for header frame (must be zero).
    pub reserved_frame: [u8; 12],
    /// Command-specific data (128 bytes).
    pub reserved_command: [u8; 128],
}

const _: () = assert!(std::mem::size_of::<Header>() == HEADER_SIZE as usize);

impl Default for Header {
    fn default() -> Self {
        Self {
            checksum: 0,
            checksum_padding: 0,
            checksum_body: 0,
            checksum_body_padding: 0,
            nonce_reserved: 0,
            cluster: 0,
            size: HEADER_SIZE,
            epoch: 0,
            view: 0,
            release: 0,
            protocol: PROTOCOL_VERSION,
            command: Command::Reserved as u8,
            replica: 0,
            reserved_frame: [0; 12],
            reserved_command: [0; 128],
        }
    }
}

impl Header {
    /// Create a new header with the given cluster ID.
    pub fn new(cluster: u128) -> Self {
        Self {
            cluster,
            ..Default::default()
        }
    }

    /// Get the command type.
    pub fn command(&self) -> Option<Command> {
        Command::try_from(self.command).ok()
    }

    /// Set the command type.
    pub fn set_command(&mut self, command: Command) {
        self.command = command as u8;
    }

    /// Get this header as a Request header view.
    pub fn as_request(&self) -> &RequestHeader {
        unsafe { &*(self.reserved_command.as_ptr() as *const RequestHeader) }
    }

    /// Get this header as a mutable Request header view.
    pub fn as_request_mut(&mut self) -> &mut RequestHeader {
        unsafe { &mut *(self.reserved_command.as_mut_ptr() as *mut RequestHeader) }
    }

    /// Get this header as a Reply header view.
    pub fn as_reply(&self) -> &ReplyHeader {
        unsafe { &*(self.reserved_command.as_ptr() as *const ReplyHeader) }
    }

    /// Get this header as a mutable Reply header view.
    pub fn as_reply_mut(&mut self) -> &mut ReplyHeader {
        unsafe { &mut *(self.reserved_command.as_mut_ptr() as *mut ReplyHeader) }
    }

    /// Get this header as a PingClient header view.
    pub fn as_ping_client(&self) -> &PingClientHeader {
        unsafe { &*(self.reserved_command.as_ptr() as *const PingClientHeader) }
    }

    /// Get this header as a mutable PingClient header view.
    pub fn as_ping_client_mut(&mut self) -> &mut PingClientHeader {
        unsafe { &mut *(self.reserved_command.as_mut_ptr() as *mut PingClientHeader) }
    }

    /// Get this header as a PongClient header view.
    pub fn as_pong_client(&self) -> &PongClientHeader {
        unsafe { &*(self.reserved_command.as_ptr() as *const PongClientHeader) }
    }

    /// Get this header as an Eviction header view.
    pub fn as_eviction(&self) -> &EvictionHeader {
        unsafe { &*(self.reserved_command.as_ptr() as *const EvictionHeader) }
    }

    /// Calculate the header checksum (covers bytes 16-255).
    pub fn calculate_checksum(&self) -> u128 {
        let bytes = self.as_bytes();
        // Checksum covers bytes starting after the checksum field (offset 16)
        checksum::checksum(&bytes[16..])
    }

    /// Calculate the body checksum.
    pub fn calculate_checksum_body(&self, body: &[u8]) -> u128 {
        checksum::checksum(body)
    }

    /// Set the header checksum (must be called after set_checksum_body).
    pub fn set_checksum(&mut self) {
        self.checksum = self.calculate_checksum();
    }

    /// Set the body checksum.
    pub fn set_checksum_body(&mut self, body: &[u8]) {
        self.checksum_body = self.calculate_checksum_body(body);
    }

    /// Verify the header checksum is valid.
    pub fn valid_checksum(&self) -> bool {
        self.checksum == self.calculate_checksum()
    }

    /// Verify the body checksum is valid.
    pub fn valid_checksum_body(&self, body: &[u8]) -> bool {
        self.checksum_body == self.calculate_checksum_body(body)
    }

    /// Get the header as a byte slice.
    pub fn as_bytes(&self) -> &[u8; HEADER_SIZE_USIZE] {
        unsafe { &*(self as *const Header as *const [u8; HEADER_SIZE_USIZE]) }
    }

    /// Get the header as a mutable byte slice.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; HEADER_SIZE_USIZE] {
        unsafe { &mut *(self as *mut Header as *mut [u8; HEADER_SIZE_USIZE]) }
    }

    /// Create a header from a byte slice.
    ///
    /// # Safety
    /// The slice must be exactly 256 bytes.
    pub fn from_bytes(bytes: &[u8; HEADER_SIZE_USIZE]) -> &Header {
        unsafe { &*(bytes.as_ptr() as *const Header) }
    }

    /// Create a mutable header from a byte slice.
    ///
    /// # Safety
    /// The slice must be exactly 256 bytes.
    pub fn from_bytes_mut(bytes: &mut [u8; HEADER_SIZE_USIZE]) -> &mut Header {
        unsafe { &mut *(bytes.as_mut_ptr() as *mut Header) }
    }

    /// Validate the header structure.
    pub fn validate(&self) -> Result<(), HeaderError> {
        if self.checksum_padding != 0 {
            return Err(HeaderError::InvalidPadding("checksum_padding"));
        }
        if self.checksum_body_padding != 0 {
            return Err(HeaderError::InvalidPadding("checksum_body_padding"));
        }
        if self.nonce_reserved != 0 {
            return Err(HeaderError::InvalidPadding("nonce_reserved"));
        }
        if self.epoch != 0 {
            return Err(HeaderError::InvalidEpoch);
        }
        if self.size < HEADER_SIZE {
            return Err(HeaderError::SizeTooSmall);
        }
        if self.protocol != PROTOCOL_VERSION {
            return Err(HeaderError::ProtocolMismatch);
        }
        if self.reserved_frame != [0; 12] {
            return Err(HeaderError::InvalidPadding("reserved_frame"));
        }
        Ok(())
    }
}

/// Request-specific header fields (overlay on reserved_command).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RequestHeader {
    /// Parent checksum for hash-chain verification.
    pub parent: u128,
    /// Padding for u256 parent (must be zero).
    pub parent_padding: u128,
    /// Client identifier.
    pub client: u128,
    /// Session number (0 for register, non-zero otherwise).
    pub session: u64,
    /// Timestamp (0 normally, used in AOF recovery).
    pub timestamp: u64,
    /// Request number (monotonically increasing).
    pub request: u32,
    /// State machine operation.
    pub operation: u8,
    /// Padding for previous_request_latency.
    pub previous_request_latency_padding: [u8; 3],
    /// Latency of previous request in nanoseconds.
    pub previous_request_latency: u32,
    /// Reserved (must be zero).
    pub reserved: [u8; 52],
}

impl Default for RequestHeader {
    fn default() -> Self {
        Self {
            parent: 0,
            parent_padding: 0,
            client: 0,
            session: 0,
            timestamp: 0,
            request: 0,
            operation: 0,
            previous_request_latency_padding: [0; 3],
            previous_request_latency: 0,
            reserved: [0; 52],
        }
    }
}

const _: () = assert!(std::mem::size_of::<RequestHeader>() == 128);

impl RequestHeader {
    /// Get the operation.
    pub fn operation(&self) -> Option<Operation> {
        Operation::try_from(self.operation).ok()
    }

    /// Set the operation.
    pub fn set_operation(&mut self, operation: Operation) {
        self.operation = operation as u8;
    }
}

/// Reply-specific header fields (overlay on reserved_command).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ReplyHeader {
    /// Checksum of the corresponding request.
    pub request_checksum: u128,
    /// Padding for u256 (must be zero).
    pub request_checksum_padding: u128,
    /// Context checksum for next request's parent.
    pub context: u128,
    /// Padding for u256 (must be zero).
    pub context_padding: u128,
    /// Client identifier.
    pub client: u128,
    /// Operation number.
    pub op: u64,
    /// Commit number (session number for register reply).
    pub commit: u64,
    /// Prepare timestamp.
    pub timestamp: u64,
    /// Request number.
    pub request: u32,
    /// Operation type.
    pub operation: u8,
    /// Reserved (must be zero).
    pub reserved: [u8; 19],
}

const _: () = assert!(std::mem::size_of::<ReplyHeader>() == 128);

impl ReplyHeader {
    /// Get the operation.
    pub fn operation(&self) -> Option<Operation> {
        Operation::try_from(self.operation).ok()
    }
}

/// PingClient-specific header fields.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PingClientHeader {
    /// Client identifier.
    pub client: u128,
    /// Monotonic timestamp for RTT measurement.
    pub ping_timestamp_monotonic: u64,
    /// Reserved.
    pub reserved: [u8; 104],
}

impl Default for PingClientHeader {
    fn default() -> Self {
        Self {
            client: 0,
            ping_timestamp_monotonic: 0,
            reserved: [0; 104],
        }
    }
}

const _: () = assert!(std::mem::size_of::<PingClientHeader>() == 128);

/// PongClient-specific header fields.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PongClientHeader {
    /// Echoed ping timestamp.
    pub ping_timestamp_monotonic: u64,
    /// Server wall clock timestamp.
    pub pong_timestamp_wall: u64,
    /// Reserved.
    pub reserved: [u8; 112],
}

impl Default for PongClientHeader {
    fn default() -> Self {
        Self {
            ping_timestamp_monotonic: 0,
            pong_timestamp_wall: 0,
            reserved: [0; 112],
        }
    }
}

const _: () = assert!(std::mem::size_of::<PongClientHeader>() == 128);

/// Eviction-specific header fields.
/// Layout: client (16 bytes) + reserved (111 bytes) + reason (1 byte) = 128 bytes
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct EvictionHeader {
    /// Client identifier.
    pub client: u128,
    /// Reserved.
    pub reserved: [u8; 111],
    /// Reason for eviction.
    pub reason: u8,
}

impl Default for EvictionHeader {
    fn default() -> Self {
        Self {
            client: 0,
            reserved: [0; 111],
            reason: 0,
        }
    }
}

const _: () = assert!(std::mem::size_of::<EvictionHeader>() == 128);

/// Eviction reason codes.
/// Note: These start at 1, not 0, matching the TigerBeetle Zig enum.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvictionReason {
    /// No active session.
    NoSession = 1,
    /// Client release is too old.
    ClientReleaseTooLow = 2,
    /// Client release is too new.
    ClientReleaseTooHigh = 3,
    /// Invalid request operation.
    InvalidRequestOperation = 4,
    /// Invalid request body.
    InvalidRequestBody = 5,
    /// Invalid request body size.
    InvalidRequestBodySize = 6,
    /// Session number too low.
    SessionTooLow = 7,
    /// Session release mismatch.
    SessionReleaseMismatch = 8,
}

impl TryFrom<u8> for EvictionReason {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(EvictionReason::NoSession),
            2 => Ok(EvictionReason::ClientReleaseTooLow),
            3 => Ok(EvictionReason::ClientReleaseTooHigh),
            4 => Ok(EvictionReason::InvalidRequestOperation),
            5 => Ok(EvictionReason::InvalidRequestBody),
            6 => Ok(EvictionReason::InvalidRequestBodySize),
            7 => Ok(EvictionReason::SessionTooLow),
            8 => Ok(EvictionReason::SessionReleaseMismatch),
            _ => Err(value),
        }
    }
}

/// Header validation errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeaderError {
    /// Invalid padding field (should be zero).
    InvalidPadding(&'static str),
    /// Invalid epoch (should be zero).
    InvalidEpoch,
    /// Size is smaller than header size.
    SizeTooSmall,
    /// Protocol version mismatch.
    ProtocolMismatch,
    /// Invalid checksum.
    InvalidChecksum,
    /// Invalid body checksum.
    InvalidBodyChecksum,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(std::mem::size_of::<Header>(), 256);
    }

    #[test]
    fn test_request_header_size() {
        assert_eq!(std::mem::size_of::<RequestHeader>(), 128);
    }

    #[test]
    fn test_reply_header_size() {
        assert_eq!(std::mem::size_of::<ReplyHeader>(), 128);
    }

    #[test]
    fn test_header_default() {
        let header = Header::default();
        assert_eq!(header.size, 256);
        assert_eq!(header.protocol, PROTOCOL_VERSION);
        assert_eq!(header.command, Command::Reserved as u8);
    }

    #[test]
    fn test_header_checksum() {
        let mut header = Header::default();
        header.cluster = 12345;
        header.set_checksum_body(&[]);
        header.set_checksum();

        assert!(header.valid_checksum());
        assert!(header.valid_checksum_body(&[]));
    }

    #[test]
    fn test_header_checksum_invalid() {
        let mut header = Header::default();
        header.cluster = 12345;
        header.set_checksum_body(&[]);
        header.set_checksum();

        // Corrupt the header
        header.cluster = 99999;
        assert!(!header.valid_checksum());
    }

    #[test]
    fn test_header_as_request() {
        let mut header = Header::default();
        header.set_command(Command::Request);

        let req = header.as_request_mut();
        req.client = 42;
        req.set_operation(Operation::CreateAccounts);
        req.request = 1;

        assert_eq!(header.as_request().client, 42);
        assert_eq!(
            header.as_request().operation,
            Operation::CreateAccounts as u8
        );
        assert_eq!(header.as_request().request, 1);
    }

    #[test]
    fn test_header_validation() {
        let header = Header::default();
        assert!(header.validate().is_ok());

        let mut invalid = Header::default();
        invalid.epoch = 1;
        assert_eq!(invalid.validate(), Err(HeaderError::InvalidEpoch));
    }

    #[test]
    fn test_header_bytes_roundtrip() {
        let mut header = Header::new(0xDEADBEEF);
        header.set_command(Command::Request);
        header.size = 512;

        let bytes = header.as_bytes();
        let restored = Header::from_bytes(bytes);

        assert_eq!(restored.cluster, 0xDEADBEEF);
        assert_eq!(restored.command, Command::Request as u8);
        assert_eq!(restored.size, 512);
    }
}
