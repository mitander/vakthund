//! Configuration Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Loads and represents configuration for Vakthund from a YAML file.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;

pub const CONFIG_FILE: &str = "config.yaml";

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AlertMethod {
    // TODO: add additional alert channels such as Dashboard, SMS, etc.
    Syslog,
    Email,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub capture: CaptureConfig,
    pub monitor: MonitorConfig,
    pub alert_methods: Vec<AlertMethod>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            capture: CaptureConfig::default(),
            monitor: MonitorConfig::default(),
            alert_methods: vec![AlertMethod::Syslog],
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CaptureConfig {
    #[serde(default = "default_mode")]
    pub mode: CaptureMode,
    pub interface: String,
    pub buffer_size: usize,
    pub promiscuous: bool,
    #[serde(default)]
    pub seed: Option<u64>,
}

fn default_mode() -> CaptureMode {
    CaptureMode::Live
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    #[default]
    Live,
    Simulation,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MonitorConfig {
    pub quarantine_timeout: u64,
    pub whitelist: Vec<String>,
    pub thresholds: Thresholds,
}

impl MonitorConfig {
    pub fn new(
        quarantine_timeout: u64,
        packet_rate: f64,
        data_volume: f64,
        port_entropy: f64,
        whitelist: Vec<String>,
    ) -> Self {
        Self {
            quarantine_timeout,
            whitelist,
            thresholds: Thresholds {
                packet_rate,
                data_volume,
                port_entropy,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Thresholds {
    pub packet_rate: f64,
    pub data_volume: f64,
    pub port_entropy: f64,
}

impl Config {
    /// Loads configuration from the specified file path.
    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
