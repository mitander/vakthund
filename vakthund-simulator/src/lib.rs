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

use rand::Rng;
use std::sync::Arc;
use std::time::Duration;

use blake3::Hasher;
use bytes::Bytes;

use vakthund_core::alloc::arena::ArenaAllocator;
use vakthund_core::events::NetworkEvent;
use vakthund_core::network::jitter::{JitterModel, RandomJitterModel};
use vakthund_core::network::latency::{FixedLatencyModel, LatencyModel};
use vakthund_core::network::packet_loss::{NoPacketLossModel, PacketLossModel};
use vakthund_core::time::VirtualClock;

pub mod chaos;
pub mod cli;
pub mod config;
pub mod replay;

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
/// The Simulator ties together the virtual clock, memory allocation, network simulation models,
/// and optional chaos injection to simulate event processing.
pub struct Simulator {
    clock: VirtualClock,
    allocator: ArenaAllocator,
    latency_model: FixedLatencyModel,
    jitter_model: RandomJitterModel,
    packet_loss: Box<dyn PacketLossModel + Send>,
    pub state_hasher: Hasher,
    chaos_enabled: bool,
    event_bus: Option<Arc<vakthund_core::events::bus::EventBus>>,
}

impl Simulator {
    /// Creates a new Simulator instance.
    ///
    /// * `seed` - Seed value for the virtual clock and randomness.
    /// * `jhaos_enabled` - Enable fault injection.
    /// * `event_bus` - Optional shared event bus. If provided, simulated events will be enqueued.
    /// * `latency_ms` - Configurable latency
    /// * `jitter` - Configurable jitter
    pub fn new(
        seed: u64,
        chaos_enabled: bool,
        latency_ms: u64,
        jitter_ms: u64,
        event_bus: Option<Arc<vakthund_core::events::bus::EventBus>>,
    ) -> Self {
        Self {
            clock: VirtualClock::new(seed),
            allocator: ArenaAllocator::new(),
            latency_model: FixedLatencyModel::new(latency_ms),
            jitter_model: RandomJitterModel::new(jitter_ms),
            packet_loss: Box::new(NoPacketLossModel),
            state_hasher: Hasher::new(),
            chaos_enabled,
            event_bus,
        }
    }

    /// Sets the packet loss model to simulate network packet drops.
    pub fn set_packet_loss_model(&mut self, model: Box<dyn PacketLossModel + Send>) {
        self.packet_loss = model;
    }

    /// Simulates a single event.
    /// Instead of directly updating the state hasher, if an event bus is provided,
    /// the event is pushed onto the bus using a blocking push.
    pub fn simulate_event(&mut self, event_id: usize) -> Option<NetworkEvent> {
        let event_content = format!("Event {}", event_id);
        let event_str = self.allocator.allocate(event_content);

        // Simulate packet loss.
        if self.packet_loss.should_drop() {
            self.state_hasher.update(b"DROPPED");
            return None;
        }

        // Simulate network delay.
        let base_delay_ns = 100_000_000; // 100ms in nanoseconds.
        let base_delay = Duration::from_nanos(base_delay_ns);
        let delay = self.latency_model.apply_latency(base_delay);
        let jitter = self.jitter_model.apply_jitter(Duration::from_nanos(0));
        let total_delay = delay + jitter;
        self.clock.advance(total_delay.as_nanos() as u64);

        // Optionally inject a fault.
        if self.chaos_enabled && rand::rng().random_bool(0.1) {
            crate::chaos::inject_fault(event_str);
        }

        // Create a new NetworkEvent with the current clock time.
        let event = NetworkEvent::new(self.clock.now_ns(), Bytes::from(event_str.clone()));

        // If an event bus is provided, use blocking_push to enqueue the event.
        if let Some(ref bus) = self.event_bus {
            blocking_push(bus, event.clone());
        } else {
            // Fallback: update the state hasher directly.
            self.state_hasher.update(event_str.as_bytes());
        }

        Some(event)
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

/// A helper function that attempts a blocking push into the event bus.
/// It repeatedly calls `try_push` until the event is successfully enqueued.
fn blocking_push(event_bus: &Arc<vakthund_core::events::bus::EventBus>, event: NetworkEvent) {
    use vakthund_core::events::bus::EventError;
    loop {
        match event_bus.try_push(event.clone()) {
            Ok(_) => break,
            Err(EventError::QueueFull) => {
                // Yield to allow the event processor to catch up.
                std::thread::yield_now();
            }
            Err(e) => {
                panic!("Failed to push simulated event: {:?}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_runs() {
        // For testing simulation, we supply no event bus.
        let mut simulator = Simulator::new(42, true, 0, 0, None);
        let hash = simulator.run(5);
        assert!(!hash.is_empty());
    }
}
