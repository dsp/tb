//! TigerBeetle client.
//!
//! This module provides the main [`Client`] type for interacting with
//! TigerBeetle clusters.
//!
//! # Example
//!
//! ```ignore
//! use tb_rs::{Client, Account, AccountFlags};
//!
//! tokio_uring::start(async {
//!     let mut client = Client::connect(0, "127.0.0.1:3000").await?;
//!
//!     let account = Account {
//!         id: tb_rs::id(),
//!         ledger: 1,
//!         code: 1,
//!         ..Default::default()
//!     };
//!     client.create_accounts(&[account]).await?;
//!
//!     client.close().await;
//!     Ok::<_, tb_rs::ClientError>(())
//! });
//! ```

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use rand::Rng;

use crate::error::{ClientError, ProtocolError, Result};
use crate::internal::{BufferPool, Driver, OwnedBuf};
use crate::protocol::{
    Account, AccountBalance, AccountFilter, Command, CreateAccountsResult, CreateTransfersResult,
    Header, Message, Operation, QueryFilter, RegisterRequest, RegisterResult, RequestBuilder,
    Transfer, HEADER_SIZE, MESSAGE_SIZE_MAX,
};

/// Minimum client release version.
const CLIENT_RELEASE: u32 = 1;

/// Client state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Disconnected,
    Registering,
    Ready,
    Shutdown,
}

/// TigerBeetle client.
///
/// Provides methods to create accounts, create transfers, and query data.
/// Uses io_uring for high-performance async I/O on Linux.
///
/// # Thread Safety
///
/// This client is `!Send` because io_uring submission queues are thread-local.
/// Create one client per thread if you need multi-threaded access.
///
/// # Example
///
/// ```ignore
/// use tb_rs::Client;
///
/// tokio_uring::start(async {
///     // Simple connection
///     let mut client = Client::connect(0, "127.0.0.1:3000").await?;
///
///     // Or with custom configuration
///     let mut client = Client::builder()
///         .cluster(0)
///         .addresses("127.0.0.1:3000,127.0.0.1:3001")?
///         .connect_timeout(Duration::from_secs(10))
///         .build()
///         .await?;
///
///     client.close().await;
///     Ok::<_, tb_rs::ClientError>(())
/// });
/// ```
pub struct Client {
    /// Unique client identifier (random).
    id: u128,
    /// Cluster identifier.
    cluster: u128,
    /// Number of replicas.
    replica_count: u8,
    /// I/O driver.
    driver: Driver,
    /// Client state.
    state: State,
    /// Current view (determines primary).
    view: u32,
    /// Session number.
    session: u64,
    /// Next request number.
    request_number: u32,
    /// Parent checksum for hash-chain.
    parent: u128,
    /// Batch size limit (from registration).
    batch_size_limit: Option<u32>,
    /// PRNG for hedging.
    rng: rand::rngs::StdRng,
    /// Send buffer.
    send_buffer: Vec<u8>,
    /// Buffer pool for receives.
    buffer_pool: BufferPool,
    /// Request timeout.
    request_timeout: Duration,
    /// Maximum request timeout.
    request_timeout_max: Duration,
}

impl Client {
    /// Connect to a TigerBeetle cluster.
    ///
    /// This is the simplest way to create a client. It connects to the cluster
    /// and registers automatically.
    ///
    /// # Arguments
    ///
    /// * `cluster` - Cluster ID (must match the cluster configuration)
    /// * `addresses` - Comma-separated replica addresses (e.g., "127.0.0.1:3000")
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut client = Client::connect(0, "127.0.0.1:3000").await?;
    /// ```
    pub async fn connect(cluster: u128, addresses: &str) -> Result<Self> {
        Self::builder()
            .cluster(cluster)
            .addresses(addresses)?
            .build()
            .await
    }

    /// Create a client builder for custom configuration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let client = Client::builder()
    ///     .cluster(0)
    ///     .addresses("127.0.0.1:3000")?
    ///     .connect_timeout(Duration::from_secs(10))
    ///     .request_timeout(Duration::from_millis(100))
    ///     .build()
    ///     .await?;
    /// ```
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Get the client ID.
    pub fn id(&self) -> u128 {
        self.id
    }

    /// Get the cluster ID.
    pub fn cluster(&self) -> u128 {
        self.cluster
    }

    /// Check if the client is ready for operations.
    pub fn is_ready(&self) -> bool {
        self.state == State::Ready
    }

    /// Get the batch size limit in bytes (available after registration).
    pub fn batch_size_limit(&self) -> Option<u32> {
        self.batch_size_limit
    }

    /// Get the maximum number of elements that can be sent in a single batch.
    ///
    /// This accounts for the multi-batch trailer overhead.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let max_accounts = client.max_batch_count::<Account>();
    /// let max_transfers = client.max_batch_count::<Transfer>();
    /// ```
    pub fn max_batch_count<T>(&self) -> Option<u32> {
        let limit = self.batch_size_limit?;
        let element_size = std::mem::size_of::<T>() as u32;
        if element_size == 0 {
            return None;
        }
        // Trailer is aligned to element_size
        let trailer_size = crate::protocol::multi_batch::trailer_total_size(element_size, 1);
        let max_payload = limit.saturating_sub(trailer_size);
        Some(max_payload / element_size)
    }

    /// Create accounts.
    ///
    /// Returns errors for accounts that could not be created.
    /// An empty result means all accounts were created successfully.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let account = Account {
    ///     id: tb_rs::id(),
    ///     ledger: 1,
    ///     code: 1,
    ///     ..Default::default()
    /// };
    ///
    /// let errors = client.create_accounts(&[account]).await?;
    /// if errors.is_empty() {
    ///     println!("Account created!");
    /// }
    /// ```
    pub async fn create_accounts(
        &mut self,
        accounts: &[Account],
    ) -> Result<Vec<CreateAccountsResult>> {
        let response = self.request(Operation::CreateAccounts, accounts).await?;
        let payload = crate::protocol::multi_batch::decode(
            &response,
            std::mem::size_of::<CreateAccountsResult>() as u32,
        );
        Ok(parse_results(payload))
    }

    /// Create transfers.
    ///
    /// Returns errors for transfers that could not be created.
    /// An empty result means all transfers were created successfully.
    pub async fn create_transfers(
        &mut self,
        transfers: &[Transfer],
    ) -> Result<Vec<CreateTransfersResult>> {
        let response = self.request(Operation::CreateTransfers, transfers).await?;
        let payload = crate::protocol::multi_batch::decode(
            &response,
            std::mem::size_of::<CreateTransfersResult>() as u32,
        );
        Ok(parse_results(payload))
    }

    /// Lookup accounts by ID.
    pub async fn lookup_accounts(&mut self, ids: &[u128]) -> Result<Vec<Account>> {
        let response = self.request(Operation::LookupAccounts, ids).await?;
        let payload =
            crate::protocol::multi_batch::decode(&response, std::mem::size_of::<Account>() as u32);
        Ok(parse_results(payload))
    }

    /// Lookup transfers by ID.
    pub async fn lookup_transfers(&mut self, ids: &[u128]) -> Result<Vec<Transfer>> {
        let response = self.request(Operation::LookupTransfers, ids).await?;
        let payload =
            crate::protocol::multi_batch::decode(&response, std::mem::size_of::<Transfer>() as u32);
        Ok(parse_results(payload))
    }

    /// Get transfers for an account.
    pub async fn get_account_transfers(&mut self, filter: AccountFilter) -> Result<Vec<Transfer>> {
        let response = self
            .request(Operation::GetAccountTransfers, &[filter])
            .await?;
        let payload =
            crate::protocol::multi_batch::decode(&response, std::mem::size_of::<Transfer>() as u32);
        Ok(parse_results(payload))
    }

    /// Get balance history for an account.
    pub async fn get_account_balances(
        &mut self,
        filter: AccountFilter,
    ) -> Result<Vec<AccountBalance>> {
        let response = self
            .request(Operation::GetAccountBalances, &[filter])
            .await?;
        let payload = crate::protocol::multi_batch::decode(
            &response,
            std::mem::size_of::<AccountBalance>() as u32,
        );
        Ok(parse_results(payload))
    }

    /// Query accounts.
    pub async fn query_accounts(&mut self, filter: QueryFilter) -> Result<Vec<Account>> {
        let response = self.request(Operation::QueryAccounts, &[filter]).await?;
        let payload =
            crate::protocol::multi_batch::decode(&response, std::mem::size_of::<Account>() as u32);
        Ok(parse_results(payload))
    }

    /// Query transfers.
    pub async fn query_transfers(&mut self, filter: QueryFilter) -> Result<Vec<Transfer>> {
        let response = self.request(Operation::QueryTransfers, &[filter]).await?;
        let payload =
            crate::protocol::multi_batch::decode(&response, std::mem::size_of::<Transfer>() as u32);
        Ok(parse_results(payload))
    }

    /// Close the client and release resources.
    pub async fn close(mut self) {
        self.state = State::Shutdown;
        self.driver.close().await;
        self.buffer_pool.clear_quarantine();
    }

    // ========================================================================
    // Internal methods
    // ========================================================================

    /// Register with the cluster.
    async fn register(&mut self) -> Result<()> {
        if self.state != State::Disconnected {
            return Err(ClientError::InvalidOperation);
        }

        self.state = State::Registering;

        // Build register request
        let body = RegisterRequest::default();
        let body_bytes = unsafe {
            std::slice::from_raw_parts(
                &body as *const _ as *const u8,
                std::mem::size_of::<RegisterRequest>(),
            )
        };

        let msg = RequestBuilder::new(self.cluster, self.id)
            .session(0)
            .request(0)
            .parent(0)
            .operation(Operation::Register)
            .release(CLIENT_RELEASE)
            .body(body_bytes)
            .build();

        self.parent = msg.header().checksum;

        // Send and wait for reply
        let reply = self.send_request_with_retry(msg).await?;

        // Parse register result
        let body = reply.body();
        if body.len() < std::mem::size_of::<RegisterResult>() {
            return Err(ClientError::Protocol(ProtocolError::InvalidSize));
        }

        let result: &RegisterResult = unsafe { &*(body.as_ptr() as *const RegisterResult) };

        // Update state
        self.batch_size_limit = Some(result.batch_size_limit);
        self.session = reply.header().as_reply().commit;
        self.parent = reply.header().as_reply().context;
        self.request_number = 1;
        self.state = State::Ready;

        Ok(())
    }

    /// Send a request.
    async fn request<E: Copy>(&mut self, operation: Operation, events: &[E]) -> Result<Vec<u8>> {
        if self.state != State::Ready {
            return Err(ClientError::NotRegistered);
        }

        // Serialize events
        let events_bytes = unsafe {
            std::slice::from_raw_parts(
                events.as_ptr() as *const u8,
                std::mem::size_of_val(events),
            )
        };

        // Apply multi-batch encoding if needed
        let body_slice: &[u8] = if operation.is_multi_batch() {
            let element_size = std::mem::size_of::<E>() as u32;
            let trailer_size = crate::protocol::multi_batch::trailer_total_size(element_size, 1);
            let total_size = events_bytes.len() as u32 + trailer_size;

            // Validate batch size before sending
            if let Some(limit) = self.batch_size_limit {
                if total_size > limit {
                    return Err(ClientError::RequestTooLarge {
                        size: total_size,
                        limit,
                    });
                }
            }
            let encoded_size = crate::protocol::multi_batch::encode(
                &mut self.send_buffer[..total_size as usize],
                events_bytes,
                element_size,
            );
            &self.send_buffer[..encoded_size as usize]
        } else {
            events_bytes
        };

        // Build request
        let msg = RequestBuilder::new(self.cluster, self.id)
            .session(self.session)
            .request(self.request_number)
            .parent(self.parent)
            .operation(operation)
            .release(CLIENT_RELEASE)
            .view(self.view)
            .body(body_slice)
            .build();

        self.parent = msg.header().checksum;
        self.request_number += 1;

        // Send with retry
        let reply = self.send_request_with_retry(msg).await?;

        // Update state
        let reply_header = reply.header().as_reply();
        self.parent = reply_header.context;

        if reply.header().view > self.view {
            self.view = reply.header().view;
        }

        Ok(reply.body().to_vec())
    }

    /// Send request with hedging and retry.
    async fn send_request_with_retry(&mut self, msg: Message) -> Result<Message> {
        let mut timeout = self.request_timeout;
        let expected_checksum = msg.header().checksum;

        loop {
            // Send with hedging
            self.send_with_hedging(&msg).await?;

            // Wait for reply
            match self.wait_for_reply(expected_checksum, timeout).await {
                Ok(reply) => return Ok(reply),
                Err(ClientError::Timeout) => {
                    // Exponential backoff with jitter
                    timeout = std::cmp::min(timeout * 2, self.request_timeout_max);
                    let jitter = self.rng.gen_range(0..timeout.as_millis() as u64 / 4);
                    timeout += Duration::from_millis(jitter);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Send with hedging (primary + random backup).
    async fn send_with_hedging(&mut self, msg: &Message) -> Result<()> {
        let primary = (self.view % self.replica_count as u32) as usize;

        // Ensure primary connected
        self.ensure_connected(primary).await?;
        self.driver.send(primary, msg.as_bytes()).await?;

        // Send to backup (hedging)
        if self.replica_count > 1 {
            let backup_offset = self.rng.gen_range(1..self.replica_count as usize);
            let backup = (primary + backup_offset) % self.replica_count as usize;

            if self.ensure_connected(backup).await.is_ok() {
                let _ = self.driver.send(backup, msg.as_bytes()).await;
            }
        }

        Ok(())
    }

    /// Ensure connected to a replica.
    async fn ensure_connected(&mut self, idx: usize) -> Result<()> {
        if !self.driver.is_connected(idx) {
            self.driver.connect(idx).await?;
        }
        Ok(())
    }

    /// Wait for a reply matching the expected checksum.
    async fn wait_for_reply(
        &mut self,
        expected_checksum: u128,
        timeout: Duration,
    ) -> Result<Message> {
        let start = Instant::now();
        let primary = (self.view % self.replica_count as u32) as usize;

        loop {
            if start.elapsed() >= timeout {
                return Err(ClientError::Timeout);
            }

            // Get a buffer
            let buf = self
                .buffer_pool
                .acquire()
                .ok_or(ClientError::Connection("buffer pool exhausted".into()))?;

            // Try to receive from primary
            let buf = match self.driver.recv(primary, buf).await {
                Ok(b) => b,
                Err(e) => {
                    // Connection error - try to reconnect
                    self.driver.disconnect(primary).await;
                    return Err(e);
                }
            };

            // Try to parse
            match self.try_parse_reply(&buf, expected_checksum) {
                Ok(msg) => {
                    self.buffer_pool.release(buf);
                    return Ok(msg);
                }
                Err(ParseError::NeedMoreData) => {
                    // TODO: Handle partial messages
                    self.buffer_pool.release(buf);
                    continue;
                }
                Err(ParseError::WrongReply) => {
                    self.buffer_pool.release(buf);
                    continue;
                }
                Err(ParseError::Evicted(reason)) => {
                    self.buffer_pool.release(buf);
                    return Err(ClientError::Evicted(reason));
                }
                Err(ParseError::Protocol(e)) => {
                    self.buffer_pool.release(buf);
                    self.driver.disconnect(primary).await;
                    return Err(ClientError::Protocol(e));
                }
            }
        }
    }

    /// Try to parse a reply.
    fn try_parse_reply(
        &self,
        buf: &OwnedBuf,
        expected_checksum: u128,
    ) -> std::result::Result<Message, ParseError> {
        let data = buf.as_slice();

        if data.len() < HEADER_SIZE as usize {
            return Err(ParseError::NeedMoreData);
        }

        let header_bytes: &[u8; HEADER_SIZE as usize] = data[..HEADER_SIZE as usize]
            .try_into()
            .map_err(|_| ParseError::Protocol(ProtocolError::InvalidHeader))?;
        let header = Header::from_bytes(header_bytes);

        if !header.valid_checksum() {
            return Err(ParseError::Protocol(ProtocolError::InvalidHeaderChecksum));
        }

        if header.command != Command::Reply as u8 {
            if header.command == Command::Eviction as u8 {
                let reason = header.as_eviction().reason;
                return Err(ParseError::Evicted(
                    reason
                        .try_into()
                        .unwrap_or(crate::protocol::header::EvictionReason::NoSession),
                ));
            }
            return Err(ParseError::Protocol(ProtocolError::UnexpectedReply));
        }

        let total_size = header.size as usize;
        if data.len() < total_size {
            return Err(ParseError::NeedMoreData);
        }

        let reply_header = header.as_reply();
        if reply_header.request_checksum != expected_checksum {
            return Err(ParseError::WrongReply);
        }
        if reply_header.client != self.id {
            return Err(ParseError::WrongReply);
        }

        let body_data = &data[HEADER_SIZE as usize..total_size];
        if !header.valid_checksum_body(body_data) {
            return Err(ParseError::Protocol(ProtocolError::InvalidBodyChecksum));
        }

        let msg_data = data[..total_size].to_vec();
        let msg = Message::from_bytes(msg_data)
            .ok_or(ParseError::Protocol(ProtocolError::InvalidHeader))?;

        Ok(msg)
    }
}

/// Reply parsing errors.
enum ParseError {
    NeedMoreData,
    WrongReply,
    Evicted(crate::protocol::header::EvictionReason),
    Protocol(ProtocolError),
}

/// Parse response body as result types.
fn parse_results<R: Copy>(data: &[u8]) -> Vec<R> {
    let count = data.len() / std::mem::size_of::<R>();
    if count == 0 {
        return Vec::new();
    }
    let ptr = data.as_ptr() as *const R;
    unsafe { std::slice::from_raw_parts(ptr, count) }.to_vec()
}

// ============================================================================
// ClientBuilder
// ============================================================================

/// Builder for creating a [`Client`] with custom configuration.
///
/// # Example
///
/// ```ignore
/// let client = Client::builder()
///     .cluster(0)
///     .addresses("127.0.0.1:3000,127.0.0.1:3001")?
///     .connect_timeout(Duration::from_secs(10))
///     .build()
///     .await?;
/// ```
pub struct ClientBuilder {
    cluster: u128,
    addresses: Vec<SocketAddr>,
    connect_timeout: Duration,
    request_timeout: Duration,
    request_timeout_max: Duration,
}

impl ClientBuilder {
    /// Create a new builder with defaults.
    pub fn new() -> Self {
        Self {
            cluster: 0,
            addresses: Vec::new(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_millis(500),
            request_timeout_max: Duration::from_secs(30),
        }
    }

    /// Set the cluster ID.
    pub fn cluster(mut self, id: u128) -> Self {
        self.cluster = id;
        self
    }

    /// Set replica addresses from a comma-separated string.
    pub fn addresses(mut self, addrs: &str) -> Result<Self> {
        if addrs.trim().is_empty() {
            return Err(ClientError::Connection("no addresses provided".into()));
        }

        self.addresses = addrs
            .split(',')
            .map(|s| {
                s.trim().parse().map_err(|e| {
                    ClientError::Connection(format!("invalid address '{}': {}", s.trim(), e))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(self)
    }

    /// Set replica addresses from a vector.
    pub fn addresses_vec(mut self, addrs: Vec<SocketAddr>) -> Self {
        self.addresses = addrs;
        self
    }

    /// Set connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set initial request timeout.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Set maximum request timeout (for backoff).
    pub fn request_timeout_max(mut self, timeout: Duration) -> Self {
        self.request_timeout_max = timeout;
        self
    }

    /// Build the client.
    ///
    /// This connects to the cluster and registers the client.
    pub async fn build(self) -> Result<Client> {
        use rand::SeedableRng;

        if self.addresses.is_empty() {
            return Err(ClientError::Connection("no addresses provided".into()));
        }

        let id: u128 = rand::random();
        if id == 0 {
            return Err(ClientError::Protocol(ProtocolError::InvalidHeader));
        }

        let replica_count = self.addresses.len() as u8;
        let driver = Driver::new(self.addresses, self.connect_timeout);

        let buffer_count = replica_count as usize + 2;
        let buffer_pool = BufferPool::new(buffer_count, MESSAGE_SIZE_MAX as usize);

        let mut client = Client {
            id,
            cluster: self.cluster,
            replica_count,
            driver,
            state: State::Disconnected,
            view: 0,
            session: 0,
            request_number: 0,
            parent: 0,
            batch_size_limit: None,
            rng: rand::rngs::StdRng::from_entropy(),
            send_buffer: vec![0u8; MESSAGE_SIZE_MAX as usize],
            buffer_pool,
            request_timeout: self.request_timeout,
            request_timeout_max: self.request_timeout_max,
        };

        // Register with cluster
        client.register().await?;

        Ok(client)
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = ClientBuilder::new();
        assert_eq!(builder.cluster, 0);
        assert!(builder.addresses.is_empty());
        assert_eq!(builder.connect_timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_builder_addresses_empty() {
        let result = ClientBuilder::new().addresses("");
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_addresses_invalid() {
        let result = ClientBuilder::new().addresses("not-an-address");
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_addresses_valid() {
        let builder = ClientBuilder::new()
            .addresses("127.0.0.1:3000,127.0.0.1:3001")
            .unwrap();
        assert_eq!(builder.addresses.len(), 2);
    }

    #[test]
    fn test_parse_results_empty() {
        let data: &[u8] = &[];
        let results: Vec<u32> = parse_results(data);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_results() {
        let data: [u8; 8] = [1, 0, 0, 0, 2, 0, 0, 0];
        let results: Vec<u32> = parse_results(&data);
        assert_eq!(results, vec![1, 2]);
    }
}
