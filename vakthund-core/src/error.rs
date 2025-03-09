use thiserror::Error;

#[derive(Debug, Error)]
pub enum SimulationError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Event processing error: {0}")]
    Processing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
