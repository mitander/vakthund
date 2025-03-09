//! Defines the VakthundRuntime trait for managing different runtime environments.

use async_trait::async_trait;
use vakthund_core::events::network::NetworkEvent;
use vakthund_core::SimulationError;

#[async_trait]
pub trait VakthundRuntime: Send + Sync {
    /// Starts the runtime environment.
    async fn start(&self) -> Result<(), SimulationError>;

    /// Stops the runtime environment.
    async fn stop(&self) -> Result<(), SimulationError>;

    /// Processes a single network event.
    async fn process_event(&self, event: &NetworkEvent) -> Result<(), SimulationError>;

    // Add additional traits for metric collection.
}

#[async_trait]
pub trait SimulationDriver: Send + Sync {
    /// Retrieves the next event from the simulation.
    async fn next_event(&self) -> Result<Option<NetworkEvent>, SimulationError>;
}
