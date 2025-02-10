//! Monitoring and alerting configuration.
//!
//! Defines thresholds and parameters for real‑time system monitoring
//! and security alert generation.

use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use validator::{self, Validate};

use crate::validation;

/// Monitoring configuration parameters.
#[derive(Default, Debug, Serialize, Deserialize, Validate, Clone)]
pub struct MonitorConfig {
    /// Anomaly detection thresholds.
    #[validate(nested)]
    pub thresholds: Thresholds,

    /// Network quarantine configuration.
    #[validate(nested)]
    pub quarantine: QuarantineConfig,

    /// Alert destination configuration.
    #[validate(nested)]
    pub alerts: AlertConfig,
}

/// Anomaly detection thresholds.
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct Thresholds {
    /// Maximum packets per second (per interface).
    #[validate(range(min = 100, max = 1_000_000))]
    #[serde(default = "default_packet_rate")]
    pub packet_rate: u32,

    /// Maximum data volume per minute (MB).
    #[validate(range(min = 10, max = 10_000))]
    #[serde(default = "default_data_volume")]
    pub data_volume: u32,

    /// Maximum port entropy score (Shannon entropy).
    #[validate(range(min = 0.0, max = 5.0))]
    #[serde(default = "default_port_entropy")]
    pub port_entropy: f32,

    /// Maximum new connections per second.
    #[validate(range(min = 10, max = 100_000))]
    #[serde(default = "default_connection_rate")]
    pub connection_rate: u32,
}

fn default_packet_rate() -> u32 {
    1000
}
fn default_data_volume() -> u32 {
    100
}
fn default_port_entropy() -> f32 {
    2.5
}
fn default_connection_rate() -> u32 {
    500
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            packet_rate: default_packet_rate(),
            data_volume: default_data_volume(),
            port_entropy: default_port_entropy(),
            connection_rate: default_connection_rate(),
        }
    }
}

/// Network quarantine parameters.
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct QuarantineConfig {
    /// Timeout for quarantined devices (seconds).
    #[validate(range(min = 60, max = 86400))]
    #[serde(default = "default_quarantine_timeout")]
    pub timeout: u32,

    /// Whitelisted IP addresses/CIDR ranges.
    #[validate(custom(function = validation::validate_cidr_list))]
    #[serde(default)]
    pub whitelist: Vec<IpNetwork>,
}

fn default_quarantine_timeout() -> u32 {
    600
}

impl Default for QuarantineConfig {
    fn default() -> Self {
        Self {
            timeout: default_quarantine_timeout(),
            whitelist: Vec::new(),
        }
    }
}

/// Alert destination configuration.
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct AlertConfig {
    /// Enable syslog alerts.
    #[serde(default = "default_true")]
    pub syslog: bool,

    /// Enable Prometheus alert metrics.
    #[serde(default = "default_true")]
    pub prometheus: bool,

    /// Webhook URL for critical alerts.
    #[validate(url)]
    #[serde(default)]
    pub webhook: Option<String>,

    /// Minimum alert severity level.
    #[validate(custom(function = validation::validate_severity))]
    #[serde(default = "default_severity")]
    pub min_severity: String,
}

fn default_true() -> bool {
    true
}
fn default_severity() -> String {
    "medium".into()
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            syslog: true,
            prometheus: true,
            webhook: None,
            min_severity: default_severity(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn valid_default_monitor_config() {
        let config = MonitorConfig::default();
        config.validate().expect("Default config should be valid");
    }

    #[test]
    fn invalid_thresholds() {
        let mut config = MonitorConfig::default();
        config.thresholds.packet_rate = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn valid_whitelist() {
        let mut config = MonitorConfig::default();
        config
            .quarantine
            .whitelist
            .push("192.168.1.0/24".parse().unwrap());
        config.validate().expect("Valid whitelist should pass");
    }
}
