//! ## vakthund-core::network
//! **Network condition models (latency/jitter/packet loss)**
//!
//! ### Expectations (Production):
//! - Deterministic and configurable network conditions for simulation
//! - Low overhead condition application
//! - Support for various network impairments
//!
//! ### Key Submodules:
//! - `latency/`: Latency models (fixed, variable, distribution-based)
//! - `jitter/`: Jitter introduction and simulation
//! - `packet_loss/`: Probabilistic packet loss models
//!
//! ### Future:
//! - Real-world network condition capture and replay
//! - Integration with network emulation tools (e.g., `netem`)

pub mod jitter;
pub mod latency;
pub mod packet_loss;
