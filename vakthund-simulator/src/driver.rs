//! Defines the SimulationDriver trait for driving different simulation strategies.

use async_trait::async_trait;

use vakthund_core::events::network::NetworkEvent;
use vakthund_core::SimulationError;

#[async_trait]
pub trait SimulationDriver: Send + Sync {
    /// Runs the simulation and returns the final state hash.
    async fn run(&mut self) -> Result<String, SimulationError>;

    /// Retrieves the next event from the simulation.
    async fn next_event(&mut self) -> Result<Option<NetworkEvent>, SimulationError>;
}
