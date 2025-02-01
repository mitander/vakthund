//! # Vakthund Monitor
//!
//! Provides traffic monitoring functionality that calculates packet rate,
//! data volume, port entropy, and applies quarantine actions based on thresholds.

pub mod monitor;

pub use monitor::{Monitor, MonitorConfig};
