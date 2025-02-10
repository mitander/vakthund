//! Observability and monitoring configuration.
//!
//! Parameters for system instrumentation:
//! - Metrics collection
//! - Distributed tracing
//! - Alerting thresholds

use crate::monitor::AlertConfig;
use serde::{Deserialize, Serialize};
use validator::{self, Validate};

#[derive(Default, Debug, Serialize, Deserialize, Validate, Clone)]
pub struct MetricsConfig {}

#[derive(Default, Debug, Serialize, Deserialize, Validate, Clone)]
pub struct TracingConfig {}

/// Telemetry configuration.
#[derive(Default, Debug, Serialize, Deserialize, Validate, Clone)]
pub struct TelemetryConfig {
    /// Metrics collection parameters.
    #[validate(nested)]
    pub metrics: MetricsConfig,

    /// Distributed tracing parameters.
    #[validate(nested)]
    pub tracing: TracingConfig,

    /// Alerting parameters.
    #[validate(nested)]
    pub alerts: AlertConfig,
}
