//! # Jitter Models for Simulation
//!
//! Provides models to simulate network jitter.
//!
//! ## Models:
//! - `RandomJitterModel`: Applies a random jitter from 0 up to a maximum magnitude.
//! - `NoJitterModel`: Applies no jitter.

use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;
use std::sync::Mutex;
use std::time::Duration;

/// Trait for jitter models.
pub trait JitterModel: Send + Sync {
    /// Applies jitter to the provided duration.
    fn apply_jitter(&mut self, base_duration: Duration) -> Duration;
}

/// A random jitter model that adds a uniform random delay (in milliseconds).
#[derive(Debug)]
pub struct RandomJitterModel {
    /// Maximum jitter magnitude in milliseconds.
    magnitude_ms: u64,
    /// Mutex-protected small RNG for thread safety.
    rng: Mutex<SmallRng>,
}

impl RandomJitterModel {
    /// Creates a new random jitter model.
    ///
    /// # Arguments
    /// * `magnitude_ms` - The maximum jitter (in ms) that can be added.
    pub fn new(magnitude_ms: u64) -> Self {
        // Seed from system entropy. In practice you might want a seed parameter.
        Self {
            magnitude_ms,
            rng: Mutex::new(SmallRng::from_rng(&mut rand::rng())),
        }
    }
}

impl JitterModel for RandomJitterModel {
    #[inline]
    fn apply_jitter(&mut self, base_duration: Duration) -> Duration {
        let added_ms = self.rng.lock().unwrap().random_range(0..=self.magnitude_ms);
        base_duration + Duration::from_millis(added_ms)
    }
}

/// A no‑jitter model that leaves the duration unchanged.
#[derive(Debug, Clone, Copy)]
pub struct NoJitterModel;

impl JitterModel for NoJitterModel {
    #[inline]
    fn apply_jitter(&mut self, base_duration: Duration) -> Duration {
        base_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_random_jitter_model_range() {
        let mut model = RandomJitterModel::new(50);
        let base = Duration::from_millis(100);
        let jittered = model.apply_jitter(base);
        // Should be at least base and no more than base + 50 ms.
        assert!(jittered >= base);
        assert!(jittered <= base + Duration::from_millis(50));
    }

    #[test]
    fn test_no_jitter_model() {
        let mut model = NoJitterModel;
        let base = Duration::from_millis(100);
        let jittered = model.apply_jitter(base);
        assert_eq!(jittered, base);
    }
}
