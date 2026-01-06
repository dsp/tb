//! TCP connection wrapper for io_uring.

use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Duration;

use tokio_uring::net::TcpStream;

use crate::error::{ClientError, Result};

/// Connection state.
pub enum ConnectionState {
    Disconnected,
    Connected(Connection),
}

impl ConnectionState {
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionState::Connected(_))
    }

    pub fn take(&mut self) -> Option<Connection> {
        match std::mem::replace(self, ConnectionState::Disconnected) {
            ConnectionState::Connected(conn) => Some(conn),
            ConnectionState::Disconnected => None,
        }
    }
}

/// A TCP connection to a TigerBeetle replica.
pub struct Connection {
    stream: Rc<RefCell<Option<TcpStream>>>,
    addr: SocketAddr,
}

impl Connection {
    /// Connect to the given address.
    pub async fn connect(addr: SocketAddr, _timeout: Duration) -> Result<Self> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| ClientError::Connection(format!("failed to connect to {}: {}", addr, e)))?;

        stream
            .set_nodelay(true)
            .map_err(|e| ClientError::Connection(format!("failed to set nodelay: {}", e)))?;

        Ok(Self {
            stream: Rc::new(RefCell::new(Some(stream))),
            addr,
        })
    }

    /// Get the remote address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Send data.
    ///
    /// # Safety Note
    /// The RefCell borrow held across await is safe because tokio_uring is single-threaded
    /// and Connection is !Send, so the Future cannot be polled from different threads.
    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let stream_ref = self.stream.borrow();
        let stream = stream_ref
            .as_ref()
            .ok_or_else(|| ClientError::Connection("connection closed".into()))?;

        let mut written = 0;
        while written < data.len() {
            let buf: Vec<u8> = data[written..].to_vec();
            let (result, _buf): (std::io::Result<usize>, Vec<u8>) =
                stream.write(buf).submit().await;
            let n = result
                .map_err(|e| ClientError::Connection(format!("write failed: {}", e)))?;
            if n == 0 {
                return Err(ClientError::Connection("connection closed".into()));
            }
            written += n;
        }

        Ok(())
    }

    /// Receive data into a buffer.
    ///
    /// Returns (bytes_read, buffer).
    ///
    /// # Safety Note
    /// The RefCell borrow held across await is safe because tokio_uring is single-threaded
    /// and Connection is !Send, so the Future cannot be polled from different threads.
    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn recv(&self, buf: Vec<u8>) -> Result<(usize, Vec<u8>)> {
        let stream_ref = self.stream.borrow();
        let stream = stream_ref
            .as_ref()
            .ok_or_else(|| ClientError::Connection("connection closed".into()))?;

        let (result, buf): (std::io::Result<usize>, Vec<u8>) = stream.read(buf).await;
        let n = result.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof
                || e.kind() == std::io::ErrorKind::ConnectionReset
            {
                ClientError::Connection("connection closed".into())
            } else {
                ClientError::Connection(format!("read failed: {}", e))
            }
        })?;

        Ok((n, buf))
    }

    /// Close the connection.
    pub async fn close(self) {
        let _ = self.stream.borrow_mut().take();
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("addr", &self.addr)
            .finish()
    }
}
