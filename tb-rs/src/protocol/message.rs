//! TigerBeetle message serialization.
//!
//! Messages consist of a fixed 256-byte header followed by a variable-length body.

use super::header::{Header, HEADER_SIZE};
use super::operation::{Command, Operation};

/// Maximum message size (1 MiB).
pub const MESSAGE_SIZE_MAX: u32 = 1024 * 1024;

/// Maximum body size.
pub const MESSAGE_BODY_SIZE_MAX: u32 = MESSAGE_SIZE_MAX - HEADER_SIZE;

/// A complete TigerBeetle message with header and body.
#[derive(Clone, Debug)]
pub struct Message {
    /// The message data (header + body).
    data: Vec<u8>,
}

impl Message {
    /// Create a new message with just a header.
    pub fn new() -> Self {
        let mut data = vec![0u8; HEADER_SIZE as usize];
        // Initialize with default header
        let header = Header::default();
        data[..HEADER_SIZE as usize].copy_from_slice(header.as_bytes());
        Self { data }
    }

    /// Create a new message with capacity for a body.
    pub fn with_body_capacity(capacity: u32) -> Self {
        let mut msg = Self::new();
        msg.data.reserve(capacity as usize);
        msg
    }

    /// Create a message from raw bytes.
    ///
    /// Returns None if the bytes are too short.
    pub fn from_bytes(bytes: Vec<u8>) -> Option<Self> {
        if (bytes.len() as u32) < HEADER_SIZE {
            return None;
        }
        Some(Self { data: bytes })
    }

    /// Get the header.
    pub fn header(&self) -> &Header {
        Header::from_bytes(self.data[..HEADER_SIZE as usize].try_into().unwrap())
    }

    /// Get the header mutably.
    pub fn header_mut(&mut self) -> &mut Header {
        Header::from_bytes_mut((&mut self.data[..HEADER_SIZE as usize]).try_into().unwrap())
    }

    /// Get the body.
    pub fn body(&self) -> &[u8] {
        &self.data[HEADER_SIZE as usize..]
    }

    /// Get the body mutably.
    pub fn body_mut(&mut self) -> &mut [u8] {
        &mut self.data[HEADER_SIZE as usize..]
    }

    /// Set the message body.
    pub fn set_body(&mut self, body: &[u8]) {
        self.data.truncate(HEADER_SIZE as usize);
        self.data.extend_from_slice(body);
        self.header_mut().size = self.data.len() as u32;
    }

    /// Append data to the body.
    pub fn append_body(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
        self.header_mut().size = self.data.len() as u32;
    }

    /// Get the entire message as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get the entire message as mutable bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Consume the message and return the underlying bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Get the total message size (header + body).
    pub fn len(&self) -> u32 {
        self.data.len() as u32
    }

    /// Check if the message has an empty body.
    pub fn is_empty(&self) -> bool {
        self.data.len() as u32 == HEADER_SIZE
    }

    /// Finalize the message by computing checksums.
    ///
    /// Must be called before sending the message.
    pub fn finalize(&mut self) {
        // Compute body checksum first
        let body_checksum = crate::protocol::checksum::checksum(&self.data[HEADER_SIZE as usize..]);
        self.header_mut().checksum_body = body_checksum;
        // Then compute header checksum
        self.header_mut().set_checksum();
    }

    /// Validate the message checksums.
    pub fn validate(&self) -> Result<(), MessageError> {
        if !self.header().valid_checksum() {
            return Err(MessageError::InvalidHeaderChecksum);
        }
        if !self.header().valid_checksum_body(self.body()) {
            return Err(MessageError::InvalidBodyChecksum);
        }
        Ok(())
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

/// Message validation errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageError {
    /// Invalid header checksum.
    InvalidHeaderChecksum,
    /// Invalid body checksum.
    InvalidBodyChecksum,
    /// Message is too small.
    TooSmall,
    /// Message is too large.
    TooLarge,
    /// Invalid command.
    InvalidCommand,
    /// Invalid operation.
    InvalidOperation,
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageError::InvalidHeaderChecksum => write!(f, "invalid header checksum"),
            MessageError::InvalidBodyChecksum => write!(f, "invalid body checksum"),
            MessageError::TooSmall => write!(f, "message too small"),
            MessageError::TooLarge => write!(f, "message too large"),
            MessageError::InvalidCommand => write!(f, "invalid command"),
            MessageError::InvalidOperation => write!(f, "invalid operation"),
        }
    }
}

impl std::error::Error for MessageError {}

/// Builder for constructing request messages.
pub struct RequestBuilder {
    message: Message,
}

impl RequestBuilder {
    /// Create a new request builder.
    pub fn new(cluster: u128, client: u128) -> Self {
        let mut message = Message::new();
        {
            let header = message.header_mut();
            header.cluster = cluster;
            header.set_command(Command::Request);

            let req = header.as_request_mut();
            req.client = client;
        }
        Self { message }
    }

    /// Set the session number.
    pub fn session(mut self, session: u64) -> Self {
        self.message.header_mut().as_request_mut().session = session;
        self
    }

    /// Set the request number.
    pub fn request(mut self, request: u32) -> Self {
        self.message.header_mut().as_request_mut().request = request;
        self
    }

    /// Set the parent checksum.
    pub fn parent(mut self, parent: u128) -> Self {
        self.message.header_mut().as_request_mut().parent = parent;
        self
    }

    /// Set the operation.
    pub fn operation(mut self, operation: Operation) -> Self {
        self.message
            .header_mut()
            .as_request_mut()
            .set_operation(operation);
        self
    }

    /// Set the view.
    pub fn view(mut self, view: u32) -> Self {
        self.message.header_mut().view = view;
        self
    }

    /// Set the release version.
    pub fn release(mut self, release: u32) -> Self {
        self.message.header_mut().release = release;
        self
    }

    /// Set the body data.
    pub fn body(mut self, body: &[u8]) -> Self {
        self.message.set_body(body);
        self
    }

    /// Build and finalize the message.
    pub fn build(mut self) -> Message {
        self.message.finalize();
        self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_new() {
        let msg = Message::new();
        assert_eq!(msg.len(), HEADER_SIZE);
        assert!(msg.body().is_empty());
    }

    #[test]
    fn test_message_set_body() {
        let mut msg = Message::new();
        msg.set_body(b"hello");
        assert_eq!(msg.body(), b"hello");
        assert_eq!(msg.header().size, HEADER_SIZE + 5);
    }

    #[test]
    fn test_message_finalize_and_validate() {
        let mut msg = Message::new();
        msg.header_mut().cluster = 12345;
        msg.set_body(b"test data");
        msg.finalize();

        assert!(msg.validate().is_ok());
    }

    #[test]
    fn test_message_validation_fails_on_corruption() {
        let mut msg = Message::new();
        msg.header_mut().cluster = 12345;
        msg.set_body(b"test data");
        msg.finalize();

        // Corrupt the body
        msg.as_bytes_mut()[HEADER_SIZE as usize] ^= 0xFF;
        assert_eq!(msg.validate(), Err(MessageError::InvalidBodyChecksum));
    }

    #[test]
    fn test_request_builder() {
        let msg = RequestBuilder::new(0xDEAD, 0xBEEF)
            .session(42)
            .request(1)
            .parent(0)
            .operation(Operation::CreateAccounts)
            .body(&[1, 2, 3, 4])
            .build();

        assert_eq!(msg.header().cluster, 0xDEAD);
        assert_eq!(msg.header().as_request().client, 0xBEEF);
        assert_eq!(msg.header().as_request().session, 42);
        assert_eq!(msg.header().as_request().request, 1);
        assert_eq!(
            msg.header().as_request().operation,
            Operation::CreateAccounts as u8
        );
        assert_eq!(msg.body(), &[1, 2, 3, 4]);
        assert!(msg.validate().is_ok());
    }
}
