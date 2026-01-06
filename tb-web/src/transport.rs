//! TigerBeetle client wrapper for bridging tokio_uring and tokio runtimes.
//!
//! tb-rs uses tokio_uring (io_uring based), while axum uses regular tokio.
//! This module provides a wrapper that runs tb-rs in a dedicated thread
//! and communicates via channels.

use std::net::SocketAddr;
use std::thread;

use tokio::sync::{mpsc, oneshot};
use tb_rs::{
    Account, AccountBalance, AccountFilter, ClientError, CreateAccountsResult,
    CreateTransfersResult, QueryFilter, Transfer,
};

/// Request types for the TigerBeetle client thread.
enum Request {
    CreateAccounts {
        accounts: Vec<Account>,
        reply: oneshot::Sender<Result<Vec<CreateAccountsResult>, ClientError>>,
    },
    CreateTransfers {
        transfers: Vec<Transfer>,
        reply: oneshot::Sender<Result<Vec<CreateTransfersResult>, ClientError>>,
    },
    LookupAccounts {
        ids: Vec<u128>,
        reply: oneshot::Sender<Result<Vec<Account>, ClientError>>,
    },
    LookupTransfers {
        ids: Vec<u128>,
        reply: oneshot::Sender<Result<Vec<Transfer>, ClientError>>,
    },
    GetAccountTransfers {
        filter: AccountFilter,
        reply: oneshot::Sender<Result<Vec<Transfer>, ClientError>>,
    },
    GetAccountBalances {
        filter: AccountFilter,
        reply: oneshot::Sender<Result<Vec<AccountBalance>, ClientError>>,
    },
    QueryAccounts {
        filter: QueryFilter,
        reply: oneshot::Sender<Result<Vec<Account>, ClientError>>,
    },
    QueryTransfers {
        filter: QueryFilter,
        reply: oneshot::Sender<Result<Vec<Transfer>, ClientError>>,
    },
    BatchSizeLimit {
        reply: oneshot::Sender<Option<u32>>,
    },
    Shutdown,
}

/// TigerBeetle client wrapper that bridges tokio and tokio_uring runtimes.
///
/// This spawns a dedicated thread running tokio_uring for the tb-rs client
/// and provides an async interface compatible with regular tokio.
pub struct TigerBeetleClient {
    tx: mpsc::Sender<Request>,
    batch_size_limit: Option<u32>,
}

impl TigerBeetleClient {
    /// Connect to a TigerBeetle cluster.
    ///
    /// Spawns a background thread with tokio_uring runtime.
    pub async fn connect(
        cluster_id: u128,
        address: SocketAddr,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel::<Request>(32);
        let (ready_tx, ready_rx) = oneshot::channel::<Result<Option<u32>, String>>();

        let addr_str = address.to_string();

        // Spawn dedicated thread for tokio_uring runtime
        thread::spawn(move || {
            tokio_uring::start(async move {
                // Connect to TigerBeetle
                let client_result = tb_rs::Client::connect(cluster_id, &addr_str).await;

                match client_result {
                    Ok(client) => {
                        let batch_limit = client.batch_size_limit();
                        let _ = ready_tx.send(Ok(batch_limit));
                        run_client_loop(client, rx).await;
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(format!("Failed to connect: {:?}", e)));
                    }
                }
            });
        });

        // Wait for connection result
        let batch_size_limit = ready_rx
            .await
            .map_err(|_| "Client thread died during startup")?
            .map_err(|e| e)?;

        Ok(Self {
            tx,
            batch_size_limit,
        })
    }

    /// Get the batch size limit (available after registration).
    pub fn batch_size_limit(&self) -> Option<u32> {
        self.batch_size_limit
    }

    /// Check if the client is connected and ready.
    ///
    /// Returns true if the client thread is alive and has successfully registered.
    pub fn is_ready(&self) -> bool {
        // If we have a client, we've successfully connected and registered.
        // The channel being open indicates the thread is alive.
        !self.tx.is_closed()
    }

    /// Create accounts.
    pub async fn create_accounts(
        &self,
        accounts: &[Account],
    ) -> Result<Vec<CreateAccountsResult>, ClientError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::CreateAccounts {
                accounts: accounts.to_vec(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?;

        reply_rx
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?
    }

    /// Create transfers.
    pub async fn create_transfers(
        &self,
        transfers: &[Transfer],
    ) -> Result<Vec<CreateTransfersResult>, ClientError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::CreateTransfers {
                transfers: transfers.to_vec(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?;

        reply_rx
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?
    }

    /// Lookup accounts by ID.
    pub async fn lookup_accounts(&self, ids: &[u128]) -> Result<Vec<Account>, ClientError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::LookupAccounts {
                ids: ids.to_vec(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?;

        reply_rx
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?
    }

    /// Lookup transfers by ID.
    pub async fn lookup_transfers(&self, ids: &[u128]) -> Result<Vec<Transfer>, ClientError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::LookupTransfers {
                ids: ids.to_vec(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?;

        reply_rx
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?
    }

    /// Get transfers for an account.
    pub async fn get_account_transfers(
        &self,
        filter: AccountFilter,
    ) -> Result<Vec<Transfer>, ClientError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::GetAccountTransfers {
                filter,
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?;

        reply_rx
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?
    }

    /// Get balance history for an account.
    pub async fn get_account_balances(
        &self,
        filter: AccountFilter,
    ) -> Result<Vec<AccountBalance>, ClientError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::GetAccountBalances {
                filter,
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?;

        reply_rx
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?
    }

    /// Query accounts.
    pub async fn query_accounts(&self, filter: QueryFilter) -> Result<Vec<Account>, ClientError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::QueryAccounts {
                filter,
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?;

        reply_rx
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?
    }

    /// Query transfers.
    pub async fn query_transfers(&self, filter: QueryFilter) -> Result<Vec<Transfer>, ClientError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Request::QueryTransfers {
                filter,
                reply: reply_tx,
            })
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?;

        reply_rx
            .await
            .map_err(|_| ClientError::Connection("client thread died".into()))?
    }

    /// Shutdown the client.
    pub async fn shutdown(&self) {
        let _ = self.tx.send(Request::Shutdown).await;
    }
}

/// Run the client event loop in the tokio_uring thread.
async fn run_client_loop(mut client: tb_rs::Client, mut rx: mpsc::Receiver<Request>) {
    while let Some(request) = rx.recv().await {
        match request {
            Request::CreateAccounts { accounts, reply } => {
                let result = client.create_accounts(&accounts).await;
                let _ = reply.send(result);
            }
            Request::CreateTransfers { transfers, reply } => {
                let result = client.create_transfers(&transfers).await;
                let _ = reply.send(result);
            }
            Request::LookupAccounts { ids, reply } => {
                let result = client.lookup_accounts(&ids).await;
                let _ = reply.send(result);
            }
            Request::LookupTransfers { ids, reply } => {
                let result = client.lookup_transfers(&ids).await;
                let _ = reply.send(result);
            }
            Request::GetAccountTransfers { filter, reply } => {
                let result = client.get_account_transfers(filter).await;
                let _ = reply.send(result);
            }
            Request::GetAccountBalances { filter, reply } => {
                let result = client.get_account_balances(filter).await;
                let _ = reply.send(result);
            }
            Request::QueryAccounts { filter, reply } => {
                let result = client.query_accounts(filter).await;
                let _ = reply.send(result);
            }
            Request::QueryTransfers { filter, reply } => {
                let result = client.query_transfers(filter).await;
                let _ = reply.send(result);
            }
            Request::BatchSizeLimit { reply } => {
                let _ = reply.send(client.batch_size_limit());
            }
            Request::Shutdown => {
                client.close().await;
                break;
            }
        }
    }
}
