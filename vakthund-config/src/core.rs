//! Core system configuration parameters.
//!
//! Manages fundamental system properties that affect all components:
//! - Event bus sizing and behavior
//! - Memory allocation strategies

use serde::{Deserialize, Serialize};
use validator::{self, Validate};

use crate::validation;

/// Core system configuration parameters.
#[derive(Default, Debug, Serialize, Deserialize, Validate, Clone)]
pub struct CoreConfig {
    /// Event bus configuration for cross‑component communication.
    #[validate(nested)]
    pub event_bus: EventBusConfig,

    /// Memory management settings for performance‑critical paths.
    #[validate(nested)]
    pub memory: MemoryConfig,
}

/// Event bus configuration for the LMAX‑style ring buffer.
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct EventBusConfig {
    /// Capacity of the event bus (must be a power of two).
    #[serde(default = "default_capacity")]
    #[validate(range(min = 128, max = 1048576))]
    #[validate(custom(function = validation::validate_power_of_two))]
    pub capacity: usize,

    /// Whether to enforce power‑of‑two sizing (required for performance).
    #[serde(default = "default_true")]
    pub require_power_of_two: bool,

    /// Number of consumers.
    #[serde(default = "default_consumers")]
    pub num_consumers: u32,

    /// Spin strategy for full queue (yield, spin_loop, or block).
    #[serde(default = "default_spin_strategy")]
    pub full_queue_strategy: String,
}

fn default_capacity() -> usize {
    4096
}

fn default_true() -> bool {
    true
}

fn default_consumers() -> u32 {
    num_cpus::get() as u32
}

fn default_spin_strategy() -> String {
    "yield".into()
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            capacity: default_capacity(),
            require_power_of_two: default_true(),
            num_consumers: default_consumers(),
            full_queue_strategy: default_spin_strategy(),
        }
    }
}

/// Memory allocation configuration.
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct MemoryConfig {
    /// Arena allocator chunk size (bytes).
    #[validate(range(min = 4096, max = 1048576))]
    pub arena_chunk_size: usize,

    /// Memory pool configuration for packet buffers.
    #[validate(nested)]
    pub packet_pool: PoolConfig,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            arena_chunk_size: 65536,
            packet_pool: PoolConfig::default(),
        }
    }
}

/// Memory pool configuration.
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct PoolConfig {
    /// Number of pre‑allocated packets.
    #[validate(range(min = 1024, max = 1048576))]
    pub initial_capacity: usize,

    /// Maximum packet size (bytes).
    #[validate(range(min = 64, max = 65536))]
    pub max_packet_size: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            initial_capacity: 8192,
            max_packet_size: 1514, // Standard Ethernet MTU.
        }
    }
}
