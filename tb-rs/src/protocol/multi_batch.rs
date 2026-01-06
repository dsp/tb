//! Multi-batch encoding for TigerBeetle operations.
//!
//! Multi-batching allows submitting multiple independent batches of work within
//! a single VSR message. This module handles encoding the batches with the
//! required trailer format.
//!
//! Trailer format (written from end of buffer toward beginning):
//! - Postamble (u16): batch_count
//! - TrailerItems (u16 each): element_count for each batch (in reverse order)
//! - Padding (0xFF bytes): to align to element_size

/// Calculate the trailer size for multi-batch encoding.
///
/// The trailer is aligned to the element_size.
pub fn trailer_total_size(element_size: u32, batch_count: u16) -> u32 {
    assert!(batch_count > 0);

    // Unpadded size: batch_count * 2 (TrailerItems) + 2 (Postamble)
    let trailer_unpadded_size = (batch_count as u32 * 2) + 2;

    if element_size == 0 {
        return trailer_unpadded_size;
    }

    // Round up to multiple of element_size
    trailer_unpadded_size.div_ceil(element_size) * element_size
}

/// Encode events with multi-batch format.
///
/// Returns the total encoded size (payload + trailer).
pub fn encode(buffer: &mut [u8], events: &[u8], element_size: u32) -> u32 {
    let events_len = events.len() as u32;
    let element_count = if element_size == 0 {
        0
    } else {
        (events_len / element_size) as u16
    };
    let batch_count: u16 = 1;

    let trailer_size = trailer_total_size(element_size, batch_count);
    let total_size = events_len + trailer_size;

    assert!((buffer.len() as u32) >= total_size);

    // Copy payload
    buffer[..events_len as usize].copy_from_slice(events);

    // Fill trailer with padding (0xFF)
    for byte in &mut buffer[events_len as usize..total_size as usize] {
        *byte = 0xFF;
    }

    // Write postamble (batch_count) at the very end
    let postamble_offset = (total_size - 2) as usize;
    buffer[postamble_offset..postamble_offset + 2].copy_from_slice(&batch_count.to_le_bytes());

    // Write TrailerItem (element_count) just before postamble
    let trailer_item_offset = postamble_offset - 2;
    buffer[trailer_item_offset..trailer_item_offset + 2]
        .copy_from_slice(&element_count.to_le_bytes());

    total_size
}

/// Decode a multi-batch message and return only the payload.
///
/// Returns the payload slice (excluding the trailer).
/// Returns an empty slice if the message is malformed.
pub fn decode(data: &[u8], element_size: u32) -> &[u8] {
    let data_len = data.len() as u32;
    if data_len < 2 {
        return &[];
    }

    // Read batch_count from last 2 bytes
    let batch_count =
        u16::from_le_bytes([data[(data_len - 2) as usize], data[(data_len - 1) as usize]]);
    if batch_count == 0 {
        return &[];
    }

    // Calculate trailer size
    let trailer_size = trailer_total_size(element_size, batch_count);
    if data_len < trailer_size {
        return &[];
    }

    // Return payload (everything before trailer)
    &data[..(data_len - trailer_size) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trailer_total_size() {
        // 1 batch, 128-byte elements: ceil(4 / 128) * 128 = 128
        assert_eq!(trailer_total_size(128, 1), 128);

        // 2 batches, 128-byte elements: ceil(6 / 128) * 128 = 128
        assert_eq!(trailer_total_size(128, 2), 128);

        // 1 batch, 8-byte elements: ceil(4 / 8) * 8 = 8
        assert_eq!(trailer_total_size(8, 1), 8);
    }

    #[test]
    fn test_encode_single_account() {
        // Account is 128 bytes, 1 element
        let events = [0u8; 128];
        let mut buffer = vec![0u8; 512];

        let size = encode(&mut buffer, &events, 128);

        // Should be 128 (payload) + 128 (trailer) = 256
        assert_eq!(size, 256);

        // Payload should be preserved
        assert_eq!(&buffer[..128], &events);

        // Trailer should have:
        // - Padding (0xFF) from 128 to 252
        // - TrailerItem at 252-253: element_count = 1
        // - Postamble at 254-255: batch_count = 1
        for i in 128..252 {
            assert_eq!(buffer[i], 0xFF, "padding at offset {}", i);
        }

        let element_count = u16::from_le_bytes([buffer[252], buffer[253]]);
        assert_eq!(element_count, 1);

        let batch_count = u16::from_le_bytes([buffer[254], buffer[255]]);
        assert_eq!(batch_count, 1);
    }

    #[test]
    fn test_decode_empty_results() {
        // Empty result response (8 bytes for element_size=8):
        // ff ff ff ff 00 00 01 00
        // padding(4) + element_count(0) + batch_count(1)
        let data = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x01, 0x00];
        let payload = decode(&data, 8);
        assert!(payload.is_empty());
    }

    #[test]
    fn test_decode_with_results() {
        // Response with 1 result (16 bytes):
        // 8 bytes payload + 8 bytes trailer
        // Trailer: ff ff ff ff 01 00 01 00
        let mut data = vec![0x42u8; 8]; // payload
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0x01, 0x00]);
        let payload = decode(&data, 8);
        assert_eq!(payload.len(), 8);
        assert_eq!(payload, &[0x42u8; 8]);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        // Encode and decode should roundtrip
        let events = [0xAB; 128];
        let mut buffer = vec![0u8; 512];
        let size = encode(&mut buffer, &events, 128);
        let payload = decode(&buffer[..size as usize], 128);
        assert_eq!(payload, &events);
    }
}
