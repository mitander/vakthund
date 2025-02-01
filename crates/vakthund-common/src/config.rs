//! # Configuration Module
//!
//! This module loads configuration from a YAML file using Serde and serde_yaml.
//! The configuration is strongly typed into Rust structs and enums, and is loaded
//! via the `Config::load` method.
//!
//! Example configuration file (config.yaml):
//!
//! ```yaml
//! capture:
//!   mode: "simulation"         # Allowed values: "live" or "simulation" (case insensitive)
//!   interface: "en0"
//!   buffer_size: 1048576
//!   promiscuous: true
//!   seed: 42
//!
//! monitor:
//!   quarantine_timeout: 600
//!   thresholds:
//!     packet_rate: 1000.0
//!     data_volume: 10485760.0
//!     port_entropy: 2.5
//!   whitelist:
//!     - "192.168.1.1"
//! ```

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;

pub const CONFIG_FILE: &str = "config.yaml";

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub capture: CaptureConfig,
    pub monitor: MonitorConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CaptureConfig {
    #[serde(default = "default_mode")]
    pub mode: CaptureMode,
    pub interface: String,
    pub buffer_size: usize,
    pub promiscuous: bool,
    #[serde(default)]
    pub seed: Option<u64>,
}

/// Returns the default capture mode: Live.
fn default_mode() -> CaptureMode {
    CaptureMode::Live
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    Live,
    Simulation,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MonitorConfig {
    pub quarantine_timeout: u64,
    pub thresholds: Thresholds,
    pub whitelist: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Thresholds {
    pub packet_rate: f64,
    pub data_volume: f64,
    pub port_entropy: f64,
}

impl Config {
    /// Loads the configuration from the specified file path.
    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_config_parsing() {
        let yaml = r#"
capture:
  mode: "Simulation"
  interface: "en0"
  buffer_size: 1048576
  promiscuous: true
  seed: 42

monitor:
  quarantine_timeout: 600
  thresholds:
    packet_rate: 1000.0
    data_volume: 10485760.0
    port_entropy: 2.5
  whitelist:
    - "192.168.1.1"
"#;
        let path = "test_config.yaml";
        {
            let mut file = File::create(path).expect("Failed to create test config file");
            file.write_all(yaml.as_bytes())
                .expect("Failed to write test config file");
        }
        let config = Config::load(path).expect("Failed to load config");
        assert_eq!(config.capture.mode, CaptureMode::Simulation);
        assert_eq!(config.capture.interface, "en0");
        assert_eq!(config.capture.buffer_size, 1048576);
        assert!(config.capture.promiscuous);
        assert_eq!(config.capture.seed, Some(42));
        assert_eq!(config.monitor.quarantine_timeout, 600);
        assert_eq!(config.monitor.thresholds.packet_rate, 1000.0);
        assert_eq!(config.monitor.thresholds.data_volume, 10485760.0);
        assert_eq!(config.monitor.thresholds.port_entropy, 2.5);
        assert_eq!(config.monitor.whitelist.len(), 1);
        assert_eq!(config.monitor.whitelist[0], "192.168.1.1");
        std::fs::remove_file(path).unwrap();
    }
}
