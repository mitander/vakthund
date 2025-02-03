//! Configuration system
//!
//! Loads YAML-based configuration with defaults and validations.
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};

fn default_flush_interval() -> Duration {
    Duration::from_secs(1)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub capture: CaptureConfig,
    #[serde(default)]
    pub detection: DetectionConfig,
    #[serde(default)]
    pub simulation: SimulationConfig,
    #[serde(default)]
    pub reporting: ReportingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaptureConfig {
    pub buffer_size: usize,
    pub promiscuous: bool,
    #[serde(default = "default_flush_interval", with = "humantime_serde")]
    pub flush_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetectionConfig {
    pub rules: Vec<String>,
    pub thresholds: Thresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Thresholds {
    pub max_alerts_per_second: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimulationConfig {
    pub segment_size: usize,
    #[serde(with = "humantime_serde")]
    pub time_scale: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportingConfig {
    pub snapshots: SnapshotConfig,
    pub alerts: AlertConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnapshotConfig {
    pub directory: String,
    #[serde(with = "humantime_serde")]
    pub retention: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertConfig {
    pub console: bool,
    pub syslog: Option<SyslogConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyslogConfig {
    pub server: String,
    pub port: u16,
}

impl Config {
    /// Load configuration from the specified file path.
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&data)?;
        assert!(config.capture.buffer_size >= 1024, "Buffer size too small");
        assert!(
            !config.detection.rules.is_empty(),
            "No detection rules specified"
        );
        Ok(config)
    }
}
