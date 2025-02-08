pub mod logging;
pub mod metrics;

pub use logging::EventLogger;
pub use metrics::MetricsRecorder;

#[cfg(feature = "engine")]
impl vakthund_core::engine::MetricsRecorderTrait for MetricsRecorder {
    fn detection_latency_observe(&self, latency: f64) {
        MetricsRecorder::detection_latency_observe(self, latency)
    }
    fn processed_events_inc(&self) {
        MetricsRecorder::processed_events_inc(self)
    }
}

#[cfg(feature = "engine")]
impl vakthund_core::engine::EventLoggerTrait for EventLogger {
    async fn log_event(&self, event_type: &str, metadata: Vec<KeyValue>) {
        EventLogger::log_event(self, event_type, metadata).await
    }
}
