//! ## vakthund-core::network::latency
//! **Latency models for network simulation**
//!
//! This module provides different models for simulating network latency.
//!
//! ### Available Models:
//! - Fixed Latency: Constant latency value.
//! - Variable Latency: Latency that varies based on a pattern or distribution.
//! - Distribution-Based Latency: Latency sampled from a statistical distribution.
//!
//! ### Future:
//! - Support for custom latency distributions.
//! - Integration with real-world latency measurements.

use std::time::Duration;

/// Trait for latency models.
pub trait LatencyModel: Send + Sync {
    /// Applies the latency model to a given duration.
    fn apply_latency(&self, duration: Duration) -> Duration;
}

/// Fixed latency model.
#[derive(Debug, Clone, Copy)]
pub struct FixedLatencyModel {
    latency: Duration,
}

impl FixedLatencyModel {
    /// Creates a new fixed latency model.
    pub fn new(latency_ms: u64) -> Self {
        Self {
            latency: Duration::from_millis(latency_ms),
        }
    }
}

impl LatencyModel for FixedLatencyModel {
    fn apply_latency(&self, duration: Duration) -> Duration {
        duration + self.latency
    }
}

/// No-op latency model (for baseline or no latency simulation).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLatencyModel;

impl LatencyModel for NoLatencyModel {
    fn apply_latency(&self, duration: Duration) -> Duration {
        duration // No latency added
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_latency_model() {
        let model = FixedLatencyModel::new(100); // 100ms fixed latency
        let initial_duration = Duration::from_millis(50);
        let delayed_duration = model.apply_latency(initial_duration);
        assert_eq!(delayed_duration, Duration::from_millis(150)); // 50 + 100 = 150
    }

    #[test]
    fn test_no_latency_model() {
        let model = NoLatencyModel::default();
        let initial_duration = Duration::from_millis(50);
        let delayed_duration = model.apply_latency(initial_duration);
        assert_eq!(delayed_duration, Duration::from_millis(50)); // No change
    }
}
