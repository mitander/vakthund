//! # Virtual Clock for Simulation
//!
//! A deterministic clock used exclusively in simulation and replay mode.
//! This implementation was formerly in the core crate but has been moved here.
//!
//! ## Expectations:
//! - Nanosecond resolution
//! - Seedable and deterministic
//! - Lock‑free operations

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A simple virtual clock that advances in nanoseconds.
#[derive(Clone)]
pub struct VirtualClock {
    // A shared atomic counter representing the current simulation time in nanoseconds.
    offset: Arc<AtomicU64>,
}

impl VirtualClock {
    /// Creates a new virtual clock with the given seed (starting time).
    pub fn new(seed: u64) -> Self {
        Self {
            offset: Arc::new(AtomicU64::new(seed)),
        }
    }

    /// Returns the current virtual time in nanoseconds.
    #[inline]
    pub fn now_ns(&self) -> u64 {
        self.offset.load(Ordering::Acquire)
    }

    /// Advances the virtual clock by the given number of nanoseconds.
    #[inline]
    pub fn advance(&self, ns: u64) {
        self.offset.fetch_add(ns, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_initial_value() {
        let clock = VirtualClock::new(100);
        assert_eq!(clock.now_ns(), 100);
    }

    #[test]
    fn test_clock_advance() {
        let clock = VirtualClock::new(0);
        clock.advance(500);
        assert_eq!(clock.now_ns(), 500);
        clock.advance(250);
        assert_eq!(clock.now_ns(), 750);
    }
}
