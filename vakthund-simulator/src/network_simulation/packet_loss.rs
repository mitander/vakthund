//! # Packet Loss Models for Simulation
//!
//! Provides models to simulate packet loss.
//!
//! ## Models:
//! - `ProbabilisticLossModel`: Drops packets with a given probability.
//! - `NoPacketLossModel`: Never drops packets.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::sync::Mutex;

/// Trait for packet loss models.
pub trait PacketLossModel: Send + Sync {
    /// Determines if a packet should be dropped.
    fn should_drop(&mut self) -> bool;
}

/// A probabilistic loss model that drops packets with a configurable probability.
#[derive(Debug)]
pub struct ProbabilisticLossModel {
    /// Drop probability (0.0 to 1.0)
    drop_probability: f64,
    /// Mutex-protected RNG for generating random booleans.
    rng: Mutex<SmallRng>,
}

impl ProbabilisticLossModel {
    /// Creates a new probabilistic loss model.
    ///
    /// # Panics
    /// Panics if `drop_probability` is not between 0.0 and 1.0.
    pub fn new(drop_probability: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&drop_probability),
            "Drop probability must be between 0.0 and 1.0"
        );
        Self {
            drop_probability,
            rng: Mutex::new(SmallRng::from_rng(&mut rand::rng())),
        }
    }
}

impl PacketLossModel for ProbabilisticLossModel {
    #[inline]
    fn should_drop(&mut self) -> bool {
        self.rng.lock().unwrap().random_bool(self.drop_probability)
    }
}

/// A no‑packet‑loss model that never drops a packet.
#[derive(Debug)]
pub struct NoPacketLossModel;

impl PacketLossModel for NoPacketLossModel {
    #[inline]
    fn should_drop(&mut self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probabilistic_loss_model_probability() {
        let mut model = ProbabilisticLossModel::new(0.5);
        let iterations = 10_000;
        let mut drops = 0;
        for _ in 0..iterations {
            if model.should_drop() {
                drops += 1;
            }
        }
        let drop_rate = drops as f64 / iterations as f64;
        // Allow a tolerance of 5%
        assert!((drop_rate - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_no_packet_loss_model() {
        let mut model = NoPacketLossModel;
        for _ in 0..100 {
            assert!(!model.should_drop());
        }
    }
}
