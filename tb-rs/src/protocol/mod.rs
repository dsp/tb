//! TigerBeetle protocol implementation.
//!
//! This module contains the wire format types and serialization logic
//! for communicating with TigerBeetle servers.

pub mod checksum;
pub mod header;
pub mod message;
pub mod multi_batch;
pub mod operation;
pub mod types;

// Re-export commonly used items
pub use checksum::checksum;
pub use header::{
    EvictionHeader, EvictionReason, Header, HeaderError, PingClientHeader, PongClientHeader,
    ReplyHeader, RequestHeader, HEADER_SIZE, PROTOCOL_VERSION,
};
pub use message::{Message, MessageError, RequestBuilder, MESSAGE_BODY_SIZE_MAX, MESSAGE_SIZE_MAX};
pub use operation::{Command, Operation, VSR_OPERATIONS_RESERVED};
pub use types::{
    Account, AccountBalance, AccountFilter, AccountFilterFlags, AccountFlags, CreateAccountResult,
    CreateAccountsResult, CreateTransferResult, CreateTransfersResult, QueryFilter,
    QueryFilterFlags, RegisterRequest, RegisterResult, Transfer, TransferFlags,
};
