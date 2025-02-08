//! Event system components for high-throughput messaging.
//!
//! Provides protocol-agnostic event handling with:
//! - Zero-copy payloads
//! - Thread-safe lock-free queues
//! - Network-specific event types

pub mod bus;
pub mod network;

// Re-export primary components
pub use bus::{EventBus, EventError};
pub use network::NetworkEvent;
