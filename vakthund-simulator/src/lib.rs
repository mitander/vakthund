//! # Vakthund Simulator
//!
//! Provides a deterministic simulation and replay engine that uses core
//! components (virtual clock, arena allocators, network condition models)
//! to process a stream of simulated events. It also supports optional chaos
//! (fault injection) and replay of recorded scenarios.

use blake3::Hasher;
use std::time::Duration;

use vakthund_core::alloc::arena::ArenaAllocator;
use vakthund_core::network::jitter::{JitterModel, RandomJitterModel};
use vakthund_core::network::latency::{FixedLatencyModel, LatencyModel};
use vakthund_core::time::VirtualClock;

use rand::Rng;

pub mod chaos;
pub mod cli;
pub mod replay;

/// The Simulator ties together the virtual clock, memory allocation, network
/// models, and chaos injection to simulate event processing.
pub struct Simulator {
    clock: VirtualClock,
    allocator: ArenaAllocator,
    latency_model: FixedLatencyModel,
    jitter_model: RandomJitterModel,
    state_hasher: Hasher,
    chaos_enabled: bool,
}

impl Simulator {
    /// Create a new Simulator with a given seed and chaos flag.
    pub fn new(seed: u64, chaos_enabled: bool) -> Self {
        Self {
            clock: VirtualClock::new(seed),
            allocator: ArenaAllocator::new(),
            // Create a fixed latency model that adds 100ms per event.
            latency_model: FixedLatencyModel::new(100),
            // Create a jitter model that can add up to 10ms jitter.
            jitter_model: RandomJitterModel::new(10),
            state_hasher: Hasher::new(),
            chaos_enabled,
        }
    }

    /// Run the simulation for a given number of events. For each event:
    /// - Allocate an event buffer from the arena (zero‑copy)
    /// - Simulate network delay via latency + jitter and update the virtual clock
    /// - Optionally inject a fault via the chaos module
    /// - Update a state hasher (using blake3) for reproducibility
    ///
    /// Returns the final state hash.
    #[inline] // Inlining for performance
    pub fn run(&mut self, event_count: usize) -> String {
        for event_id in 0..event_count {
            self.simulate_event(event_id);
        }
        hex::encode(self.state_hasher.finalize().as_bytes())
    }

    #[inline]
    fn simulate_event(&mut self, event_id: usize) {
        // Allocate an event using the arena allocator.
        // (Here we simulate an event as a String; in a real system this might be a packet buffer.)
        let event_content = format!("Event {}", event_id);
        let event_ref = self.allocator.allocate(event_content);

        // Simulate network delay.
        let base_delay_ns = 100_000_000; // 100ms in nanoseconds
        let base_delay = Duration::from_nanos(base_delay_ns);
        let delay = self.latency_model.apply_latency(base_delay);
        let jitter = self.jitter_model.apply_jitter(Duration::from_nanos(0));
        let total_delay = delay + jitter;
        self.clock.advance(total_delay.as_nanos() as u64);

        // Optionally inject a fault into the event.
        if self.chaos_enabled && rand::rng().random_bool(0.1) {
            // Replaced deprecated thread_rng with rng()
            // Replaced deprecated gen_bool
            crate::chaos::inject_fault(event_ref);
        }

        // Update the state hasher using the event content.
        self.state_hasher.update(event_ref.as_bytes());
    }
}

/// Public entry point to run the simulation.
/// This function parses CLI arguments and runs the simulation engine.
// Note: `simulate` function is no longer needed as the logic is moved to `vakthund-cli/src/commands.rs`
// pub fn simulate() {
//     let args = SimArgs::parse();
//     let seed = args.seed.unwrap_or(42);
//     let mut simulator = Simulator::new(seed, args.chaos);
//     let state_hash = simulator.run(args.events);
//     println!("Simulation complete. State hash: {}", state_hash);
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_runs() {
        let mut simulator = Simulator::new(42, true);
        let hash = simulator.run(5);
        assert!(!hash.is_empty());
    }
}
