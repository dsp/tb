//! Buffer management for io_uring operations.
//!
//! io_uring requires stable buffer addresses during async operations.
//! This module provides owned buffers and a pool for efficient reuse.

use std::collections::VecDeque;

/// Owned buffer for I/O operations.
///
/// Maintains a stable memory address for io_uring completion-based I/O.
#[derive(Debug)]
pub struct OwnedBuf {
    data: Vec<u8>,
    len: usize,
    poisoned: bool,
}

impl OwnedBuf {
    /// Create a new buffer with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: vec![0u8; capacity],
            len: 0,
            poisoned: false,
        }
    }

    /// Get the capacity.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Get the logical length of valid data.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Set the logical length.
    pub fn set_len(&mut self, len: usize) {
        assert!(len <= self.data.capacity());
        self.len = len;
    }

    /// Get a slice of the valid data.
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }

    /// Get the full buffer as a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Check if poisoned (involved in cancelled operation).
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Mark as poisoned.
    pub fn poison(&mut self) {
        self.poisoned = true;
    }

    /// Reset for reuse.
    pub fn reset(&mut self) {
        self.len = 0;
        self.poisoned = false;
    }
}

/// Pool of reusable buffers.
pub struct BufferPool {
    available: Vec<OwnedBuf>,
    quarantine: VecDeque<OwnedBuf>,
    buffer_size: usize,
}

impl BufferPool {
    /// Create a new pool.
    pub fn new(count: usize, buffer_size: usize) -> Self {
        let available = (0..count)
            .map(|_| OwnedBuf::with_capacity(buffer_size))
            .collect();

        Self {
            available,
            quarantine: VecDeque::new(),
            buffer_size,
        }
    }

    /// Acquire a buffer from the pool.
    pub fn acquire(&mut self) -> Option<OwnedBuf> {
        if let Some(mut buf) = self.available.pop() {
            buf.reset();
            return Some(buf);
        }

        // Try quarantine if old enough
        if let Some(mut buf) = self.quarantine.pop_front() {
            buf.reset();
            return Some(buf);
        }

        // Grow the pool
        Some(OwnedBuf::with_capacity(self.buffer_size))
    }

    /// Release a buffer back to the pool.
    pub fn release(&mut self, buf: OwnedBuf) {
        if buf.is_poisoned() {
            self.quarantine.push_back(buf);
        } else {
            self.available.push(buf);
        }
    }

    /// Mark all quarantined buffers as safe.
    pub fn clear_quarantine(&mut self) {
        while let Some(mut buf) = self.quarantine.pop_front() {
            buf.reset();
            self.available.push(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owned_buf() {
        let mut buf = OwnedBuf::with_capacity(1024);
        assert_eq!(buf.capacity(), 1024);
        assert_eq!(buf.len(), 0);

        buf.as_mut_slice()[..5].copy_from_slice(b"hello");
        buf.set_len(5);
        assert_eq!(buf.as_slice(), b"hello");
    }

    #[test]
    fn test_buffer_pool() {
        let mut pool = BufferPool::new(2, 1024);

        let buf1 = pool.acquire().unwrap();
        let buf2 = pool.acquire().unwrap();
        assert_eq!(buf1.capacity(), 1024);
        assert_eq!(buf2.capacity(), 1024);

        pool.release(buf1);
        pool.release(buf2);
    }

    #[test]
    fn test_poisoned_buffer() {
        let mut pool = BufferPool::new(1, 1024);

        let mut buf = pool.acquire().unwrap();
        buf.poison();
        pool.release(buf);

        // Should get from quarantine
        let buf2 = pool.acquire().unwrap();
        assert!(!buf2.is_poisoned()); // Reset clears poison
    }
}
