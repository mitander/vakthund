// vakthund-core/src/engine/runtime.rs

/*!
# Runtime Engine

This module provides the core runtime functionality for Vakthund,
including the production mode (live capture) and the simulation mode.
This abstraction lets different frontends (CLI, GUI, web) share the same
implementation of event processing, simulation, and error reporting.
*/

use std::fs::File;
use std::io::Write;
use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::Instrument;

use bytes::Bytes;
use opentelemetry::KeyValue;
use tracing::{error, info, instrument};

use vakthund_core::events::{EventBus, NetworkEvent};
use vakthund_core::time::VirtualClock;

use vakthund_capture::capture;
use vakthund_detection::signatures::SignatureEngine;
use vakthund_prevention::firewall::Firewall;
use vakthund_protocols::mqtt::MqttParser;
use vakthund_telemetry::{logging::EventLogger, metrics::MetricsRecorder};

/// Runs the production mode using live capture (pcap).
#[instrument(level = "info", name = "run_production_mode", skip(metrics))]
pub async fn run_production_mode(
    interface: &str,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event_bus = Arc::new(EventBus::with_capacity(10_000).expect("Failed to create event bus"));
    let event_bus_for_processing = event_bus.share().into();
    let metrics_processor = metrics.clone();

    let processor_handle = tokio::spawn(
        async move { process_events_from_bus(event_bus_for_processing, metrics_processor).await }
            .instrument(tracing::info_span!("event_processor_task")),
    );

    let buffer_size = 1_048_576;
    let promiscuous = true;
    let terminate = Arc::new(AtomicBool::new(false));
    info!("Starting live capture on interface: {}", interface);
    let event_bus_for_capture = event_bus.share().into();

    let interface_clone = interface.to_owned();
    let capture_handle = tokio::spawn(
        async move {
            run_capture_loop(
                interface_clone.as_str(),
                buffer_size,
                promiscuous,
                &terminate,
                event_bus_for_capture,
            )
            .await
        }
        .instrument(tracing::info_span!("capture_task")),
    );

    processor_handle.await??;
    let _ = capture_handle.await?;
    Ok(())
}

/// Runs the simulation mode (or replay if a scenario file is provided).
///
/// * `scenario_path` is an optional path to a scenario file.
/// * If no scenario file is provided, a simulation will be run.
#[instrument(level = "info", name = "run_simulation_mode", skip(metrics))]
pub async fn run_simulation_mode<P: AsRef<Path> + std::fmt::Debug>(
    scenario_path: Option<P>,
    num_events: usize,
    seed: u64,
    validate_hash: Option<&str>,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    metrics.processed_events.inc();
    if let Some(path) = scenario_path {
        info!("Replaying scenario from file: {:?}", path.as_ref());
        let scenario = match vakthund_simulator::replay::Scenario::load_from_file(path) {
            Ok(s) => s,
            Err(e) => {
                generate_bug_report(&format!("Failed to load scenario.\nError: {:?}", e));
                return Err(Box::new(e));
            }
        };
        let clock = VirtualClock::new(seed);
        let replay_engine = vakthund_simulator::replay::ReplayEngine::new(scenario, clock);
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
    } else {
        let mut simulator = vakthund_simulator::Simulator::new(seed, false);
        let final_hash = simulator.run(num_events);
        info!("Simulation complete. State hash: {}", final_hash);
        if let Some(expected) = validate_hash {
            if final_hash != expected {
                generate_bug_report(&format!(
                    "Simulation error: state hash mismatch!\nExpected: {}\nGot: {}",
                    expected, final_hash
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
                KeyValue::new("seed", seed.to_string()),
                KeyValue::new("final_hash", final_hash.clone()),
            ],
        )
        .await;
    }
    Ok(())
}

/// Internal function to process events from the event bus.
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
    }
    Ok(())
}

/// Internal function to process a single network event.
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

/// Internal function to run the capture loop.
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
            enqueue_captured_packet(packet, event_bus.clone());
        },
    );
    Ok(())
}

/// Internal function to enqueue a captured packet.
#[instrument(level = "debug", name = "enqueue_captured_packet", skip(event_bus))]
fn enqueue_captured_packet(packet: &vakthund_capture::packet::Packet, event_bus: Arc<EventBus>) {
    let timestamp = Instant::now().elapsed().as_nanos() as u64;
    let event = NetworkEvent::new(timestamp, Bytes::from(packet.data.clone()));
    if let Err(e) = event_bus.try_push(event) {
        error!("Failed to push packet: {:?}", e);
    }
    info!("Captured packet with {} bytes", packet.data.len());
}

/// Generates a bug report file with the given report details.
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
