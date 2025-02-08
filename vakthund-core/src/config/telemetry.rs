//! Telemetry configuration.

use serde::Deserialize;

/// Telemetry and monitoring settings
#[derive(Debug, Deserialize)]
pub struct TelemetryConfig {
    /// Metrics server bind address
    #[serde(default = "default_metrics_addr")]
    pub metrics_addr: String,

    /// Logging verbosity level
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Enable OpenTelemetry integration
    #[serde(default)]
    pub enable_otel: bool,
}

fn default_metrics_addr() -> String {
    "127.0.0.1:9090".into()
}

fn default_log_level() -> String {
    "info".into()
}
