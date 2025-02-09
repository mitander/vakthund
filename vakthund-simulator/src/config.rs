use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct SimConfig {
    pub capture: CaptureConfig,
    pub monitor: MonitorConfig,
    pub alert_methods: Vec<String>,
    /// Optional event bus capacity for simulation (default is 4096 if not provided).
    pub event_bus_capacity: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CaptureConfig {
    pub mode: String,
    pub interface: String,
    pub buffer_size: usize,
    pub promiscuous: bool,
    pub seed: u64,
    /// Optional fixed latency in milliseconds (default 100).
    pub latency_ms: Option<u64>,
    /// Optional maximum jitter in milliseconds (default 10).
    pub jitter_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct MonitorConfig {
    pub quarantine_timeout: u64,
    pub thresholds: Thresholds,
    pub whitelist: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Thresholds {
    pub packet_rate: f64,
    pub data_volume: f64,
    pub port_entropy: f64,
}

#[derive(Error, Debug)]
pub enum SimConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Deserialization error: {0}")]
    Serde(#[from] serde_yaml::Error),
}

/// Loads the simulation configuration from a YAML file.
pub fn load_sim_config<P: AsRef<Path>>(path: P) -> Result<SimConfig, SimConfigError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(SimConfigError::FileNotFound(format!(
            "{} does not exist",
            path.display()
        )));
    }
    let content = std::fs::read_to_string(path)?;
    let config: SimConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}
