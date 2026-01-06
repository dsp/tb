# tb-rs

Native Rust client for [TigerBeetle](https://tigerbeetle.com), the financial transactions database.

## Features

- **High-performance**: Uses io_uring for efficient async I/O on Linux
- **Type-safe**: Strong typing for accounts, transfers, and results
- **Simple API**: One `Client` type with a clean builder pattern

## Requirements

- Linux (kernel 5.6+) with io_uring support
- Rust 1.75+

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tb-rs = "0.1"
```

## Quick Start

```rust
use tb_rs::{Client, Account, AccountFlags};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Must run inside tokio_uring runtime
    tokio_uring::start(async {
        // Connect to cluster
        let mut client = Client::connect(0, "127.0.0.1:3000").await?;

        // Create an account
        let account = Account {
            id: tb_rs::id(),
            ledger: 1,
            code: 1,
            ..Default::default()
        };
        let errors = client.create_accounts(&[account]).await?;
        assert!(errors.is_empty(), "Account creation failed");

        // Lookup the account
        let accounts = client.lookup_accounts(&[account.id]).await?;
        println!("Found {} accounts", accounts.len());

        client.close().await;
        Ok(())
    })
}
```

## Configuration

Use the builder pattern for custom configuration:

```rust
use std::time::Duration;
use tb_rs::Client;

let client = Client::builder()
    .cluster(0)
    .addresses("127.0.0.1:3000,127.0.0.1:3001")?
    .connect_timeout(Duration::from_secs(10))
    .request_timeout(Duration::from_millis(100))
    .build()
    .await?;
```

## API

### Account Operations

- `create_accounts(&[Account])` - Create accounts, returns errors for failures
- `lookup_accounts(&[u128])` - Lookup accounts by ID
- `query_accounts(QueryFilter)` - Query accounts with filters
- `get_account_balances(AccountFilter)` - Get balance history

### Transfer Operations

- `create_transfers(&[Transfer])` - Create transfers, returns errors for failures
- `lookup_transfers(&[u128])` - Lookup transfers by ID
- `query_transfers(QueryFilter)` - Query transfers with filters
- `get_account_transfers(AccountFilter)` - Get transfers for an account

## Thread Safety

The `Client` is `!Send` because io_uring submission queues are thread-local.
Create one client per thread if you need multi-threaded access.

## License

Apache-2.0
