//! Reporting subsystem
//!
//! Provides alerting and snapshot functionality.
//! Alerts are forwarded to multiple sinks, and snapshots capture system state for replay.

pub mod alerts;
pub mod snapshots;

pub use alerts::{init_alerts, send_alert, Alert, AlertLevel};
pub use snapshots::{init_snapshots, load_snapshot, save_snapshot, Snapshot};

/// Initialize reporting (alerts and snapshots) using the configuration.
pub fn init(config: &crate::config::Config) -> anyhow::Result<()> {
    init_alerts(&config.reporting.alerts)?;
    init_snapshots(&config.reporting.snapshots)?;
    Ok(())
}
