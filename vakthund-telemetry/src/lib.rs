//! # Vakthund Telemetry and Monitoring
//!
//! Crate for logging, metrics, and alerting functionalities.

// TODO: implement
// pub mod alerts;
pub mod logging;
pub mod metrics;

pub use logging::EventLogger;
pub use metrics::MetricsRecorder;
