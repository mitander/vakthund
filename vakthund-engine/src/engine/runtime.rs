//! Simulation runtime core - coordinates execution of detection, prevention, and simulation components

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use opentelemetry::KeyValue;
use parking_lot::Mutex;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tracing::{debug, error, info, instrument, trace, warn};

use vakthund_config::{SimulatorConfig, VakthundConfig};
use vakthund_core::events::{bus::EventBus, network::NetworkEvent};
use vakthund_detection::signatures::SignatureEngine;
use vakthund_prevention::firewall::Firewall;
use vakthund_protocols::{AnyParser, CoapParser, ModbusParser, MqttParser};
use vakthund_simulator::{Scenario, Simulator};
use vakthund_telemetry::logging::EventLogger;
use vakthund_telemetry::MetricsRecorder;

use crate::engine::{diagnostics::DiagnosticsCollector, error::SimulationError};

/// Central coordination point for system operations. Manages:
/// - Event bus communication
/// - Simulation state
/// - Telemetry collection
/// - Diagnostics reporting
pub struct SimulationRuntime {
    /// System configuration parameters
    config: Arc<VakthundConfig>,
    /// Event bus for cross-component communication
    pub event_bus: Arc<EventBus>,
    /// Current simulation state (if running)
    simulator: Mutex<Option<Simulator>>,
    /// Metrics collection subsystem
    metrics: MetricsRecorder,
    /// Diagnostic data collector
    diagnostics: Mutex<DiagnosticsCollector>,
}

impl SimulationRuntime {
    /// Creates a new simulation runtime with loaded configuration
    ///
    /// # Arguments
    /// * `config` - Fully validated system configuration
    ///
    /// # Panics
    /// If event bus creation fails due to invalid capacity
    pub fn new(config: VakthundConfig) -> Self {
        info!("Initializing simulation runtime");
        debug!("Core config: {:?}", config.core);

        let event_bus = Arc::new(
            EventBus::with_capacity(config.core.event_bus.capacity)
                .expect("Failed to create event bus"),
        );

        Self {
            config: Arc::new(config),
            event_bus,
            simulator: Mutex::new(None),
            metrics: MetricsRecorder::new(),
            diagnostics: Mutex::new(DiagnosticsCollector::new()),
        }
    }

    /// Starts production mode operation on specified network interface
    ///
    /// # Arguments
    /// * `interface` - Network interface name to monitor
    #[instrument(skip_all, fields(interface = %interface))]
    pub async fn run_production(self: Arc<Self>, interface: &str) -> Result<(), SimulationError> {
        info!("Starting production mode on {interface}");
        debug!("Using capture config: {:?}", self.config.capture);

        let terminate = Arc::new(AtomicBool::new(false));
        let event_bus = self.event_bus.clone();

        // Spawn event processor
        let processor_self = self.clone();
        let processor = tokio::spawn(async move {
            debug!("Spawning event processor thread");
            processor_self.spawn_event_processor().await
        });

        // Start capture loop
        let capture_task = tokio::task::spawn_blocking({
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
            .map_err(|e| {
                error!("Processor task panicked: {e}");
                SimulationError::Processing(format!("Processor panic: {e}"))
            })?
            .map_err(|e| {
                error!("Event processing failed: {e}");
                e
            })?;

        // Handle capture task completion
        capture_result.map_err(|e| {
            error!("Capture task failed: {e}");
            SimulationError::Processing(format!("Capture failure: {e}"))
        })?;

        info!("Production mode shutdown complete");
        Ok(())
    }

    /// Creates and spawns the event processing pipeline
    #[instrument(skip(self))]
    fn spawn_event_processor(&self) -> JoinHandle<Result<(), SimulationError>> {
        debug!("Initializing event processor");
        let event_bus = self.event_bus.clone();
        let metrics = self.metrics.clone();
        let signature_engine = SignatureEngine::new();

        tokio::spawn(async move {
            info!("Event processor started");
            let mut processed_events = 0;

            while let Some(event) = event_bus.recv() {
                processed_events += 1;
                trace!("Processing event #{}", processed_events);
                process_network_event(&event, &signature_engine, &metrics).await;
            }

            info!("Event processor shutdown");
            Ok(())
        })
    }

    /// Executes a predefined scenario
    ///
    /// # Arguments
    /// * `scenario` - Preloaded scenario definition
    #[instrument(skip(self, scenario))]
    pub async fn run_scenario(self: Arc<Self>, scenario: Scenario) -> Result<(), SimulationError> {
        info!("Starting scenario execution");
        debug!("Scenario details: {} events", scenario.events.len());

        let processor_self = self.clone();
        let processor = tokio::spawn(async move { processor_self.spawn_event_processor().await });

        let self_clone = self.clone();
        let simulator_task = tokio::spawn({
            let scenario = scenario.clone();
            async move {
                self_clone
                    .run_simulator(Simulator::from_scenario(&scenario), scenario.events.len())
                    .await
            }
        });

        let (processor_result, simulator_result) = tokio::join!(processor, simulator_task);

        // Convert JoinError to SimulationError
        let _processor_result =
            processor_result.map_err(|e| SimulationError::Processing(e.to_string()))??;

        let actual_hash =
            simulator_result.map_err(|e| SimulationError::Processing(e.to_string()))??;

        self.validate_scenario_hash(&scenario, &actual_hash)?;
        info!("Scenario execution completed");
        Ok(())
    }

    /// Manages simulator execution
    ///
    /// # Arguments
    /// * `simulator` - Initialized simulator instance
    /// * `event_count` - Number of events to generate
    #[instrument(skip(self, simulator))]
    pub async fn run_simulator(
        &self,
        simulator: Simulator,
        event_count: usize,
    ) -> Result<String, SimulationError> {
        debug!("Initializing simulator with {event_count} events");
        *self.simulator.lock() = Some(simulator);

        let event_bus = self.event_bus.clone();
        let mut simulator_guard = self.simulator.lock();
        let simulator = simulator_guard.as_mut().expect("Simulator was just set");

        for event_id in 0..event_count {
            trace!("Generating event {event_id}");
            if let Some(event) = simulator.simulate_event(event_id) {
                debug!("Dispatching simulated event");
                if let Err(e) = event_bus.send(event) {
                    warn!("Failed to send simulated event: {e}");
                }
            }
        }

        Ok(simulator.finalize_hash())
    }

    /// Validates scenario execution against expected hash
    ///
    /// # Arguments
    /// * `scenario` - Original scenario definition
    /// * `actual_hash` - Hash generated during execution
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

    /// Executes fuzz testing with generated scenarios
    ///
    /// # Arguments
    /// * `seed` - Base seed for random number generation
    /// * `iterations` - Number of fuzzing iterations
    /// * `max_events` - Maximum events per iteration
    #[instrument(skip(self))]
    pub async fn run_fuzz_testing(
        &self,
        seed: u64,
        iterations: usize,
        max_events: usize,
    ) -> Result<(), SimulationError> {
        info!("Starting fuzz testing");

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
            - Chaos enabled: true\n\
            - Packet rate: {}/s\n\
            - Latency: {}ms\n\
            - Jitter: {}ms",
            self.config.monitor.thresholds.packet_rate,
            self.config.monitor.thresholds.data_volume,
            self.config.monitor.thresholds.connection_rate
        );

        // Initial warning for infinite mode
        if iterations == 0 {
            warn!("Infinite fuzz mode activated (Ctrl-C to exit)");
        }

        let signature_engine = SignatureEngine::new();
        let metrics = self.metrics.clone();

        let mut current_iteration = 0;
        loop {
            if iterations > 0 && current_iteration >= iterations {
                break;
            }

            let current_seed = seed + current_iteration as u64;
            let sim_config = SimulatorConfig::generate_fuzz_config(current_seed, max_events);

            debug!(
                "Starting fuzz iteration {} with seed {current_seed}",
                current_iteration + 1
            );

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

            // Generate and collect events
            let events = (0..sim_config.event_count)
                .filter_map(|event_id| simulator.simulate_event(event_id))
                .collect::<Vec<_>>();

            debug!("Generated {} valid fuzzed events", events.len());
            process_events(events, &signature_engine, &metrics).await;

            info!(
                "Completed fuzz iteration {} with seed {current_seed}",
                current_iteration + 1
            );

            if iterations > 0 && (current_iteration + 1) % 10 == 0 {
                info!(
                    "Progress: {}/{iterations} iterations",
                    current_iteration + 1
                );
            }

            current_iteration += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }
}

/// Processes network events through protocol parsers and detection engine
///
/// # Arguments
/// * `event` - Network event to process
/// * `signature_engine` - Detection rules engine
/// * `metrics` - Metrics collection system
#[instrument(skip_all, fields(event_id = %event.timestamp))]
async fn process_network_event(
    event: &NetworkEvent,
    signature_engine: &SignatureEngine,
    metrics: &MetricsRecorder,
) {
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
                    let start_time = std::time::Instant::now();
                    let matches = signature_engine.buffer_scan(packet.payload());

                    metrics
                        .detection_latency
                        .observe(start_time.elapsed().as_nanos() as f64);
                    handle_detection_results(matches, "MQTT").await;
                    return;
                }
            }
            AnyParser::Coap(p) => {
                trace!("Attempting CoAP parsing");
                if let Ok(packet) = p.parse(&event.payload) {
                    debug!("CoAP packet parsed");
                    let start_time = std::time::Instant::now();
                    let matches = signature_engine.buffer_scan(packet.payload());
                    metrics
                        .detection_latency
                        .observe(start_time.elapsed().as_nanos() as f64);
                    handle_detection_results(matches, "CoAP").await;
                    return;
                }
            }
            AnyParser::Modbus(p) => {
                trace!("Attempting Modbus parsing");
                if let Ok(packet) = p.parse(&event.payload) {
                    debug!("Modbus packet parsed");
                    let start_time = std::time::Instant::now();
                    let matches = signature_engine.buffer_scan(packet.payload());
                    metrics
                        .detection_latency
                        .observe(start_time.elapsed().as_nanos() as f64);
                    handle_detection_results(matches, "Modbus").await;
                    return;
                }
            }
        }
    }

    warn!("No compatible protocol parser found");
}

/// Handles detection results and triggers prevention mechanisms
///
/// # Arguments
/// * `matches` - Detected pattern matches
/// * `protocol` - Protocol type where matches were found
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

#[instrument(skip_all, fields(total_events = events.len()))]
async fn process_events(
    events: Vec<NetworkEvent>,
    signature_engine: &SignatureEngine,
    metrics: &MetricsRecorder,
) {
    debug!("Processing {} fuzzed events", events.len());

    for (idx, event) in events.into_iter().enumerate() {
        trace!("Processing fuzzed event {}: {:?}", idx + 1, event);

        // 1. Validate basic event structure
        debug_assert!(!event.payload.is_empty(), "Empty payload in fuzzed event");

        // 2. Measure processing latency
        let start_time = std::time::Instant::now();

        // 3. Run through full detection pipeline
        process_network_event(&event, signature_engine, metrics).await;

        // 4. Record performance metrics
        let processing_time = start_time.elapsed();
        metrics
            .detection_latency
            .observe(processing_time.as_nanos() as f64);

        debug!("Processed fuzzed event {} in {processing_time:?}", idx + 1);

        // 5. Simulate real-time processing delay
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
