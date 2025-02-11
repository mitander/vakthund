//! # Runtime Engine
//!
//! This module provides the core runtime functionality for Vakthund
//! - production mode
//! - simulation mode
//! - replay mode with scenarios

use std::fs::File;
use std::io::Write;
use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use opentelemetry::KeyValue;
use tracing::{error, info, instrument, Instrument};

use vakthund_capture::capture;
use vakthund_config::{self, SimulatorConfig, VakthundConfig};
use vakthund_core::events::{bus::EventBus, network::NetworkEvent};
use vakthund_detection::signatures::SignatureEngine;
use vakthund_prevention::firewall::Firewall;
use vakthund_protocols::mqtt::MqttParser;
use vakthund_simulator::{
    replay::{ReplayEngine, Scenario},
    virtual_clock::VirtualClock,
    Simulator,
};
use vakthund_telemetry::{logging::EventLogger, metrics::MetricsRecorder};

/// Generates a bug report file with the given report details.
/// This function is used when a fatal error (such as a state hash mismatch)
/// is encountered.
fn generate_bug_report(report: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let filename = format!("bug_report_{}.txt", now);
    match File::create(&filename) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(report.as_bytes()) {
                eprintln!("Failed to write bug report: {:?}", e);
            } else {
                println!("Bug report written to {}", filename);
            }
        }
        Err(e) => eprintln!("Failed to create bug report file: {:?}", e),
    }
}

/// Runs production (live capture) mode.
/// Loads configuration from `"config/production.yaml"` and uses the specified
/// settings for event bus capacity, telemetry, etc.
///
/// # Arguments
/// * `interface` - The network interface to capture on.
/// * `metrics` - The metrics recorder to be used for telemetry.
#[instrument(level = "info", name = "run_production_mode", skip(metrics))]
pub async fn run_production_mode(
    interface: &str,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_path = "config/vakthund.yaml";
    let config = VakthundConfig::load_from_path(config_path)
        .unwrap_or_else(|err| panic!("Failed to load configuration from {config_path}: {err}",));
    info!("Loaded configuration: {:?}", config);

    // Use the event bus capacity from configuration.
    let event_bus_capacity = config.core.event_bus.capacity;
    let event_bus = Arc::new(
        EventBus::with_capacity(event_bus_capacity)
            .expect("Failed to create event bus with configured capacity"),
    );
    let event_bus_for_processing = event_bus.share().into();
    let metrics_processor = metrics.clone();

    // Spawn the event processing task.
    let processor_handle = tokio::spawn(
        async move { process_events_from_bus(event_bus_for_processing, metrics_processor).await }
            .instrument(tracing::info_span!("event_processor_task")),
    );

    let terminate = Arc::new(AtomicBool::new(false));
    info!("Starting live capture on interface: {}", interface);
    let event_bus_for_capture = event_bus.share().into();

    // Create an owned copy of the interface string so that it can be moved into the async block.
    let interface_owned = interface.to_owned();

    // Spawn the capture task.
    let capture_handle = tokio::spawn(
        async move {
            run_capture_loop(
                interface_owned.as_str(),
                config.capture.buffer_size,
                config.capture.promiscuous,
                &terminate,
                event_bus_for_capture,
            )
            .await
        }
        .instrument(tracing::info_span!("capture_task")),
    );

    // Wait for the processing and capture tasks to complete.
    processor_handle.await??;
    let _ = capture_handle.await?;
    Ok(())
}

/// Runs simulation mode (or replay if a scenario file is provided).
/// Loads simulation configuration from `"config.yaml"`.
///
/// # Arguments
/// * `scenario_path` - Optional path to a scenario file. If provided, replay mode is used.
/// * `num_events` - Number of events to simulate (if no scenario file is given).
/// * `seed` - Seed for the virtual clock and randomness.
/// * `validate_hash` - Optional expected state hash (for validation).
/// * `metrics` - The metrics recorder for telemetry.
#[instrument(level = "info", name = "run_simulation_mode", skip(metrics))]
pub async fn run_simulation_mode<P: AsRef<Path> + std::fmt::Debug>(
    scenario_path: Option<P>,
    num_events: usize,
    seed: u64,
    validate_hash: Option<&str>,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = VakthundConfig::load().expect("Configuration load failed");
    info!("Loaded configuration: {:?}", config);

    // Load the simulation-specific configuration.
    let path = "config/sim_config.yaml";
    let sim_config = match SimulatorConfig::load_from_path(path) {
        Ok(cfg) => cfg,
        Err(err) => {
            info!("No simulation config found: {err}");
            SimulatorConfig::default()
        }
    };

    info!("Loaded simulation configuration: {:?}", config);

    metrics.processed_events.inc();

    if let Some(path) = scenario_path {
        // Replay mode remains unchanged.
        info!("Replaying scenario from file: {:?}", path.as_ref());
        let scenario = match Scenario::load_from_path(path) {
            Ok(s) => s,
            Err(e) => {
                generate_bug_report(&format!(
                    "Failed to load scenario.\nError: {:?}\nConfiguration: {:?}",
                    e, config
                ));
                return Err(Box::new(e));
            }
        };
        let clock = VirtualClock::new(seed);
        let replay_engine = ReplayEngine::new(scenario, clock);
        while let Some(_event) = replay_engine.next_event().await {
            // Process replayed events as needed.
        }
        let final_hash = "replay_dummy_hash".to_string();
        info!("Replay complete. State hash: {}", final_hash);
        EventLogger::log_event(
            "replay_complete",
            vec![
                KeyValue::new("seed", seed.to_string()),
                KeyValue::new("final_hash", final_hash.clone()),
            ],
        )
        .await;
        Ok(())
    } else {
        let event_bus = Arc::new(
            EventBus::with_capacity(config.core.event_bus.capacity)
                .expect("Failed to create event bus with configured capacity"),
        );
        let event_bus_sim: Arc<EventBus> = event_bus.share().into();
        let event_bus_processing: Arc<EventBus> = event_bus.share().into();

        let effective_seed = if seed != 0 { seed } else { sim_config.seed };

        let mut simulator = Simulator::new(
            effective_seed,
            sim_config.chaos.fault_probability > 0.0,
            sim_config.network.latency_ms,
            sim_config.network.jitter_ms,
            Some(event_bus),
        );

        let metrics_processor = metrics.clone();
        let processor_handle = tokio::spawn(
            async move { process_events_from_bus(event_bus_processing, metrics_processor).await }
                .instrument(tracing::info_span!("event_processor_task")),
        );

        // Spawn a task to simulate events and push them into the event bus.
        let simulator_handle = tokio::spawn(async move {
            for event_id in 0..num_events {
                if let Some(event) = simulator.simulate_event(event_id) {
                    blocking_push(&event_bus_sim, event)
                }
            }
            // Return the final state hash.
            simulator.finalize_hash()
        });

        let final_hash = simulator_handle.await.unwrap();
        processor_handle.await??;
        println!("Simulation complete. State hash: {}", final_hash);
        if let Some(expected) = validate_hash {
            if final_hash != expected {
                generate_bug_report(&format!(
                    "Simulation error: state hash mismatch!\nExpected: {}\nGot: {}\nConfiguration: {:?}",
                    expected, final_hash, config
                ));
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "State hash mismatch",
                )));
            }
        }
        EventLogger::log_event(
            "simulation_complete",
            vec![
                KeyValue::new("event_count", num_events.to_string()),
                KeyValue::new("seed", effective_seed.to_string()),
                KeyValue::new("final_hash", final_hash.clone()),
            ],
        )
        .await;
        Ok(())
    }
}

/// Internal function that continuously processes events from the event bus.
///
/// # Arguments
/// * `event_bus` - Shared event bus handle.
/// * `metrics` - Metrics recorder for telemetry.
#[instrument(
    level = "debug",
    name = "process_events_from_bus",
    skip(event_bus, metrics)
)]
async fn process_events_from_bus(
    event_bus: Arc<EventBus>,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let parser = MqttParser::new();
    let signature_engine = SignatureEngine::new();

    while let Some(event) = event_bus.try_pop() {
        process_network_event(event, &parser, &signature_engine, &metrics).await;
        println!("processed event");
    }
    Ok(())
}

/// Internal function to process a single network event.
///
/// It parses the MQTT packet, measures detection latency, and if a signature match
/// is found, triggers the detection handler.
///
/// # Arguments
/// * `event` - The network event.
/// * `parser` - The MQTT parser.
/// * `signature_engine` - The signature engine for matching patterns.
/// * `metrics` - Metrics recorder for telemetry.
#[instrument(
    level = "debug",
    name = "process_network_event",
    skip(parser, signature_engine, metrics)
)]
async fn process_network_event(
    event: NetworkEvent,
    parser: &MqttParser,
    signature_engine: &SignatureEngine,
    metrics: &MetricsRecorder,
) {
    let packet_result = parser.parse(&event.payload);
    let packet = match packet_result {
        Ok(pkt) => pkt,
        Err(e) => {
            error!("MQTT Parse error: {:?}", e);
            return;
        }
    };

    let start_time = Instant::now();
    let matches = signature_engine.buffer_scan(packet.payload);
    let elapsed_time = start_time.elapsed();
    metrics
        .detection_latency
        .observe(elapsed_time.as_nanos() as f64);

    if !matches.is_empty() {
        handle_detection_match(metrics).await;
    }
}

/// Internal function to handle detection events.
/// It attempts to block an IP address (dummy implementation) and logs the event.
///
/// # Arguments
/// * `metrics` - Metrics recorder for telemetry.
#[instrument(level = "debug", name = "handle_detection_match", skip(metrics))]
async fn handle_detection_match(metrics: &MetricsRecorder) {
    let block_result = Firewall::new("dummy_interface")
        .and_then(|mut fw| fw.ip_block(Ipv4Addr::new(127, 0, 0, 1)));

    match block_result {
        Ok(_) => {
            metrics.processed_events.inc();
            EventLogger::log_event(
                "firewall_block",
                vec![KeyValue::new("ip_address", "127.0.0.1")],
            )
            .await;
        }
        Err(e) => {
            error!("Firewall block error: {:?}", e);
            EventLogger::log_event(
                "firewall_error",
                vec![
                    KeyValue::new("error", e.to_string()),
                    KeyValue::new("action", "block_ip"),
                ],
            )
            .await;
        }
    }
}

/// Internal function to run the capture loop using pcap.
///
/// # Arguments
/// * `interface` - The network interface name.
/// * `buffer_size` - Maximum capture buffer size.
/// * `promiscuous` - Whether to run in promiscuous mode.
/// * `terminate` - A flag that, when set, causes the capture loop to exit.
/// * `event_bus` - The shared event bus to which captured packets are enqueued.
#[instrument(level = "debug", name = "run_capture_loop", skip(interface, event_bus))]
async fn run_capture_loop(
    interface: &str,
    buffer_size: usize,
    promiscuous: bool,
    terminate: &Arc<AtomicBool>,
    event_bus: Arc<EventBus>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    capture::run(
        interface,
        buffer_size,
        promiscuous,
        terminate,
        &mut move |packet: &vakthund_capture::packet::Packet| {
            push_captured_packet(packet, event_bus.clone());
        },
    );
    Ok(())
}

/// Internal function to enqueue a captured packet onto the event bus.
///
/// # Arguments
/// * `packet` - The captured packet.
/// * `event_bus` - The shared event bus.
#[instrument(level = "debug", name = "enqueue_captured_packet", skip(event_bus))]
fn push_captured_packet(packet: &vakthund_capture::packet::Packet, event_bus: Arc<EventBus>) {
    let timestamp = Instant::now().elapsed().as_nanos() as u64;
    let event = NetworkEvent::new(timestamp, packet.data.clone());
    blocking_push(&event_bus, event);
    info!("Captured packet with {} bytes", packet.data.len());
}

fn blocking_push(event_bus: &Arc<EventBus>, event: NetworkEvent) {
    use vakthund_core::events::bus::EventError;
    loop {
        match event_bus.try_push(event.clone()) {
            Ok(_) => break,
            Err(EventError::QueueFull) => {
                std::thread::yield_now();
            }
            Err(e) => {
                error!("Unexpected error during blocking push: {:?}", e);
                break;
            }
        }
    }
}
