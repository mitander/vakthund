//! ## vakthund-telemetry::metrics
//! **Prometheus exporter with histograms**
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

use prometheus::{Counter, Histogram, HistogramOpts, Registry};

#[derive(Debug, Clone)]
pub struct MetricsRecorder {
    pub registry: prometheus::Registry,
    pub processed_events: prometheus::Counter,
    pub detection_latency: prometheus::Histogram,
}

impl Default for MetricsRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsRecorder {
    pub fn new() -> Self {
        let registry = Registry::new();
        let processed_events =
            Counter::new("vakthund_events_total", "Total processed network events").unwrap();

        let detection_latency = Histogram::with_opts(
            HistogramOpts::new(
                "vakthund_detection_latency_ns",
                "Detection engine processing time",
            )
            .buckets(vec![1_000.0, 10_000.0, 100_000.0, 1_000_000.0]),
        )
        .unwrap();

        registry
            .register(Box::new(processed_events.clone()))
            .unwrap();
        registry
            .register(Box::new(detection_latency.clone()))
            .unwrap();

        Self {
            registry,
            processed_events,
            detection_latency,
        }
    }

    pub fn gather_metrics(&self) -> Result<String, prometheus::Error> {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let mut buffer = Vec::<u8>::new();
        encoder.encode(&self.registry.gather(), &mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }

    pub fn inc_processed_events(&self) {
        self.processed_events.inc();
    }
}
