//! ## vakthund-telemetry::logging
//! **JSON logger with `serde` integration**
//!
//! ### Expectations:
//! - <3% overhead at 1M events/sec
//! - Structured logging with OpenTelemetry
//! - Alert deduplication with sliding windows
//!
//! ### Components:
//! - `metrics/`: Prometheus exporter with histograms
//! - `logging/`: JSON logger with `serde` integration
//! - `alerts/`: Stateful alert correlation engine
//!
//! ### Future:
//! - eBPF-based performance monitoring
//! - Anomaly detection on telemetry data
//!
//! Structured logging with tracing and OpenTelemetry

use opentelemetry::KeyValue;
use tracing::{info_span, Instrument};
use tracing_subscriber::fmt::format::FmtSpan;

pub struct EventLogger;

impl EventLogger {
    pub fn init() {
        tracing_subscriber::fmt()
            .with_span_events(FmtSpan::ENTER)
            .with_thread_names(true)
            .init();
    }

    /// Log security event with key metadata
    #[inline] // Potential inlining for performance
    pub async fn log_event(event_type: &str, metadata: Vec<KeyValue>) {
        let span = info_span!(
            "security_event",
            event_type = event_type,
            otel.kind = "INTERNAL"
        );

        async {
            tracing::info!(
                metadata = ?metadata,
                "Security event occurred"
            );
        }
        .instrument(span)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_logging() {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(EventLogger::log_event(
                "test",
                vec![KeyValue::new("key", "value")],
            )); // Block on future
        assert!(logs_contain("Security event occurred"));
    }
}
