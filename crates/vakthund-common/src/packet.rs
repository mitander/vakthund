//! Packet Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Defines the Packet type as a zero‑copy wrapper over raw byte data.

use bytes::Bytes;

#[derive(Clone, Debug)]
pub struct Packet {
    pub data: Bytes,
}

impl Packet {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Bytes::from(data),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }
}
