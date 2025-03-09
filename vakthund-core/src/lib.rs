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
//! - `events`: Tokio-powered event bus with MPSC ringbuffer
//!
//! ### Future:
//! - ARM-optimized memory allocators
//! - Hardware timestamping support

pub mod alloc;
pub mod error;
pub mod events;

pub mod prelude {
    pub use crate::alloc::*;
    pub use crate::error::*;
    pub use crate::events::*;
}

pub use error::SimulationError;
