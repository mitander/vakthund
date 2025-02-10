//! Simulator configuration.
//!
//! **Deprecated:** Simulation‑specific configuration is now handled in the vakthund‑simulator crate.
//! This module remains only as a stub for backward compatibility.

use serde::{Deserialize, Serialize};
use validator::{self, Validate};

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct SimulatorConfig {
    /// Seed for deterministic simulation.
    pub seed: u64,
    /// Number of events to simulate.
    pub event_count: usize,
    /// Chaos configuration.
    #[serde(default)]
    pub chaos: ChaosConfig,
    /// Network emulation parameters.
    #[serde(default)]
    pub network: NetworkModelConfig,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            event_count: 10000,
            chaos: ChaosConfig {
                fault_probability: 0.0,
            },
            network: NetworkModelConfig {
                latency_ms: 0,
                jitter_ms: 0,
            },
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Validate, Clone)]
pub struct ChaosConfig {
    /// Fault injection probability (0.0 to 1.0).
    pub fault_probability: f64,
}

#[derive(Default, Debug, Serialize, Deserialize, Validate, Clone)]
pub struct NetworkModelConfig {
    /// Fixed latency in milliseconds.
    pub latency_ms: u64,
    /// Maximum jitter in milliseconds.
    pub jitter_ms: u64,
}
