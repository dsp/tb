//! I/O driver managing connections to cluster replicas.

use std::marker::PhantomData;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::buffer::OwnedBuf;
use super::connection::{Connection, ConnectionState};
use crate::error::{ClientError, Result};

/// I/O driver for TigerBeetle cluster communication.
///
/// Manages connections to all replicas and handles send/recv operations.
/// This type is `!Send` because io_uring is thread-local.
pub struct Driver {
    connections: Vec<ConnectionState>,
    addresses: Vec<SocketAddr>,
    connect_timeout: Duration,
    start_time: Instant,
    _not_send: PhantomData<Rc<()>>,
}

impl Driver {
    /// Create a new driver.
    pub fn new(addresses: Vec<SocketAddr>, connect_timeout: Duration) -> Self {
        let connections = addresses.iter().map(|_| ConnectionState::Disconnected).collect();

        Self {
            connections,
            addresses,
            connect_timeout,
            start_time: Instant::now(),
            _not_send: PhantomData,
        }
    }

    /// Get the number of replicas.
    pub fn replica_count(&self) -> usize {
        self.addresses.len()
    }

    /// Connect to a replica.
    pub async fn connect(&mut self, idx: usize) -> Result<()> {
        if idx >= self.addresses.len() {
            return Err(ClientError::Connection(format!(
                "invalid replica index: {}",
                idx
            )));
        }

        if self.connections[idx].is_connected() {
            return Ok(());
        }

        let addr = self.addresses[idx];
        let conn = Connection::connect(addr, self.connect_timeout).await?;
        self.connections[idx] = ConnectionState::Connected(conn);

        Ok(())
    }

    /// Check if connected to a replica.
    pub fn is_connected(&self, idx: usize) -> bool {
        idx < self.connections.len() && self.connections[idx].is_connected()
    }

    /// Disconnect from a replica.
    pub async fn disconnect(&mut self, idx: usize) {
        if idx >= self.connections.len() {
            return;
        }

        if let Some(conn) = self.connections[idx].take() {
            conn.close().await;
        }
    }

    /// Send data to a replica.
    pub async fn send(&self, idx: usize, data: &[u8]) -> Result<()> {
        let conn = match &self.connections[idx] {
            ConnectionState::Connected(c) => c,
            ConnectionState::Disconnected => {
                return Err(ClientError::Connection("not connected".into()));
            }
        };

        conn.send(data).await
    }

    /// Receive data from a replica.
    ///
    /// Takes ownership of the buffer and returns it with received data.
    pub async fn recv(&self, idx: usize, mut buf: OwnedBuf) -> Result<OwnedBuf> {
        let conn = match &self.connections[idx] {
            ConnectionState::Connected(c) => c,
            ConnectionState::Disconnected => {
                return Err(ClientError::Connection("not connected".into()));
            }
        };

        let capacity = buf.capacity();
        let recv_buf = vec![0u8; capacity];

        let (n, recv_buf) = conn.recv(recv_buf).await?;

        buf.as_mut_slice()[..n].copy_from_slice(&recv_buf[..n]);
        buf.set_len(n);

        Ok(buf)
    }

    /// Get monotonic time in nanoseconds.
    pub fn now_ns(&self) -> u64 {
        self.start_time.elapsed().as_nanos() as u64
    }

    /// Disconnect all connections.
    pub async fn close(&mut self) {
        for idx in 0..self.connections.len() {
            self.disconnect(idx).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_creation() {
        let addrs = vec!["127.0.0.1:3001".parse().unwrap()];
        let driver = Driver::new(addrs, Duration::from_secs(5));
        assert_eq!(driver.replica_count(), 1);
        assert!(!driver.is_connected(0));
    }
}
