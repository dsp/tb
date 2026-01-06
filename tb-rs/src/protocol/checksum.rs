//! TigerBeetle checksum implementation using Aegis128L.
//!
//! TigerBeetle uses Aegis128L AEAD with a zero key/nonce for fast, hardware-accelerated
//! checksumming. The authentication tag serves as the checksum, providing strong
//! integrity guarantees while being extremely fast on modern CPUs with AES-NI support.

use aegis::aegis128l::Aegis128L;

/// Zero key used for checksum (TigerBeetle convention).
const ZERO_KEY: [u8; 16] = [0u8; 16];

/// Zero nonce used for checksum (TigerBeetle convention).
const ZERO_NONCE: [u8; 16] = [0u8; 16];

/// Compute the TigerBeetle checksum for the given data.
///
/// This uses Aegis128L AEAD with a zero key and nonce, computing the
/// authentication tag which serves as a fast, hardware-accelerated checksum.
/// The data to checksum is passed as Associated Data (AD), with an empty message.
/// This matches TigerBeetle's MAC mode usage: "in mac mode, message to sign is
/// treated as AD, not as a secret message."
pub fn checksum(data: &[u8]) -> u128 {
    let cipher = Aegis128L::<16>::new(&ZERO_KEY, &ZERO_NONCE);
    // Data is passed as Associated Data (AD), not as message
    let (_, tag) = cipher.encrypt(&[], data);
    u128::from_le_bytes(tag)
}

/// Streaming checksum for incremental computation.
///
/// Note: This accumulates data internally and computes the checksum at finalization.
/// For large data, consider using `checksum()` directly on the complete data.
pub struct ChecksumStream {
    data: Vec<u8>,
}

impl ChecksumStream {
    /// Create a new checksum stream.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Add data to the checksum computation.
    pub fn update(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    /// Finalize and return the checksum.
    pub fn finalize(self) -> u128 {
        checksum(&self.data)
    }
}

impl Default for ChecksumStream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test vector from TigerBeetle: empty input.
    /// Expected value: 0x49F174618255402DE6E7E3C40D60CC83
    #[test]
    fn test_checksum_empty() {
        let result = checksum(&[]);
        // The expected value from Zig (already in correct byte order for u128)
        assert_eq!(result, 0x49F174618255402DE6E7E3C40D60CC83);
    }

    /// Test that streaming checksum matches direct checksum.
    #[test]
    fn test_checksum_stream() {
        let data = b"Hello, TigerBeetle!";
        let direct = checksum(data);

        let mut stream = ChecksumStream::new();
        stream.update(&data[..5]);
        stream.update(&data[5..]);
        let streamed = stream.finalize();

        assert_eq!(direct, streamed);
    }

    /// Test that different inputs produce different checksums.
    #[test]
    fn test_checksum_uniqueness() {
        let a = checksum(b"hello");
        let b = checksum(b"Hello");
        let c = checksum(b"hello ");
        let d = checksum(b"");

        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(b, c);
        assert_ne!(b, d);
        assert_ne!(c, d);
    }

    /// Test checksum of zeros (various lengths).
    #[test]
    fn test_checksum_zeros() {
        let checksums: Vec<u128> = (0..32).map(|len| checksum(&vec![0u8; len])).collect();

        // All should be unique
        for (i, a) in checksums.iter().enumerate() {
            for b in &checksums[..i] {
                assert_ne!(a, b, "Checksums at different lengths should differ");
            }
        }
    }

    /// Test that checksum is non-zero for any input.
    #[test]
    fn test_checksum_non_trivial() {
        for len in 0..64 {
            let result = checksum(&vec![0u8; len]);
            assert_ne!(result, 0, "Checksum should not be zero for length {}", len);
            assert_ne!(
                result,
                u128::MAX,
                "Checksum should not be u128::MAX for length {}",
                len
            );
        }
    }
}
