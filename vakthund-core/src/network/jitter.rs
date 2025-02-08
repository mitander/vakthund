//! ## vakthund-core::network::jitter
//! **Jitter simulation for network conditions**
//!
//! This module provides mechanisms to introduce jitter (variations in latency)
//! into network simulations.
//!
//! ### Features:
//! - Jitter based on statistical distributions.
//! - Configurable jitter magnitude and frequency.
//! - Realistic jitter patterns.
//!
//! ### Future:
//! - Support for different jitter distribution models.
//! - Correlation between latency and jitter.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::sync::Mutex;
use std::time::Duration;

/// Trait for jitter models.
pub trait JitterModel: Send + Sync {
    /// Applies jitter to a given duration, returning the jittered duration.
    fn apply_jitter(&mut self, duration: Duration) -> Duration;
}

/// A random jitter model using a simple uniform distribution.
#[derive(Debug)]
pub struct RandomJitterModel {
    magnitude_ms: u64,    // Maximum jitter magnitude in milliseconds
    rng: Mutex<SmallRng>, // Use SmallRng for deterministic, thread-safe randomness
}

impl RandomJitterModel {
    /// Creates a new random jitter model.
    pub fn new(magnitude_ms: u64) -> Self {
        Self {
            magnitude_ms,
            rng: Mutex::new(SmallRng::from_rng(&mut rand::rng())),
        }
    }
}

impl JitterModel for RandomJitterModel {
    fn apply_jitter(&mut self, duration: Duration) -> Duration {
        // Use gen_range (allowing deprecated warnings if necessary)
        #[allow(deprecated)]
        let jitter_ms = self.rng.lock().unwrap().gen_range(0..=self.magnitude_ms);
        duration + Duration::from_millis(jitter_ms)
    }
}

/// No-op jitter model (no jitter).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoJitterModel;

impl JitterModel for NoJitterModel {
    fn apply_jitter(&mut self, duration: Duration) -> Duration {
        duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_random_jitter_model() {
        let mut model = RandomJitterModel::new(50); // Max 50ms jitter
        let initial_duration = Duration::from_millis(100);
        let jittered_duration = model.apply_jitter(initial_duration);

        // The jittered duration should be at least the original duration...
        assert!(jittered_duration >= initial_duration);
        // ... and at most initial + 50ms.
        assert!(jittered_duration <= initial_duration + Duration::from_millis(50));
    }

    #[test]
    fn test_no_jitter_model() {
        let mut model = NoJitterModel::default();
        let initial_duration = Duration::from_millis(100);
        let jittered_duration = model.apply_jitter(initial_duration);
        assert_eq!(jittered_duration, initial_duration);
    }
}
