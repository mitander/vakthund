//! # Vakthund Simulator
//!
//! Provides a deterministic simulation engine for Vakthund.
//!
//! This module uses simulation‑only components (virtual clock and network simulation)
//! and shares the production event bus and event type from `vakthund-core`.

use blake3::Hasher;
use bytes::Bytes;
use std::sync::Arc;
use std::time::Duration;

// Import our simulation modules:
pub mod virtual_clock;
pub use virtual_clock::VirtualClock;

pub mod network_simulation;
pub use network_simulation::jitter::{JitterModel, RandomJitterModel};
pub use network_simulation::latency::{FixedLatencyModel, LatencyModel};
pub use network_simulation::packet_loss::{NoPacketLossModel, PacketLossModel};
pub use vakthund_config::SimulatorConfig;

pub mod chaos;
pub mod cli;
pub mod replay;

/// The Simulator ties together simulation‐specific components.
pub struct Simulator {
    clock: VirtualClock,
    latency_model: FixedLatencyModel,
    jitter_model: RandomJitterModel,
    packet_loss: Box<dyn PacketLossModel + Send>,
    pub state_hasher: Hasher,
    chaos_enabled: bool,
    /// Shared event bus from production (using vakthund-core’s bus).
    event_bus: Option<Arc<vakthund_core::events::bus::EventBus>>,
}

impl Simulator {
    /// Creates a new Simulator.
    ///
    /// # Arguments
    /// * `seed` - Seed for the virtual clock.
    /// * `chaos_enabled` - Enable fault injection.
    /// * `latency_ms` - Fixed network latency (ms).
    /// * `jitter_ms` - Maximum jitter (ms).
    /// * `event_bus` - Optional shared event bus.
    pub fn new(
        seed: u64,
        chaos_enabled: bool,
        latency_ms: u64,
        jitter_ms: u64,
        event_bus: Option<Arc<vakthund_core::events::bus::EventBus>>,
    ) -> Self {
        Self {
            clock: VirtualClock::new(seed),
            latency_model: FixedLatencyModel::new(latency_ms),
            jitter_model: RandomJitterModel::new(jitter_ms),
            packet_loss: Box::new(NoPacketLossModel),
            state_hasher: Hasher::new(),
            chaos_enabled,
            event_bus,
        }
    }

    /// Allows replacing the packet loss model.
    pub fn set_packet_loss_model(&mut self, model: Box<dyn PacketLossModel + Send>) {
        self.packet_loss = model;
    }

    /// Simulates a single event.
    /// Returns an event of type `vakthund_core::events::network::NetworkEvent`.
    pub fn simulate_event(
        &mut self,
        event_id: usize,
    ) -> Option<vakthund_core::events::network::NetworkEvent> {
        let mut event_content = format!("Event {}", event_id);

        // Simulate packet loss.
        if self.packet_loss.should_drop() {
            self.state_hasher.update(b"DROPPED");
            return None;
        }

        // Simulate network delay.
        let base_delay = Duration::from_nanos(100_000_000); // 100ms in ns
        let delay = self.latency_model.apply_latency(base_delay);
        let jitter = self.jitter_model.apply_jitter(Duration::from_nanos(0));
        let total_delay = delay + jitter;
        self.clock.advance(total_delay.as_nanos() as u64);

        // Optionally inject chaos.
        if self.chaos_enabled && rand::random::<f64>() < 0.1 {
            chaos::inject_fault(&mut event_content);
        }

        // Create a NetworkEvent from vakthund-core.
        let event = vakthund_core::events::network::NetworkEvent::new(
            self.clock.now_ns(),
            Bytes::from(event_content.clone()),
        );

        // If an event bus is provided, push the event.
        if let Some(ref bus) = self.event_bus {
            blocking_push(bus, event.clone());
        }

        // Update state hash.
        self.state_hasher.update(event_content.as_bytes());
        Some(event)
    }

    /// Runs the simulation for a fixed number of events.
    /// Returns the final state hash as a hex string.
    pub fn run(&mut self, event_count: usize) -> String {
        for event_id in 0..event_count {
            let _ = self.simulate_event(event_id);
        }
        hex::encode(self.state_hasher.finalize().as_bytes())
    }
}

/// Helper function to perform a blocking push on the event bus.
fn blocking_push(
    event_bus: &Arc<vakthund_core::events::bus::EventBus>,
    event: vakthund_core::events::network::NetworkEvent,
) {
    use vakthund_core::events::bus::EventError;
    loop {
        match event_bus.try_push(event.clone()) {
            Ok(_) => break,
            Err(EventError::QueueFull) => {
                std::thread::yield_now();
            }
            Err(e) => panic!("Failed to push simulated event: {:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vakthund_core::events::bus::EventBus;

    #[test]
    fn test_simulate_event_without_bus() {
        let mut simulator = Simulator::new(42, false, 100, 20, None);
        let event = simulator.simulate_event(1);
        assert!(event.is_some());
        let e = event.unwrap();
        // Ensure the timestamp is greater than the seed.
        assert!(e.timestamp > 42);
    }

    #[test]
    fn test_simulate_event_with_bus() {
        let bus = EventBus::with_capacity(8).unwrap();
        let bus_arc = Arc::new(bus);
        let mut simulator = Simulator::new(42, false, 100, 20, Some(Arc::clone(&bus_arc)));
        let event = simulator.simulate_event(2);
        assert!(event.is_some());
        // After pushing, the bus should have an event.
        let popped = bus_arc.try_pop();
        assert!(popped.is_some());
    }

    #[test]
    fn test_run_simulation() {
        let mut simulator = Simulator::new(42, false, 100, 20, None);
        let final_hash = simulator.run(10);
        // Expect a nonempty hash string.
        assert!(!final_hash.is_empty());
    }
}
