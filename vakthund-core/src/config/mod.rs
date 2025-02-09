//! Configuration management with validation.
//!
//! Supports loading from YAML files using serde.

pub mod runtime;
pub mod telemetry;

pub use runtime::RuntimeConfig;
use std::path::Path;
pub use telemetry::TelemetryConfig;

use std::path::PathBuf;
use thiserror::Error;

/// Configuration-related error conditions
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid configuration: {0}")]
    Validation(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Deserialization error: {0}")]
    Serde(#[from] serde_yaml::Error),
}

/// Loads configuration from YAML file.
///
/// # Arguments
///
/// * `path` - Path to the YAML configuration file.
pub fn load<P: AsRef<Path>>(path: P) -> Result<RuntimeConfig, ConfigError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(ConfigError::FileNotFound(path.into()));
    }

    let content = std::fs::read_to_string(path)?;
    let config: RuntimeConfig = serde_yaml::from_str(&content)?;
    config.validate()?;
    Ok(config)
}
