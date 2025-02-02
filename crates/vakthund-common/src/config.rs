//! Configuration Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Loads and represents configuration for Vakthund from a YAML file.

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
    /// Loads configuration from the specified file path.
    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
