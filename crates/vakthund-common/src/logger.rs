//! Logger Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! This module provides a simple logging API built on the `tracing` crate. It delegates
//! logging calls to `tracing::info!`, `tracing::warn!`, and `tracing::error!` for structured,
//! low‑overhead logging. Use `init_logger()` early in your application to set up the global
//! logging subscriber.

use tracing::{error, info, warn};
use tracing_subscriber::FmtSubscriber;

/// Initializes the global logger using `tracing_subscriber`.
///
/// Call this function once at startup before any logging occurs.
/// In production, the overhead is minimized when compiled in release mode.
pub fn init_logger() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");
}

/// Logs an informational message.
pub fn log_info(message: &str) {
    info!("{}", message);
}

/// Logs a warning message.
pub fn log_warn(message: &str) {
    warn!("{}", message);
}

/// Logs an error message.
pub fn log_error(message: &str) {
    error!("{}", message);
}
