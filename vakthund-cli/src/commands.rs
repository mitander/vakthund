//! ## vakthund-cli::commands
//! **Command parsing and execution logic**
//!
//! This module handles command-line argument parsing using `clap` and
//! orchestrates the execution of different Vakthund modes (`run`, `simulate`).

use bytes::Bytes;
use clap::{Args, Parser, Subcommand};
use opentelemetry::KeyValue;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Instant;
use tracing::info_span;
use tracing::{error, info, instrument, Instrument};

use vakthund_capture::capture;
use vakthund_core::event_bus::{EventBus, NetworkEvent};
use vakthund_detection::signatures::SignatureEngine;
use vakthund_prevention::firewall::Firewall;
use vakthund_protocols::mqtt::MqttParser;
use vakthund_simulator::Simulator;
use vakthund_telemetry::logging::EventLogger;
use vakthund_telemetry::metrics::MetricsRecorder;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run in production mode (live capture using pcap)
    Run(RunArgs),
    /// Run deterministic simulation (using a scenario file)
    Simulate(SimulateArgs),
}

#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    #[arg(short, long)]
    pub interface: String,
}

#[derive(Args, Debug, Clone)]
pub struct SimulateArgs {
    #[arg(short, long)]
    pub scenario: PathBuf,
    /// Number of events to simulate
    #[arg(long, default_value_t = 10)]
    pub events: usize,
    #[arg(long, default_value_t = 0)]
    pub seed: u64,
    #[arg(long)]
    pub validate_hash: Option<String>,
}

/// Production mode that uses live capture via pcap.
/// It sets up a termination flag and calls run().
/// Each captured packet is converted into a NetworkEvent and enqueued for processing.
#[instrument(level = "info", name = "run_production_mode", skip(metrics))]
pub async fn run_production_mode(
    run_args: RunArgs,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event_bus = Arc::new(EventBus::with_capacity(10_000).expect("Failed to create event bus"));
    let event_bus_for_processing = event_bus.share();
    let metrics_processor = metrics.clone();

    let processor_handle =
        tokio::spawn(
            async move {
                process_events_from_bus(event_bus_for_processing.into(), metrics_processor).await
            }
            .instrument(info_span!("event_processor_task")),
        );

    let buffer_size = 1_048_576; // 1 MB
    let promiscuous = true;
    let terminate = Arc::new(AtomicBool::new(false));
    let interface = run_args.interface.clone();

    info!("Starting live capture on interface: {}", interface);

    let event_bus_for_capture = event_bus.share();
    let interface_clone = interface.clone();

    let capture_handle = tokio::spawn(
        async move {
            run_capture_loop(
                &interface_clone,
                buffer_size,
                promiscuous,
                &terminate,
                event_bus_for_capture.into(),
            )
            .await
        }
        .instrument(info_span!("capture_task")),
    );

    processor_handle.await??;
    let _ = capture_handle.await?;
    Ok(())
}

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
    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
}

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
            return; // Skip processing this event on parse error
        }
    };

    let start_time = Instant::now(); // Start timer before detection
    let matches = signature_engine.buffer_scan(packet.payload); // Use signature engine
    let elapsed_time = start_time.elapsed();

    metrics
        .detection_latency
        .observe(elapsed_time.as_nanos() as f64); // Record latency
    if !matches.is_empty() {
        handle_detection_match(metrics).await;
    }
}

#[instrument(level = "debug", name = "handle_detection_match", skip(metrics))]
async fn handle_detection_match(metrics: &MetricsRecorder) {
    let block_result =
        Firewall::new("dummy_interface") // Consider making interface configurable
            .and_then(|mut fw| fw.ip_block(std::net::Ipv4Addr::new(127, 0, 0, 1))); // Example block

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

#[instrument(level = "debug", name = "run_capture_loop", skip(terminate, event_bus))]
async fn run_capture_loop(
    interface: &str,
    buffer_size: usize,
    promiscuous: bool,
    terminate: &AtomicBool,
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

#[instrument(level = "debug", name = "enqueue_captured_packet", skip(event_bus))]
fn enqueue_captured_packet(packet: &vakthund_capture::packet::Packet, event_bus: Arc<EventBus>) {
    let event = NetworkEvent {
        timestamp: 0, // Replace with actual timestamp if available
        payload: Bytes::from(packet.data.clone()),
    };

    if let Err(e) = event_bus.try_push(event) {
        error!("Failed to push packet: {:?}", e);
    }

    let packet_size = packet.data.len();
    tokio::spawn(
        async move {
            EventLogger::log_event(
                "packet_captured",
                vec![KeyValue::new("packet_size", packet_size.to_string())],
            )
            .await;
        }
        .instrument(info_span!("log_packet_capture")),
    );

    info!("Captured packet with {} bytes", packet_size);
}

/// Simulation mode: run the simulator for a given number of events.
#[instrument(level = "info", name = "run_simulation_mode", skip(metrics))]
pub async fn run_simulation_mode(
    sim_args: SimulateArgs,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    metrics.processed_events.inc();

    let mut simulator = Simulator::new(sim_args.seed, false);
    let final_hash = simulator.run(sim_args.events);
    println!("Simulation complete. State hash: {}", final_hash);
    if let Some(expected_hash) = sim_args.validate_hash {
        assert_eq!(final_hash, expected_hash, "State hash mismatch!");
    }
    EventLogger::log_event(
        "simulation_complete",
        vec![
            KeyValue::new("event_count", sim_args.events.to_string()),
            KeyValue::new("seed", sim_args.seed.to_string()),
            KeyValue::new("final_hash", final_hash.clone()),
        ],
    )
    .await;
    Ok(())
}
