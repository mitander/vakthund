//! # Application Configuration
//!
//! This module provides a unified configuration loading mechanism.
//! Depending on the environment (e.g. production or development), it loads the appropriate YAML file.
//! The configuration is deserialized into strongly typed structs using `serde`.
//!
//! **Files:**
//! - Default configuration: `config.yaml` at the repository root
//! - Production configuration: `config/production.yaml`
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::Path;

/// The main application configuration.
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// Configuration for the event bus.
    pub event_bus: EventBusConfig,
    /// Telemetry and logging configuration.
    pub telemetry: TelemetryConfig,
}

/// Configuration for the event bus.
#[derive(Debug, Deserialize)]
pub struct EventBusConfig {
    /// The capacity of the event bus (must be a power of two).
    pub capacity: usize,
    /// Whether to enforce that capacity is a power of two.
    pub require_power_of_two: bool,
}

/// Telemetry configuration.
#[derive(Debug, Deserialize)]
pub struct TelemetryConfig {
    /// The bind address for exposing Prometheus metrics.
    #[serde(default = "default_metrics_addr")]
    pub metrics_addr: String,
    /// The default logging level.
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Whether OpenTelemetry integration is enabled.
    #[serde(default)]
    pub enable_otel: bool,
}

fn default_metrics_addr() -> String {
    "127.0.0.1:9090".into()
}

fn default_log_level() -> String {
    "info".into()
}

/// Loads the application configuration.
///
/// The file path is selected based on the `RUN_MODE` environment variable:
/// - If `RUN_MODE=production`, then `config/production.yaml` is loaded.
/// - Otherwise, `config.yaml` at the repository root is used.
///
/// # Errors
///
/// Returns an error if the configuration file cannot be read or deserialized.
pub fn load_config() -> Result<AppConfig, Box<dyn Error>> {
    // Choose the configuration file based on RUN_MODE.
    let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
    let config_path = if run_mode == "production" {
        "config/production.yaml"
    } else {
        "config.yaml"
    };

    let config_content = fs::read_to_string(Path::new(config_path))?;
    let config: AppConfig = serde_yaml::from_str(&config_content)?;
    Ok(config)
}
