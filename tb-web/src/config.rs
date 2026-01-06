//! Configuration for tb-web.

use std::net::SocketAddr;

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address to bind the web server.
    pub address: SocketAddr,
    /// TigerBeetle cluster address.
    pub tb_address: SocketAddr,
    /// TigerBeetle cluster ID.
    pub cluster_id: u128,
}
