//! Runtime configuration parameters.

use serde::Deserialize;

use super::ConfigError;

/// Main runtime configuration
#[derive(Debug, Deserialize)]
pub struct RuntimeConfig {
    /// Event bus configuration
    pub event_bus: EventBusConfig,

    /// Telemetry settings
    pub telemetry: super::telemetry::TelemetryConfig,
}

/// Event bus specific configuration
#[derive(Debug, Deserialize)]
pub struct EventBusConfig {
    /// Capacity of the event bus (must be a power of two)
    pub capacity: usize,

    /// Enforce power-of-two capacity
    #[serde(default = "default_power_of_two")]
    pub require_power_of_two: bool,
}

fn default_power_of_two() -> bool {
    true
}

impl RuntimeConfig {
    /// Validates configuration parameters
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.event_bus.require_power_of_two && !self.event_bus.capacity.is_power_of_two() {
            return Err(ConfigError::Validation(format!(
                "Event bus capacity {} is not a power of two",
                self.event_bus.capacity
            )));
        }
        Ok(())
    }
}
