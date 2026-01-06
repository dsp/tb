//! Native Rust client for TigerBeetle.
//!
//! This crate provides a high-performance client for [TigerBeetle](https://tigerbeetle.com),
//! the financial transactions database.
//!
//! # Features
//!
//! - **High-performance**: Uses io_uring for efficient async I/O on Linux
//! - **Type-safe**: Strong typing for accounts, transfers, and results
//! - **Simple API**: One `Client` type with a clean builder pattern
//!
//! # Requirements
//!
//! - Linux (kernel 5.6+) with io_uring support
//!
//! # Quick Start
//!
//! ```ignore
//! use tb_rs::{Client, Account, AccountFlags};
//!
//! // Run inside tokio_uring runtime
//! tokio_uring::start(async {
//!     // Connect to cluster
//!     let mut client = Client::connect(0, "127.0.0.1:3000").await?;
//!
//!     // Create an account
//!     let account = Account {
//!         id: tb_rs::id(),
//!         ledger: 1,
//!         code: 1,
//!         ..Default::default()
//!     };
//!     let errors = client.create_accounts(&[account]).await?;
//!     assert!(errors.is_empty(), "Account creation failed");
//!
//!     // Lookup the account
//!     let accounts = client.lookup_accounts(&[account.id]).await?;
//!     println!("Found {} accounts", accounts.len());
//!
//!     client.close().await;
//!     Ok::<_, tb_rs::ClientError>(())
//! });
//! ```
//!
//! # Configuration
//!
//! Use the builder pattern for custom configuration:
//!
//! ```ignore
//! use std::time::Duration;
//! use tb_rs::Client;
//!
//! let client = Client::builder()
//!     .cluster(0)
//!     .addresses("127.0.0.1:3000,127.0.0.1:3001")?
//!     .connect_timeout(Duration::from_secs(10))
//!     .request_timeout(Duration::from_millis(100))
//!     .build()
//!     .await?;
//! ```

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

// Public modules
mod client;
mod error;
pub mod protocol;

// Internal implementation (not public)
mod internal;

// Re-export main types
pub use client::{Client, ClientBuilder};
pub use error::{ClientError, ProtocolError, Result};

// Re-export protocol types
pub use protocol::{
    Account, AccountBalance, AccountFilter, AccountFilterFlags, AccountFlags, CreateAccountResult,
    CreateAccountsResult, CreateTransferResult, CreateTransfersResult, QueryFilter,
    QueryFilterFlags, Transfer, TransferFlags,
};

/// Generate a unique TigerBeetle ID.
///
/// Creates a globally unique identifier using timestamp and random data,
/// suitable for account or transfer IDs.
///
/// # Example
///
/// ```
/// let account_id = tb_rs::id();
/// let transfer_id = tb_rs::id();
/// assert_ne!(account_id, transfer_id);
/// ```
pub fn id() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let random: u64 = rand::random();

    ((timestamp as u128) << 64) | (random as u128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_uniqueness() {
        let ids: Vec<u128> = (0..1000).map(|_| id()).collect();

        for (i, a) in ids.iter().enumerate() {
            assert_ne!(*a, 0);
            for b in &ids[..i] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn test_id_temporal_ordering() {
        let id1 = id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = id();

        let ts1 = id1 >> 64;
        let ts2 = id2 >> 64;
        assert!(ts2 >= ts1);
    }
}
