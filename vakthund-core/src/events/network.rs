//! Network event types and payload handling.

use bytes::Bytes;
use std::net::SocketAddr;

/// Protocol-agnostic network event with metadata
#[derive(Clone, Debug)]
pub struct NetworkEvent {
    /// Monotonic timestamp in nanoseconds from system/clock
    pub timestamp: u64,

    /// Immutable payload buffer using zero-copy semantics
    pub payload: Bytes,

    /// Optional source address for network context
    pub source: Option<SocketAddr>,

    /// Optional destination address for network context
    pub destination: Option<SocketAddr>,
}

impl NetworkEvent {
    /// Creates a new network event with minimal fields
    #[inline]
    pub fn new(timestamp: u64, payload: Bytes) -> Self {
        Self {
            timestamp,
            payload,
            source: None,
            destination: None,
        }
    }
}
