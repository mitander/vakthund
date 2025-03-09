use crate::engine::runtime_trait::SimulationDriver;
use async_trait::async_trait;
use parking_lot::Mutex;
use vakthund_core::{events::network::NetworkEvent, SimulationError};
use vakthund_simulator::Simulator;

pub struct DefaultSimulationDriver {
    simulator: Mutex<Simulator>,
    current_event: Mutex<usize>,
    max_events: usize,
}

impl DefaultSimulationDriver {
    pub fn new(simulator: Simulator, max_events: usize) -> Self {
        Self {
            simulator: Mutex::new(simulator),
            current_event: Mutex::new(0),
            max_events,
        }
    }
}

#[async_trait]
impl SimulationDriver for DefaultSimulationDriver {
    async fn next_event(&self) -> Result<Option<NetworkEvent>, SimulationError> {
        // With parking_lot, lock() returns the guard directly without Result
        let mut current = self.current_event.lock();

        if *current >= self.max_events {
            return Ok(None);
        }

        let event_id = *current;
        *current += 1;

        // Same with simulator - no Result to unwrap
        let mut simulator = self.simulator.lock();

        let event = simulator.simulate_event(event_id);

        Ok(event)
    }
}
