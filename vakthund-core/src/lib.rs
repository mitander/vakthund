//! # vakthund-core
//!
//! Foundation layer for event processing and resource management.
//! Built with safety, performance, and maintainability as primary design constraints.
//! # Vakthund IDPS Core Platform
//!
//! A deterministic-first intrusion detection/prevention system for IoT networks.
//! Built with safety, performance, and maintainability as primary design constraints.
//!
//! ### Expectations (Production):
//! - <2ms startup time for embedded deployments
//! - Zero heap allocations in packet processing paths
//! - Lock-free synchronization primitives
//!
//! ### Key Submodules:
//! - `alloc`: Memory pools and arena allocators using `bumpalo`
//! - `event_bus`: Tokio-powered event bus with MPSC ringbuffer
//! - `sim`: Deterministic simulation core with virtual clock
//! - `network`: Network condition models (latency/jitter/packet loss)
//! - `time`: `VirtualClock` using atomic counters + scheduler
//!
//! ### Future:
//! - ARM-optimized memory allocators
//! - Hardware timestamping support

pub mod alloc;
pub mod event_bus;
pub mod network;
pub mod sim;
pub mod time;

pub mod prelude {
    pub use crate::alloc::*;
    pub use crate::event_bus::*;
    pub use crate::network::*;
    pub use crate::sim::*;
    pub use crate::time::*;
}
