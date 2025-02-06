//! ## vakthund-core::network::packet_loss
//! **Packet loss models for network simulation**
//!
//! This module implements models for simulating packet loss in network scenarios.
//!
//! ### Models:
//! - Probabilistic Packet Loss: Packets are dropped with a given probability.
//! - Burst Packet Loss: Simulate bursts of packet loss.
//! - State-Based Packet Loss: Packet loss based on network state.
//!
//! ### Future:
//! - Advanced packet loss models (e.g., Gilbert-Elliot).
//! - Packet loss based on simulated network congestion.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::sync::Mutex;

/// Trait for packet loss models.
pub trait PacketLossModel: Send + Sync {
    /// Determines if a packet should be dropped based on the model.
    fn should_drop(&mut self) -> bool;
}

/// Probabilistic packet loss model.
#[derive(Debug)]
pub struct ProbabilisticLossModel {
    drop_probability: f64, // Probability of packet drop (0.0 to 1.0)
    rng: Mutex<SmallRng>,  // Use SmallRng for deterministic, thread-safe randomness
}

impl ProbabilisticLossModel {
    /// Creates a new probabilistic packet loss model.
    ///
    /// # Panics
    ///
    /// Panics if `drop_probability` is not within the range [0.0, 1.0].
    pub fn new(drop_probability: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&drop_probability),
            "Drop probability must be between 0.0 and 1.0"
        );
        Self {
            drop_probability,
            // Initialize using from_entropy, which is seedable and does not require a mutable reference.
            rng: Mutex::new(SmallRng::from_rng(&mut rand::rng())),
        }
    }
}

impl PacketLossModel for ProbabilisticLossModel {
    fn should_drop(&mut self) -> bool {
        // Generate a boolean based on drop_probability.
        self.rng.lock().unwrap().random_bool(self.drop_probability)
    }
}

/// No-op packet loss model (no packet loss).
#[derive(Debug, Default)]
pub struct NoPacketLossModel;

impl PacketLossModel for NoPacketLossModel {
    fn should_drop(&mut self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probabilistic_loss_model() {
        let mut model = ProbabilisticLossModel::new(0.5); // 50% drop probability
        let mut drop_count = 0;
        let test_iterations = 10_000;

        for _ in 0..test_iterations {
            if model.should_drop() {
                drop_count += 1;
            }
        }

        let actual_probability = (drop_count as f64) / (test_iterations as f64);
        // Allow some tolerance (around 0.5 with ±0.05 deviation)
        assert!((actual_probability - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_no_packet_loss_model() {
        let mut model = NoPacketLossModel::default();
        for _ in 0..100 {
            assert_eq!(model.should_drop(), false);
        }
    }

    #[test]
    #[should_panic]
    fn test_probabilistic_loss_model_invalid_probability() {
        ProbabilisticLossModel::new(1.5); // Should panic
    }
}
