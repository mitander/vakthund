//! Defines the EventProcessor trait for processing network events.
use async_trait::async_trait;
use vakthund_core::events::network::NetworkEvent;
use vakthund_core::SimulationError;

/// Trait for processing network events.
#[async_trait]
pub trait EventProcessor: Send + Sync {
    /// Processes a single network event.
    async fn process(&self, event: &NetworkEvent) -> Result<(), SimulationError>;
}
