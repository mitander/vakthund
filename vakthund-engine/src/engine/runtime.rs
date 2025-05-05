//! Simulation runtime core - coordinates execution of detection, prevention, and simulation components
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use opentelemetry::KeyValue;
use parking_lot::Mutex;
use tokio::task::{spawn_blocking, JoinHandle};
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, trace, warn};

use vakthund_config::{SimulatorConfig, VakthundConfig};
use vakthund_core::events::{bus::EventBus, network::NetworkEvent};
use vakthund_core::SimulationError;

use vakthund_detection::signatures::SignatureEngine;
use vakthund_prevention::firewall::Firewall;
use vakthund_protocols::{AnyParser, CoapParser, ModbusParser, MqttParser};
use vakthund_simulator::{Scenario, Simulator};
use vakthund_telemetry::{logging::EventLogger, MetricsRecorder};

use crate::engine::diagnostics::DiagnosticsCollector;
use crate::engine::event_processing::EventProcessor;
use crate::engine::runtime_trait::SimulationDriver;

/// Coordinates system operations in Vakthund, including event processing, simulation,
/// fuzz testing, and scenario-based execution.
pub struct SimulationRuntime<T: SimulationDriver + Send + Sync + 'static> {
    /// System configuration parameters
    config: Arc<VakthundConfig>,
    /// Event bus for cross-component communication (SPSC)
    pub event_bus: Arc<EventBus>,
    /// Metrics collection subsystem
    pub metrics: Arc<MetricsRecorder>,
    /// Diagnostic data collector
    diagnostics: Mutex<DiagnosticsCollector>,
    event_processor: Arc<dyn EventProcessor + Send + Sync>,
    driver: Arc<Mutex<T>>,
}

impl<T: SimulationDriver + Send + Sync + 'static> SimulationRuntime<T> {
    /// Creates a new simulation runtime with loaded configuration.
    ///
    /// # Panics
    /// If event bus creation fails due to invalid capacity
    pub fn new(config: VakthundConfig, driver: T) -> Self {
        info!("Initializing simulation runtime");
        debug!("Core config: {:?}", config.core);

        let event_bus = Arc::new(
            EventBus::with_capacity(config.core.event_bus.capacity)
                .expect("Failed to create event bus"),
        );

        // Create shared metrics
        let metrics = Arc::new(MetricsRecorder::new());

        // Construct the default event processor with shared metrics
        let default_event_processor = DefaultEventProcessor::new(metrics.clone());

        Self {
            config: Arc::new(config),
            event_bus,
            metrics,
            diagnostics: Mutex::new(DiagnosticsCollector::new()),
            event_processor: Arc::new(default_event_processor),
            driver: Arc::new(Mutex::new(driver)),
        }
    }
    /// Runs in "production mode," capturing live packets from a specified network interface.
    /// Then it sends them to the event bus and processes them in a background task.
    ///
    /// # Arguments
    /// * `interface` - Network interface name to monitor
    #[instrument(skip_all, fields(interface = %interface))]
    pub async fn run_production(self: Arc<Self>, interface: &str) -> Result<(), SimulationError> {
        info!("Starting production mode on {interface}");
        debug!("Using capture config: {:?}", self.config.capture);

        let terminate = Arc::new(AtomicBool::new(false));
        let event_bus = self.event_bus.clone();

        // Spawn event processor (drains the bus in the background)
        let processor_self = self.clone();
        let processor = tokio::spawn(async move {
            debug!("Spawning event processor thread");
            processor_self.spawn_event_processor().await
        });

        // Start capture loop on a blocking thread
        let capture_task = spawn_blocking({
            let interface = interface.to_string();
            let event_bus = event_bus.clone();
            let config = self.config.capture.clone();

            move || {
                info!("Starting packet capture on {interface}");
                vakthund_capture::capture::run_capture_loop(
                    &interface,
                    config.buffer_size,
                    config.promiscuous,
                    &terminate,
                    |packet| {
                        trace!("Captured packet: {} bytes", packet.data.len());

                        let timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("Time went backwards")
                            .as_nanos() as u64;

                        let event = NetworkEvent {
                            timestamp,
                            payload: packet.data.clone(),
                            source: None,
                            destination: None,
                        };

                        debug!("Queueing network event");
                        if let Err(e) = event_bus.send(event) {
                            warn!("Failed to queue event: {e}");
                        }
                    },
                )
            }
        });

        info!("Waiting for processor and capture tasks");
        let (processor_result, capture_result) = tokio::join!(processor, capture_task);

        // Handle processor task completion
        let _ = processor_result
            .map_err(|e| SimulationError::Processing(e.to_string()))
            .map_err(|e| {
                error!("Processor task panicked: {e}");
                SimulationError::Processing(format!("Processor panic: {e}"))
            })?;

        // Handle capture task completion
        let _ = capture_result.map_err(|e| {
            error!("Capture task failed: {e}");
            SimulationError::Processing(format!("Capture failure: {e}"))
        })?;

        info!("Production mode shutdown complete");
        Ok(())
    }

    /// Spawns a dedicated event processor task that continuously calls `recv()`
    /// on the EventBus. This can have multiple implementations based on use cases.
    ///
    /// This loop runs forever unless externally cancelled or aborted.
    #[instrument(skip(self))]
    fn spawn_event_processor(&self) -> JoinHandle<Result<(), SimulationError>> {
        let event_bus = self.event_bus.clone();
        let event_processor = self.event_processor.clone(); // Clone the trait object

        tokio::spawn(async move {
            info!("Event processor started");
            let mut processed_events = 0;

            loop {
                match event_bus.recv() {
                    Some(event) => {
                        processed_events += 1;
                        trace!("Processing event #{}", processed_events);

                        // Call Event Processor using Trait
                        event_processor.process(&event).await?;
                    }
                    None => {
                        // Queue empty, avoid busy-spin
                        sleep(Duration::from_millis(10)).await;
                    }
                }
            }
            // Not expected to return normally unless aborted
        })
    }

    /// Runs the simulation by using the concrete driver implementation
    #[instrument(skip(self))]
    pub async fn run_simulation(&self, event_count: usize) -> Result<String, SimulationError> {
        debug!("Starting simulation with {} events", event_count);

        let final_hash = String::new();
        for _i in 0..event_count {
            match self.driver.lock().next_event().await {
                Ok(Some(event)) => match self.process_event(&event).await {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                },
                Ok(None) => {
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(final_hash)
    }

    async fn process_event(&self, event: &NetworkEvent) -> Result<(), SimulationError> {
        let event_processor = self.event_processor.clone();
        event_processor.process(event).await
    }

    /// Validates scenario execution hash against the expected result in the scenario.
    #[instrument(skip(self))]
    fn validate_scenario_hash(
        &self,
        scenario: &Scenario,
        actual_hash: &str,
    ) -> Result<(), SimulationError> {
        debug!("Validating scenario hash");
        if scenario.expected_hash != actual_hash {
            error!("Hash mismatch! Expected: {}", scenario.expected_hash);

            let report = format!(
                "Scenario validation failed!\nExpected: {}\nActual: {}",
                scenario.expected_hash, actual_hash
            );

            let filename = self.diagnostics.lock().record_bug_report(&report);
            error!("Bug report saved to: {filename}");

            Err(SimulationError::Validation(report))
        } else {
            info!("Scenario validation successful");
            Ok(())
        }
    }

    /// Performs fuzz testing by repeatedly generating random (chaotic) simulator configurations.
    /// Each iteration pushes events to the bus for the background event processor to handle.
    ///
    /// # Arguments
    /// * `seed` - Base seed for RNG
    /// * `iterations` - How many fuzz cycles to run (0 means infinite)
    /// * `max_events` - Max number of events in each fuzz iteration
    #[instrument(skip(self))]
    pub async fn run_fuzz_testing(
        self: Arc<Self>,
        seed: u64,
        iterations: usize,
        max_events: usize,
    ) -> Result<(), SimulationError> {
        info!("Starting fuzz testing");

        // Log relevant config details
        let iterations_str = if iterations == 0 {
            "infinite".to_string()
        } else {
            iterations.to_string()
        };

        debug!(
            "Fuzz configuration:\n\
             - Base seed: {seed}\n\
             - Iterations: {iterations_str}\n\
             - Max events/iteration: {max_events}\n\
             - Chaos enabled: {}\n\
             - Packet rate: {}/s\n\
             - Latency: {}ms\n\
             - Jitter: {}ms",
            true, // or use self.config.* if you have a chaos-enabled setting
            self.config.monitor.thresholds.packet_rate,
            self.config.monitor.thresholds.data_volume,
            self.config.monitor.thresholds.connection_rate
        );

        if iterations == 0 {
            warn!("Infinite fuzz mode activated (Ctrl-C to exit)");
        }

        // Spawn the event processor to drain the bus in the background
        let processor_handle = tokio::spawn({
            let this_arc = self.clone();
            async move { this_arc.spawn_event_processor().await }
        });

        let mut current_iteration = 0;
        loop {
            // If a nonzero iteration count was given, break once we reach it
            if iterations > 0 && current_iteration >= iterations {
                break;
            }

            let current_seed = seed + current_iteration as u64;
            let sim_config = SimulatorConfig::generate_fuzz_config(current_seed, max_events);

            info!(
                "Starting fuzz iteration {} with seed {current_seed}",
                current_iteration + 1
            );

            // Log more details about the fuzz config
            debug!(
                "Simulator configuration:\n\
                 - Chaos probability: {:.2}%\n\
                 - Base latency: {}ms\n\
                 - Max jitter: {}ms\n\
                 - Simulated events: {}",
                sim_config.chaos.fault_probability * 100.0,
                sim_config.network.latency_ms,
                sim_config.network.jitter_ms,
                sim_config.event_count,
            );

            let mut simulator = Simulator::new(
                current_seed,
                sim_config.chaos.fault_probability > 0.0,
                sim_config.network.latency_ms,
                sim_config.network.jitter_ms,
                Some(self.event_bus.clone()),
            );

            // Generate & push events with proper validation
            let mut generated = 0usize;
            let mut failed = 0usize;
            let mut event_queue = Vec::with_capacity(sim_config.event_count);

            for event_id in 0..sim_config.event_count {
                if let Some(event) = simulator.simulate_event(event_id) {
                    generated += 1;
                    event_queue.push(event);
                }
            }

            // Batch send events and track failures
            for event in event_queue {
                match self.event_bus.send(event) {
                    Ok(_) => {}
                    Err(e) => {
                        failed += 1;
                        warn!("Failed to queue fuzzed event: {e}");
                    }
                }
            }

            // Log detailed event generation statistics
            debug!(
                "Event generation statistics:\n\
                 - Total events: {}/{}\n\
                 - Successfully queued: {}\n\
                 - Failed to queue: {}",
                generated,
                sim_config.event_count,
                generated - failed,
                failed
            );

            // Calculate appropriate sleep duration based on event count
            let sleep_duration =
                Duration::from_millis((sim_config.event_count as f64 * 10.0).min(1000.0) as u64);
            sleep(sleep_duration).await;

            info!(
                "Completed fuzz iteration {} with seed {current_seed}\n\
                 - Events: {}/{}\n\
                 - Sleep duration: {}ms",
                current_iteration + 1,
                generated - failed,
                sim_config.event_count,
                sleep_duration.as_millis()
            );

            // Optional iteration progress log
            if iterations > 0 && (current_iteration + 1) % 10 == 0 {
                info!("Progress: {}/{}", current_iteration + 1, iterations);
            }

            current_iteration += 1;
        }

        // Signal the processor to stop and wait for it to complete
        info!("Waiting for event processor to complete...");
        self.event_bus.close();

        // Wait for the processor to finish
        let _ = processor_handle.await;

        // Verify all events were processed
        if let Err(e) = self.event_bus.verify_completion() {
            warn!("Event bus verification failed: {e}");
        }

        info!(
            "Fuzz testing complete. Processed {} iterations",
            current_iteration
        );
        Ok(())
    }
}

/// Default Implementation of EventProcessor
struct DefaultEventProcessor {
    signature_engine: SignatureEngine,
    metrics: Arc<MetricsRecorder>,
}

impl DefaultEventProcessor {
    fn new(metrics: Arc<MetricsRecorder>) -> Self {
        Self {
            signature_engine: SignatureEngine::new(),
            metrics,
        }
    }
}

#[async_trait::async_trait]
impl EventProcessor for DefaultEventProcessor {
    #[instrument(skip_all, level = "debug")]
    async fn process(&self, event: &NetworkEvent) -> Result<(), SimulationError> {
        // The function name & arguments are at debug level.
        // That means “enter” logs only show if RUST_LOG=debug or lower.
        debug!("Processing network event ({} bytes)", event.payload.len());

        let parsers = [
            AnyParser::Mqtt(MqttParser::new()),
            AnyParser::Coap(CoapParser::new()),
            AnyParser::Modbus(ModbusParser::new()),
        ];

        for parser in &parsers {
            match parser {
                AnyParser::Mqtt(p) => {
                    trace!("Attempting MQTT parsing");
                    if let Ok(packet) = p.parse(&event.payload) {
                        debug!("MQTT packet parsed");
                        let start_time = SystemTime::now();
                        let matches = self.signature_engine.buffer_scan(packet.payload());

                        self.metrics
                            .detection_latency
                            .observe(start_time.elapsed().unwrap().as_nanos() as f64);
                        handle_detection_results(matches, "MQTT").await;
                        return Ok(());
                    }
                }
                AnyParser::Coap(p) => {
                    trace!("Attempting CoAP parsing");
                    if let Ok(packet) = p.parse(&event.payload) {
                        debug!("CoAP packet parsed");
                        let start_time = SystemTime::now();
                        let matches = self.signature_engine.buffer_scan(packet.payload());
                        self.metrics
                            .detection_latency
                            .observe(start_time.elapsed().unwrap().as_nanos() as f64);
                        handle_detection_results(matches, "CoAP").await;
                        return Ok(());
                    }
                }
                AnyParser::Modbus(p) => {
                    trace!("Attempting Modbus parsing");
                    if let Ok(packet) = p.parse(&event.payload) {
                        debug!("Modbus packet parsed");
                        let start_time = SystemTime::now();
                        let matches = self.signature_engine.buffer_scan(packet.payload());
                        self.metrics
                            .detection_latency
                            .observe(start_time.elapsed().unwrap().as_nanos() as f64);
                        handle_detection_results(matches, "Modbus").await;
                        return Ok(());
                    }
                }
            }
        }

        warn!("No compatible protocol parser found");
        Ok(())
    }
}

/// Handles detection results (e.g., malicious signatures) and triggers prevention actions.
async fn handle_detection_results(matches: Vec<usize>, protocol: &str) {
    if matches.is_empty() {
        return;
    }

    info!(
        "Detected {} suspicious patterns in {protocol}",
        matches.len()
    );

    let fw = match Firewall::new("eth0") {
        Ok(fw) => fw,
        Err(e) => {
            error!("Firewall initialization failed: {e}");
            return;
        }
    };

    const BLOCK_IP: std::net::Ipv4Addr = std::net::Ipv4Addr::new(127, 0, 0, 1);

    let result = block_ip_and_log(fw, BLOCK_IP).await;
    if let Err(e) = result {
        error!("Firewall block failed: {e}");
    }
}

async fn block_ip_and_log(mut firewall: Firewall, ip: std::net::Ipv4Addr) -> Result<(), String> {
    if let Err(e) = firewall.block_ip(ip) {
        let error_msg = e.to_string();
        EventLogger::log_event(
            "firewall_error",
            vec![
                KeyValue::new("error", error_msg.clone()),
                KeyValue::new("action", "block_ip"),
            ],
        )
        .await;
        return Err(error_msg);
    }

    info!("Successfully blocked IP: {ip}");
    EventLogger::log_event(
        "firewall_block",
        vec![KeyValue::new("ip_address", ip.to_string())],
    )
    .await;

    Ok(())
}
