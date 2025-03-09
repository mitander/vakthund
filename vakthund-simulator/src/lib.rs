//! # Vakthund Simulator
//!
//! Provides a deterministic simulation engine for Vakthund.
//!
//! This module uses simulation‑only components (virtual clock and network simulation)
//! and shares the production event bus and event type from `vakthund-core`.
pub mod chaos;
pub mod cli;
pub mod network_simulation;
pub mod replay;
pub mod virtual_clock;

use blake3::Hasher;
use bytes::Bytes;
use std::sync::Arc;
use std::time::Duration;

pub use network_simulation::jitter::{JitterModel, RandomJitterModel};
pub use network_simulation::latency::{FixedLatencyModel, LatencyModel};
pub use network_simulation::packet_loss::{
    NoPacketLossModel, PacketLossModel, ProbabilisticLossModel,
};

pub use replay::{Scenario, ScenarioEvent};
pub use vakthund_config::SimulatorConfig;
pub use virtual_clock::VirtualClock;

/// The Simulator ties together simulation‑specific components.
pub struct Simulator {
    event_log: Vec<ScenarioEvent>,
    clock: VirtualClock,
    latency_model: FixedLatencyModel,
    jitter_model: RandomJitterModel,
    packet_loss: Box<dyn PacketLossModel + Send>,
    state_hasher: Hasher,
    chaos_enabled: bool,
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
            event_log: Vec::new(),
            clock: VirtualClock::new(seed),
            latency_model: FixedLatencyModel::new(latency_ms),
            jitter_model: RandomJitterModel::new(jitter_ms),
            packet_loss: Box::new(NoPacketLossModel),
            state_hasher: Hasher::new(),
            chaos_enabled,
            event_bus,
        }
    }

    pub fn apply_scenario_event(&mut self, event: ScenarioEvent) {
        match event {
            ScenarioEvent::NetworkDelay(delay_ns) => {
                self.clock.advance(delay_ns);
            }
            ScenarioEvent::PacketLoss(probability) => {
                self.set_packet_loss_model(Box::new(ProbabilisticLossModel::new(probability)));
            }
            ScenarioEvent::NetworkEvent { delay_ns, event } => {
                self.clock.advance(delay_ns);
                if let Some(ref bus) = self.event_bus {
                    bus.send_blocking(event.clone());
                }
            }
            // Add handling for other scenario event types
            _ => {}
        }
    }

    // Add this new constructor
    pub fn from_scenario(scenario: &Scenario) -> Self {
        Self::new(
            scenario.seed,
            scenario.config.chaos.fault_probability > 0.0,
            scenario.config.network.latency_ms,
            scenario.config.network.jitter_ms,
            None,
        )
    }

    // Add this method
    pub async fn replay_events(
        &mut self,
        events: Vec<ScenarioEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for event in events {
            self.apply_scenario_event(event);
        }
        Ok(())
    }

    // Add event tracking
    pub fn get_recorded_events(&self) -> Vec<ScenarioEvent> {
        self.event_log.clone()
    }

    // TODO:
    // pub fn state_hash(&self) -> String {
    //     let mut hasher = blake3::Hasher::new();
    //     hasher.update(&self.event_bus_state());
    //     hasher.update(&self.detection_engine_state());
    //     hex::encode(hasher.finalize().as_bytes())
    // }

    // Finalize and consume the hasher
    pub fn finalize_hash(&self) -> String {
        // Finalize consumes the hasher; the output is an owned value.
        let output = self.state_hasher.finalize();
        hex::encode(output.as_bytes())
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
            bus.send_blocking(event.clone());
        }

        // Update state hash.
        self.state_hasher.update(event_content.as_bytes());

        self.event_log.push(ScenarioEvent::NetworkEvent {
            delay_ns: total_delay.as_nanos() as u64,
            event: event.clone(),
        });
        Some(event)
    }

    /// Runs the simulation for a fixed number of events.
    /// Returns the final state hash as a hex string.
    pub fn run(&mut self, event_count: usize) -> String {
        for event_id in 0..event_count {
            let _ = self.simulate_event(event_id);
        }
        self.finalize_hash()
    }
}
