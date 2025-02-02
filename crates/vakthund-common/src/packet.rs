//! Packet Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Defines the Packet type as a zero‑copy wrapper over raw byte data.

use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Packet {
    pub data: Arc<[u8]>,
}

impl Packet {
    /// Creates a new Packet from a byte vector.
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: data.into_boxed_slice().into(),
        }
    }

    /// Returns the packet data as a UTF‑8 string, if possible.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }
}
