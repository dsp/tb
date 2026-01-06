//! Example of using the tb-rs Client.
//!
//! This example shows the recommended way to use tb-rs on Linux.
//! The `Client` type uses io_uring for high-performance I/O.
//!
//! # Requirements
//!
//! - Linux kernel 5.6 or later
//!
//! # Running
//!
//! ```bash
//! cargo run --example uring_transport -- 127.0.0.1:3001
//! ```

use tb_rs::Client;

async fn run(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to TigerBeetle at {} using io_uring...", address);

    // Connect to the cluster (auto-registers)
    let mut client = Client::connect(0, address).await?;

    println!("Client ID: {:032x}", client.id());
    println!("Batch size limit: {:?}", client.batch_size_limit());

    // Create a test account
    let account = tb_rs::Account {
        id: tb_rs::id(),
        ledger: 1,
        code: 100,
        flags: tb_rs::AccountFlags::empty(),
        ..Default::default()
    };

    println!("Creating account {:032x}...", account.id);
    let results = client.create_accounts(&[account]).await?;

    if results.is_empty() {
        println!("Account created successfully!");
    } else {
        println!("Account creation result: {:?}", results[0].result);
    }

    // Lookup the account
    println!("Looking up account...");
    let accounts = client.lookup_accounts(&[account.id]).await?;

    if let Some(found) = accounts.first() {
        println!("Found account:");
        println!("  ID: {:032x}", found.id);
        println!("  Ledger: {}", found.ledger);
        println!("  Code: {}", found.code);
        println!(
            "  Debits: {} pending, {} posted",
            found.debits_pending, found.debits_posted
        );
        println!(
            "  Credits: {} pending, {} posted",
            found.credits_pending, found.credits_posted
        );
    } else {
        println!("Account not found");
    }

    // Close the client
    client.close().await;
    println!("Done!");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments for cluster address
    let args: Vec<String> = std::env::args().collect();
    let address = args.get(1).map(|s| s.as_str()).unwrap_or("127.0.0.1:3001");

    // Run in tokio_uring runtime
    tokio_uring::start(async { run(address).await })
}
