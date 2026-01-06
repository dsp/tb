//! Application state management.

use crate::config::Config;
use crate::transport::TigerBeetleClient;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state.
pub struct AppState {
    /// TigerBeetle client (mutex for shared access).
    pub client: Mutex<TigerBeetleClient>,
    /// Application configuration.
    pub config: Config,
}

impl AppState {
    /// Create new application state and connect to TigerBeetle.
    pub async fn new(config: Config) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        tracing::info!("Connecting to TigerBeetle at {}...", config.tb_address);

        let client = TigerBeetleClient::connect(config.cluster_id, config.tb_address).await?;

        tracing::info!(
            "Connected! Batch size limit: {:?}",
            client.batch_size_limit()
        );

        Ok(Arc::new(Self {
            client: Mutex::new(client),
            config,
        }))
    }
}
