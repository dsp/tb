//! Test data generator for TigerBeetle.
//!
//! Generates random accounts and transfers for testing and benchmarking.
//!
//! # Usage
//!
//! ```bash
//! # Generate 100 accounts and 1000 transfers
//! tb-gen --accounts 100 --transfers 1000 --address 127.0.0.1:3001
//!
//! # Generate only accounts
//! tb-gen --accounts 50 --address 127.0.0.1:3001
//!
//! # Use custom ledger and batch size
//! tb-gen --accounts 100 --transfers 500 --ledger 1 --batch-size 1000
//! ```

use clap::Parser;
use rand::Rng;
use tb_rs::{Account, AccountFlags, Transfer, TransferFlags};

/// Test data generator for TigerBeetle
#[derive(Parser, Debug)]
#[command(name = "tb-gen")]
#[command(about = "Generate test data for TigerBeetle")]
struct Args {
    /// TigerBeetle server address
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    address: String,

    /// Cluster ID
    #[arg(short, long, default_value_t = 0)]
    cluster: u128,

    /// Number of accounts to create
    #[arg(long, default_value_t = 100)]
    accounts: u32,

    /// Number of transfers to create
    #[arg(long, default_value_t = 0)]
    transfers: u32,

    /// Ledger ID for all accounts and transfers
    #[arg(short, long, default_value_t = 1)]
    ledger: u32,

    /// Account code
    #[arg(long, default_value_t = 1)]
    code: u16,

    /// Batch size for sending requests (will be capped by server limit)
    #[arg(short, long, default_value_t = 8190)]
    batch_size: u32,

    /// Maximum transfer amount
    #[arg(long, default_value_t = 10000)]
    max_amount: u128,

    /// Dry run - generate data but don't send to server
    #[arg(long)]
    dry_run: bool,
}

/// Generate a batch of random accounts.
fn generate_accounts(count: u32, ledger: u32, code: u16) -> Vec<Account> {
    let mut accounts = Vec::with_capacity(count as usize);

    for _ in 0..count {
        accounts.push(Account {
            id: tb_rs::id(),
            ledger,
            code,
            flags: AccountFlags::empty(),
            ..Default::default()
        });
    }

    accounts
}

/// Generate a batch of random transfers between accounts.
fn generate_transfers(
    count: u32,
    account_ids: &[u128],
    ledger: u32,
    code: u16,
    max_amount: u128,
) -> Vec<Transfer> {
    assert!(
        account_ids.len() >= 2,
        "Need at least 2 accounts for transfers"
    );

    let mut rng = rand::thread_rng();
    let mut transfers = Vec::with_capacity(count as usize);

    for _ in 0..count {
        // Pick random debit and credit accounts (must be different)
        let debit_idx = rng.gen_range(0..account_ids.len());
        let mut credit_idx = rng.gen_range(0..account_ids.len());
        while credit_idx == debit_idx {
            credit_idx = rng.gen_range(0..account_ids.len());
        }

        let amount = rng.gen_range(1..=max_amount);

        transfers.push(Transfer {
            id: tb_rs::id(),
            debit_account_id: account_ids[debit_idx],
            credit_account_id: account_ids[credit_idx],
            amount,
            ledger,
            code,
            flags: TransferFlags::empty(),
            ..Default::default()
        });
    }

    transfers
}

async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("TigerBeetle Test Data Generator");
    println!("================================");
    println!("Address: {}", args.address);
    println!("Cluster: {}", args.cluster);
    println!("Accounts: {}", args.accounts);
    println!("Transfers: {}", args.transfers);
    println!("Ledger: {}", args.ledger);
    println!("Batch size: {}", args.batch_size);
    println!();

    if args.accounts == 0 {
        println!("No accounts to create. Exiting.");
        return Ok(());
    }

    if args.transfers > 0 && args.accounts < 2 {
        return Err("Need at least 2 accounts to create transfers".into());
    }

    // Generate all accounts first
    println!("Generating {} accounts...", args.accounts);
    let accounts = generate_accounts(args.accounts, args.ledger, args.code);
    let account_ids: Vec<u128> = accounts.iter().map(|a| a.id).collect();
    println!("Generated {} accounts", accounts.len());

    // Generate transfers if requested
    let transfers = if args.transfers > 0 {
        println!("Generating {} transfers...", args.transfers);
        let t = generate_transfers(
            args.transfers,
            &account_ids,
            args.ledger,
            args.code,
            args.max_amount,
        );
        println!("Generated {} transfers", t.len());
        t
    } else {
        Vec::new()
    };

    if args.dry_run {
        println!();
        println!("Dry run mode - not sending to server");
        println!("Sample account: {:032x}", accounts[0].id);
        if !transfers.is_empty() {
            println!(
                "Sample transfer: {:032x} ({} units)",
                transfers[0].id, transfers[0].amount
            );
        }
        return Ok(());
    }

    // Connect to TigerBeetle
    println!();
    println!("Connecting to TigerBeetle at {}...", args.address);
    let mut client = tb_rs::Client::connect(args.cluster, &args.address).await?;
    println!("Connected! Client ID: {:032x}", client.id());

    // Use the server's batch size limit (tb-rs will reject oversized batches)
    let effective_batch_size = client
        .max_batch_count::<Account>()
        .map(|max| std::cmp::min(args.batch_size, max))
        .unwrap_or(args.batch_size);
    println!(
        "Using batch size: {} (max: {:?})",
        effective_batch_size,
        client.max_batch_count::<Account>()
    );

    // Create accounts in batches
    println!();
    println!("Creating accounts...");
    let mut accounts_created: u32 = 0;
    let mut accounts_failed: u32 = 0;

    for chunk in accounts.chunks(effective_batch_size as usize) {
        let results = client.create_accounts(chunk).await?;

        if results.is_empty() {
            accounts_created += chunk.len() as u32;
        } else {
            // Some accounts failed
            accounts_failed += results.len() as u32;
            accounts_created += (chunk.len() - results.len()) as u32;

            for result in &results {
                eprintln!(
                    "  Account {} failed: {:?}",
                    result.index, result.result
                );
            }
        }

        print!(
            "\r  Progress: {}/{} accounts",
            accounts_created + accounts_failed,
            args.accounts
        );
    }
    println!();
    println!(
        "Accounts: {} created, {} failed",
        accounts_created, accounts_failed
    );

    // Create transfers in batches
    if !transfers.is_empty() {
        println!();
        println!("Creating transfers...");
        let mut transfers_created: u32 = 0;
        let mut transfers_failed: u32 = 0;

        for chunk in transfers.chunks(effective_batch_size as usize) {
            let results = client.create_transfers(chunk).await?;

            if results.is_empty() {
                transfers_created += chunk.len() as u32;
            } else {
                // Some transfers failed
                transfers_failed += results.len() as u32;
                transfers_created += (chunk.len() - results.len()) as u32;

                for result in &results {
                    eprintln!(
                        "  Transfer {} failed: {:?}",
                        result.index, result.result
                    );
                }
            }

            print!(
                "\r  Progress: {}/{} transfers",
                transfers_created + transfers_failed,
                args.transfers
            );
        }
        println!();
        println!(
            "Transfers: {} created, {} failed",
            transfers_created, transfers_failed
        );
    }

    // Close client
    client.close().await;

    println!();
    println!("Done!");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    tokio_uring::start(async { run(args).await })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_accounts() {
        let accounts = generate_accounts(10, 1, 100);

        assert_eq!(accounts.len(), 10);
        for account in &accounts {
            assert_ne!(account.id, 0);
            assert_eq!(account.ledger, 1);
            assert_eq!(account.code, 100);
            assert!(account.flags.is_empty());
        }

        // Verify all IDs are unique
        let mut ids: Vec<u128> = accounts.iter().map(|a| a.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10);
    }

    #[test]
    fn test_generate_transfers() {
        let account_ids: Vec<u128> = (1..=5).map(|i| i as u128).collect();
        let transfers = generate_transfers(20, &account_ids, 1, 50, 1000);

        assert_eq!(transfers.len(), 20);
        for transfer in &transfers {
            assert_ne!(transfer.id, 0);
            assert_eq!(transfer.ledger, 1);
            assert_eq!(transfer.code, 50);
            assert!(transfer.flags.is_empty());
            assert!(transfer.amount >= 1 && transfer.amount <= 1000);
            assert!(account_ids.contains(&transfer.debit_account_id));
            assert!(account_ids.contains(&transfer.credit_account_id));
            assert_ne!(transfer.debit_account_id, transfer.credit_account_id);
        }
    }

    #[test]
    #[should_panic(expected = "Need at least 2 accounts")]
    fn test_generate_transfers_requires_two_accounts() {
        let account_ids = vec![1u128];
        generate_transfers(1, &account_ids, 1, 1, 100);
    }
}
