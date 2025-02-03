//! Alert reporting module.
use crate::config::AlertConfig;
use bytes::Bytes;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub enum AlertLevel {
    Info,
    Warn,
    Critical,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub message: String,
    pub level: AlertLevel,
    pub packet: Bytes,
}

pub fn init_alerts(config: &AlertConfig) -> anyhow::Result<()> {
    if config.console {
        info!("Console alerting enabled");
    }
    if let Some(syslog) = &config.syslog {
        info!("Syslog alerting enabled: {}:{}", syslog.server, syslog.port);
    }
    Ok(())
}

pub fn send_alert(alert: Alert) {
    match alert.level {
        AlertLevel::Info => info!("ALERT (INFO): {}", alert.message),
        AlertLevel::Warn => warn!("ALERT (WARN): {}", alert.message),
        AlertLevel::Critical => error!("ALERT (CRITICAL): {}", alert.message),
    }
}
