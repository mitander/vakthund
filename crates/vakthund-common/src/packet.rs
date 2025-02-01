//! # Packet Module
//!
//! Defines the `Packet` struct that holds raw data for a network packet.
//! The underlying data is stored in an `Arc<[u8]>` to achieve zero‑copy semantics.

use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Packet {
    pub data: Arc<[u8]>,
}

impl Packet {
    /// Creates a new `Packet` from a vector of bytes.
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: data.into_boxed_slice().into(),
        }
    }

    /// Attempts to interpret the packet data as a UTF‑8 string.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::Packet;

    #[test]
    fn test_packet_as_str_positive() {
        let packet = Packet::new("test data".as_bytes().to_vec());
        assert_eq!(packet.as_str(), Some("test data"));
    }

    #[test]
    fn test_packet_as_str_negative() {
        // Create invalid UTF-8 (e.g. 0xff bytes).
        let packet = Packet::new(vec![0xff, 0xff, 0xff]);
        assert!(packet.as_str().is_none());
    }
}
