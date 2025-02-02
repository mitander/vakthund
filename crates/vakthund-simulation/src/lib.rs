//! Vakthund Simulation Engine
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Provides a deterministic simulation engine and storage for simulation events.
//! Designed for reproducible testing and deterministic replay.

pub mod simulation_engine;
pub mod storage;

pub use simulation_engine::{compute_event_hash, run_simulation, SimEvent};
pub use storage::InMemoryStorage;
