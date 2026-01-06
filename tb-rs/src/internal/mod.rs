//! Internal implementation details.
//!
//! This module contains the io_uring driver, connection handling, and buffer
//! management. These are implementation details and not part of the public API.

pub(crate) mod buffer;
pub(crate) mod connection;
pub(crate) mod driver;

pub(crate) use buffer::{BufferPool, OwnedBuf};
pub(crate) use driver::Driver;
