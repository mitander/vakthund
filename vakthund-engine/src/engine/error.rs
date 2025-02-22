use thiserror::Error;
use tokio::task::JoinError;
use vakthund_config::ConfigError;

#[derive(Debug, Error)]
pub enum SimulationError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Event processing error: {0}")]
    Processing(String),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for SimulationError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        SimulationError::Processing(err.to_string())
    }
}

impl From<JoinError> for SimulationError {
    fn from(err: JoinError) -> Self {
        SimulationError::Processing(err.to_string())
    }
}
