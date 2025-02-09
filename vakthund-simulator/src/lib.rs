// vakthund-simulator/src/lib.rs

/*!
# Vakthund Simulator

The Vakthund Simulator provides a deterministic simulation and replay engine for the Vakthund IDPS.
It leverages virtual time, network condition emulation (latency, jitter, and packet loss), and optional fault injection
to enable reproducible testing and debugging of intrusion detection/prevention scenarios.

## Key Components:
- **Virtual Clock:** Simulated time with nanosecond precision.
- **Memory Allocator:** Arena allocator for zero‑copy event buffers.
- **Network Models:** Fixed latency, random jitter, and packet loss simulation.
- **Chaos Engine:** Optional fault injection for robustness testing.
- **Replay Engine:** Deterministic replay of recorded scenarios.
*/

use blake3::Hasher;
use rand::Rng;
use std::time::Duration;
use vakthund_core::alloc::arena::ArenaAllocator;
use vakthund_core::network::jitter::{JitterModel, RandomJitterModel};
use vakthund_core::network::latency::{FixedLatencyModel, LatencyModel};
use vakthund_core::network::packet_loss::{NoPacketLossModel, PacketLossModel};
use vakthund_core::time::VirtualClock;

pub mod chaos;
pub mod cli;
pub mod replay; // Contains ReplayEngine, Scenario and related definitions.

/// The Simulator ties together the virtual clock, memory allocation, network simulation models, and optional chaos injection to simulate event processing.
///
/// # Fields
/// - `clock`: Virtual clock for deterministic simulation.
/// - `allocator`: Arena allocator for zero‑copy event buffers.
/// - `latency_model`: Fixed latency model to simulate network delay.
/// - `jitter_model`: Random jitter model for simulating latency variability.
/// - `packet_loss`: Packet loss model for simulating network packet drops.
/// - `state_hasher`: BLAKE3 hasher to record deterministic system state.
/// - `chaos_enabled`: Flag to enable fault injection.
pub struct Simulator {
    clock: VirtualClock,
    allocator: ArenaAllocator,
    latency_model: FixedLatencyModel,
    jitter_model: RandomJitterModel,
    packet_loss: Box<dyn PacketLossModel + Send>,
    state_hasher: Hasher,
    chaos_enabled: bool,
}

impl Simulator {
    /// Creates a new Simulator instance.
    ///
    /// * `seed` - Seed value for the virtual clock and randomness.
    /// * `chaos_enabled` - Enable fault injection.
    pub fn new(seed: u64, chaos_enabled: bool) -> Self {
        Self {
            clock: VirtualClock::new(seed),
            allocator: ArenaAllocator::new(),
            // Fixed latency of 100ms per event.
            latency_model: FixedLatencyModel::new(100),
            // Random jitter up to 10ms.
            jitter_model: RandomJitterModel::new(10),
            // Default to no packet loss. This can be replaced via `set_packet_loss_model`.
            packet_loss: Box::new(NoPacketLossModel),
            state_hasher: Hasher::new(),
            chaos_enabled,
        }
    }

    /// Sets the packet loss model to simulate network packet drops.
    pub fn set_packet_loss_model(&mut self, model: Box<dyn PacketLossModel + Send>) {
        self.packet_loss = model;
    }

    /// Simulates a single event.
    /// Allocates an event, applies network delay (latency + jitter), possibly drops the event due to packet loss,
    /// optionally injects a fault, and then updates the system state.
    fn simulate_event(&mut self, event_id: usize) {
        // Allocate an event using the arena allocator.
        let event_content = format!("Event {}", event_id);
        let event_ref = self.allocator.allocate(event_content);

        // Simulate packet loss: if the packet is dropped, update state (optionally mark it) and return.
        if self.packet_loss.should_drop() {
            self.state_hasher.update(b"DROPPED");
            return;
        }

        // Simulate network delay: base delay (100ms) plus jitter.
        let base_delay_ns = 100_000_000; // 100ms in nanoseconds.
        let base_delay = Duration::from_nanos(base_delay_ns);
        let delay = self.latency_model.apply_latency(base_delay);
        let jitter = self.jitter_model.apply_jitter(Duration::from_nanos(0));
        let total_delay = delay + jitter;
        self.clock.advance(total_delay.as_nanos() as u64);

        // Optionally inject a fault into the event.
        if self.chaos_enabled && rand::rng().random_bool(0.1) {
            crate::chaos::inject_fault(event_ref);
        }

        // Update the state hasher with the event content.
        self.state_hasher.update(event_ref.as_bytes());
    }

    /// Runs the simulation for a given number of events.
    /// Returns the final state hash as a hex string.
    pub fn run(&mut self, event_count: usize) -> String {
        for event_id in 0..event_count {
            self.simulate_event(event_id);
        }
        hex::encode(self.state_hasher.finalize().as_bytes())
    }
}

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
