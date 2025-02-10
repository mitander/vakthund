// vakthund-config/src/capture.rs
//! Packet capture configuration for live and simulated environments.
//!
//! Defines parameters for network packet acquisition across different modes:
//! - Live capture (XDP/pcap)
//! - File‑based replay
//! - Simulated traffic generation

use serde::{Deserialize, Deserializer, Serialize};
use validator::{self, Validate};

use crate::validation;

/// Packet capture configuration.
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct CaptureConfig {
    /// Capture mode (xdp, pcap, simulated).
    #[validate(custom(function = validation::validate_mode))]
    pub mode: String,

    /// Network interface for live capture.
    #[validate(custom(function = validation::validate_interface))]
    #[serde(default = "default_interface")]
    pub interface: String,

    /// Run in promiscuous mode?
    #[serde(default = "default_promiscuous")]
    pub promiscuous: bool,

    /// Capture buffer size in bytes.
    #[validate(range(min = 4096, max = 1073741824))]
    #[serde(default = "default_buffer_size", deserialize_with = "deserialize_size")]
    pub buffer_size: usize,

    /// Maximum capture latency (milliseconds).
    #[validate(range(min = 1, max = 5000))]
    #[serde(default = "default_latency")]
    pub max_latency_ms: u32,
}

fn default_interface() -> String {
    "eth0".into()
}

fn default_promiscuous() -> bool {
    true
}

fn default_buffer_size() -> usize {
    1048576
}

fn default_latency() -> u32 {
    100
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SizeValue {
    Num(usize),
    Str(String),
}

/// Custom deserializer to allow human‑friendly sizes (e.g. "1MiB") or direct numbers.
fn deserialize_size<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let sv = SizeValue::deserialize(deserializer)?;
    match sv {
        SizeValue::Num(n) => Ok(n),
        SizeValue::Str(s) => {
            let s = s.trim();
            let mut num_part = String::new();
            let mut unit_part = String::new();
            for c in s.chars() {
                if c.is_ascii_digit() || c == '.' {
                    num_part.push(c);
                } else {
                    unit_part.push(c);
                }
            }
            let number: f64 = num_part.parse().map_err(serde::de::Error::custom)?;
            let multiplier = match unit_part.to_lowercase().as_str() {
                "kb" | "kib" => 1024.0,
                "mb" | "mib" => 1024.0 * 1024.0,
                "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
                "" => 1.0,
                _ => return Err(serde::de::Error::custom("Unknown size unit")),
            };
            Ok((number * multiplier) as usize)
        }
    }
}
impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mode: "xdp".into(),
            interface: default_interface(),
            promiscuous: default_promiscuous(),
            buffer_size: default_buffer_size(),
            max_latency_ms: default_latency(),
        }
    }
}
