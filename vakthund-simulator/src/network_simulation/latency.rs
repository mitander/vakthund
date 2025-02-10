//! # Latency Models for Simulation
//!
//! Provides models to simulate network delay.
//!
//! ## Models:
//! - `FixedLatencyModel`: Always adds a fixed delay.
//! - `NoLatencyModel`: No delay (for baseline measurements).

use std::time::Duration;

/// Trait for network latency models.
pub trait LatencyModel: Send + Sync {
    /// Applies latency to a given duration.
    fn apply_latency(&self, base_duration: Duration) -> Duration;
}

/// A fixed latency model that always adds a constant delay.
#[derive(Debug, Clone, Copy)]
pub struct FixedLatencyModel {
    delay: Duration,
}

impl FixedLatencyModel {
    /// Creates a new fixed latency model.
    ///
    /// # Arguments
    /// * `latency_ms` - The fixed latency in milliseconds.
    pub fn new(latency_ms: u64) -> Self {
        Self {
            delay: Duration::from_millis(latency_ms),
        }
    }
}

impl LatencyModel for FixedLatencyModel {
    #[inline]
    fn apply_latency(&self, base_duration: Duration) -> Duration {
        base_duration + self.delay
    }
}

/// A no‑latency model that leaves the duration unchanged.
#[derive(Debug, Clone, Copy)]
pub struct NoLatencyModel;

impl LatencyModel for NoLatencyModel {
    #[inline]
    fn apply_latency(&self, base_duration: Duration) -> Duration {
        base_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_fixed_latency_model() {
        let model = FixedLatencyModel::new(100);
        let base = Duration::from_millis(50);
        let result = model.apply_latency(base);
        assert_eq!(result, Duration::from_millis(150));
    }

    #[test]
    fn test_no_latency_model() {
        let model = NoLatencyModel;
        let base = Duration::from_millis(50);
        let result = model.apply_latency(base);
        assert_eq!(result, base);
    }
}
