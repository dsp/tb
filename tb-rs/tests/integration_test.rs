//! Integration tests for tb-rs.
//!
//! These tests require a running TigerBeetle server.
//! Set the TB_ADDR environment variable to the server address (e.g., "127.0.0.1:3001").
//!
//! Run with: TB_ADDR=127.0.0.1:3001 cargo test --test integration_test

use std::net::SocketAddr;
use tb_rs::{Account, AccountFlags, Client, QueryFilter, QueryFilterFlags};

/// Get the TigerBeetle address from environment variable.
fn get_tb_addr() -> Option<SocketAddr> {
    std::env::var("TB_ADDR").ok().and_then(|s| s.parse().ok())
}

/// Create a client connected to TigerBeetle.
async fn create_client() -> Option<Client> {
    let addr = get_tb_addr()?;
    eprintln!("Connecting to TigerBeetle at {}...", addr);

    match Client::connect(0, &addr.to_string()).await {
        Ok(client) => {
            eprintln!("Connected! Client ID: {:032x}", client.id());
            Some(client)
        }
        Err(e) => {
            eprintln!("Failed to connect: {:?}", e);
            None
        }
    }
}

/// Run a test inside tokio_uring runtime.
macro_rules! uring_test {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() {
            tokio_uring::start(async { $body.await });
        }
    };
}

uring_test!(test_query_accounts_pagination, async {
    // Skip if TB_ADDR is not set
    let Some(mut client) = create_client().await else {
        eprintln!("Skipping test: TB_ADDR not set or connection failed");
        return;
    };

    println!("Connected to TigerBeetle, client ID: {:032x}", client.id());
    println!("Batch size limit: {:?}", client.batch_size_limit());

    // Create test accounts
    let num_accounts = 25;
    let mut accounts: Vec<Account> = Vec::with_capacity(num_accounts);

    for i in 0..num_accounts {
        accounts.push(Account {
            id: tb_rs::id(), // Generate unique ID
            ledger: 1,
            code: 100 + (i as u16),
            flags: AccountFlags::empty(),
            user_data_128: i as u128,
            user_data_64: i as u64,
            user_data_32: i as u32,
            ..Default::default()
        });
    }

    println!("Creating {} test accounts...", num_accounts);
    let results = client.create_accounts(&accounts).await.unwrap();

    // Check for errors (empty results means all succeeded)
    if !results.is_empty() {
        for result in &results {
            println!(
                "Account creation result: index={}, result={:?}",
                result.index, result.result
            );
        }
    }
    println!("Accounts created successfully!");

    // Query accounts with pagination (similar to Python example)
    const LIMIT: u32 = 8189;
    let mut timestamp_min: u64 = 0;
    let mut total_queried = 0;

    println!("\nQuerying accounts with pagination (limit={})...", LIMIT);

    loop {
        let queried = client
            .query_accounts(QueryFilter {
                user_data_128: 0,
                user_data_64: 0,
                user_data_32: 0,
                ledger: 0,
                code: 0,
                reserved: [0; 6],
                timestamp_min,
                timestamp_max: 0,
                limit: LIMIT,
                flags: QueryFilterFlags::empty(),
            })
            .await
            .unwrap();

        if queried.is_empty() {
            break;
        }

        for account in &queried {
            println!(
                "Account: id={:032x}, ledger={}, code={}, timestamp={}",
                account.id, account.ledger, account.code, account.timestamp
            );
            timestamp_min = account.timestamp;
        }

        total_queried += queried.len();
        timestamp_min += 1;
    }

    println!(
        "\nTotal accounts queried: {} (created {})",
        total_queried, num_accounts
    );

    // We should have queried at least the accounts we created
    // (there may be more from previous test runs)
    assert!(
        total_queried >= num_accounts,
        "Expected at least {} accounts, got {}",
        num_accounts,
        total_queried
    );

    client.close().await;
    println!("Test completed successfully!");
});

uring_test!(test_create_and_lookup_accounts, async {
    let Some(mut client) = create_client().await else {
        eprintln!("Skipping test: TB_ADDR not set or connection failed");
        return;
    };

    // Create a single account
    let account_id = tb_rs::id();
    let account = Account {
        id: account_id,
        ledger: 42,
        code: 999,
        flags: AccountFlags::empty(),
        user_data_128: 0xDEADBEEF,
        ..Default::default()
    };

    println!("Creating account {:032x}...", account_id);
    let results = client.create_accounts(&[account]).await.unwrap();
    assert!(results.is_empty(), "Account creation failed: {:?}", results);

    // Lookup the account
    println!("Looking up account...");
    let found = client.lookup_accounts(&[account_id]).await.unwrap();
    assert_eq!(found.len(), 1, "Account not found");

    let found_account = &found[0];
    assert_eq!(found_account.id, account_id);
    assert_eq!(found_account.ledger, 42);
    assert_eq!(found_account.code, 999);
    assert_eq!(found_account.user_data_128, 0xDEADBEEF);
    assert!(found_account.timestamp > 0, "Timestamp should be set");

    println!("Account found with timestamp: {}", found_account.timestamp);

    client.close().await;
});

uring_test!(test_raw_protocol_debug, async {
    use tb_rs::protocol::{
        checksum::checksum,
        header::{Header, HEADER_SIZE},
        operation::{Command, Operation},
    };
    use tokio_uring::net::TcpStream;

    let Some(addr) = get_tb_addr() else {
        eprintln!("Skipping test: TB_ADDR not set");
        return;
    };

    eprintln!("Connecting to TigerBeetle at {}...", addr);

    // Connect directly
    let stream = TcpStream::connect(addr)
        .await
        .expect("Failed to connect");
    stream.set_nodelay(true).unwrap();

    eprintln!("Connected! Building register request...");

    // Generate client ID
    let client_id: u128 = rand::random();
    eprintln!("Client ID: {:032x}", client_id);

    // Create header
    let mut header = Header::default();
    header.cluster = 0; // cluster ID
    header.set_command(Command::Request);
    header.release = 1; // 0.0.1 - minimum release
    header.size = HEADER_SIZE as u32 + 256; // RegisterRequest is 256 bytes

    // Set request-specific fields
    {
        let req = header.as_request_mut();
        req.client = client_id;
        req.session = 0; // 0 for register
        req.request = 0; // first request
        req.parent = 0; // 0 for register
        req.set_operation(Operation::Register);
    }

    // RegisterRequest body (256 bytes) - batch_size_limit=0 + 252 bytes reserved
    let body = vec![0u8; 256];

    // Calculate body checksum
    header.checksum_body = checksum(&body);

    // Calculate header checksum
    header.set_checksum();

    // Build full message
    let mut message = Vec::with_capacity(HEADER_SIZE as usize + body.len());
    message.extend_from_slice(header.as_bytes());
    message.extend_from_slice(&body);

    eprintln!("Message size: {} bytes", message.len());
    eprintln!("Header checksum: {:032x}", header.checksum);
    eprintln!("Body checksum: {:032x}", header.checksum_body);
    eprintln!("Release: {}", header.release);
    eprintln!("Protocol: {}", header.protocol);
    eprintln!("Command: {}", header.command);
    eprintln!("Operation: {}", header.as_request().operation);

    // Print hex dump of first 128 bytes (common header + start of request fields)
    eprintln!("\nMessage hex dump (first 128 bytes):");
    for (i, chunk) in message[..128].chunks(16).enumerate() {
        eprint!("{:04x}: ", i * 16);
        for b in chunk {
            eprint!("{:02x} ", b);
        }
        eprintln!();
    }

    // Print the request-specific fields (bytes 128-255)
    eprintln!("\nRequest-specific fields (bytes 128-255):");
    for (i, chunk) in message[128..256].chunks(16).enumerate() {
        eprint!("{:04x}: ", 128 + i * 16);
        for b in chunk {
            eprint!("{:02x} ", b);
        }
        eprintln!();
    }

    // Send message
    eprintln!("\nSending message...");
    let (result, _) = stream.write(message).submit().await;
    result.expect("Failed to send");
    eprintln!("Message sent!");

    // Try to read response with timeout
    let response = vec![0u8; HEADER_SIZE as usize];

    // Note: tokio_uring doesn't have built-in timeout, so we just do a blocking read
    let (result, response) = stream.read(response).await;
    match result {
        Ok(n) if n == HEADER_SIZE as usize => {
            eprintln!("Received {} bytes", n);

            // Parse response header
            let resp_header =
                Header::from_bytes((&response[..HEADER_SIZE as usize]).try_into().unwrap());
            eprintln!("\nResponse header:");
            eprintln!("  Checksum: {:032x}", resp_header.checksum);
            eprintln!("  Command: {}", resp_header.command);
            eprintln!("  Size: {}", resp_header.size);

            // Check if it's an eviction
            if resp_header.command == Command::Eviction as u8 {
                let eviction = resp_header.as_eviction();
                eprintln!("  EVICTION! Reason: {}", eviction.reason);
            } else if resp_header.command == Command::Reply as u8 {
                eprintln!("  Got Reply!");
                let reply = resp_header.as_reply();
                eprintln!("  Request checksum: {:032x}", reply.request_checksum);
                eprintln!("  Context: {:032x}", reply.context);
                eprintln!("  Commit: {}", reply.commit);
            }

            // Print hex dump of response
            eprintln!("\nResponse hex dump:");
            for (i, chunk) in response.chunks(16).enumerate() {
                eprint!("{:04x}: ", i * 16);
                for b in chunk {
                    eprint!("{:02x} ", b);
                }
                eprintln!();
            }
        }
        Ok(n) => {
            eprintln!("Partial read: {} bytes", n);
        }
        Err(e) => {
            eprintln!("Read error: {:?}", e);
        }
    }
});
