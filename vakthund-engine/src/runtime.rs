//! # Runtime Engine
//!
//! This module provides the core runtime functionality for Vakthund:
//! - Production mode (live network capture)
//! - Simulation mode (controlled environment testing)
//! - Scenario replay mode (deterministic event replay)
use std::{
    fmt::Debug,
    fs::File,
    io::Write,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use opentelemetry::KeyValue;
use tracing::{debug, error, info, instrument, Instrument};

use vakthund_capture::capture;
use vakthund_config::{SimulatorConfig, VakthundConfig};
use vakthund_core::events::{bus::EventBus, network::NetworkEvent};
use vakthund_detection::signatures::SignatureEngine;
use vakthund_prevention::firewall::Firewall;
use vakthund_protocols::AnyParser;
use vakthund_protocols::{CoapParser, ModbusParser, MqttParser};
use vakthund_simulator::{replay::Scenario, Simulator};
use vakthund_telemetry::{logging::EventLogger, metrics::MetricsRecorder};

/// Main production mode execution flow
#[instrument(level = "info", name = "run_production_mode", skip(metrics))]
pub async fn run_production_mode(
    interface: &str,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = VakthundConfig::load_from_path("config/vakthund.yaml")
        .unwrap_or_else(|e| panic!("Configuration load failed: {}", e));

    info!("Loaded production configuration: {:?}", config);

    let event_bus = Arc::new(
        EventBus::with_capacity(config.core.event_bus.capacity)
            .expect("Failed to create event bus"),
    );

    let (processor_handle, capture_handle) = spawn_production_tasks(
        interface.to_string(),
        config.capture.buffer_size,
        config.capture.promiscuous,
        Arc::clone(&event_bus),
        metrics,
    )
    .await;
    processor_handle.await??;
    capture_handle.await??;
    Ok(())
}

/// Main simulation mode execution flow
#[instrument(level = "info", name = "run_simulation_mode", skip(metrics))]
pub async fn run_simulation_mode<P: AsRef<Path> + Debug>(
    sim_config: SimulatorConfig,
    scenario_path: Option<P>,
    num_events: usize,
    seed: u64,
    validate_hash: Option<&str>,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match scenario_path {
        Some(path) => {
            let scenario = Scenario::load_from_path(path)?;
            let mut simulator = Simulator::from_scenario(&scenario);

            simulator.replay_events(scenario.events.clone()).await?;
            validate_simulation_hash(&scenario, simulator.finalize_hash())?;
            Ok(())
        }
        None => {
            let config = VakthundConfig::load()?;
            debug!("Loaded configuration {:?}", config);

            let effective_seed = if seed != 0 { seed } else { sim_config.seed };
            let event_bus = Arc::new(
                EventBus::with_capacity(config.core.event_bus.capacity)
                    .context("Event bus creation failed")?,
            );

            let simulator = Simulator::new(
                seed,
                sim_config.chaos.fault_probability > 0.0,
                sim_config.network.latency_ms,
                sim_config.network.jitter_ms,
                Some(Arc::clone(&event_bus)),
            );

            let (processor_handle, simulator_handle) =
                spawn_simulation_tasks(event_bus, metrics, num_events, simulator).await;

            let (_, final_hash) = simulator_handle.await??;
            processor_handle.await??;

            validate_simulation_result(validate_hash, &final_hash, &config)?;
            EventLogger::log_event(
                "simulation_complete",
                vec![
                    KeyValue::new("event_count", num_events.to_string()),
                    KeyValue::new("seed", effective_seed.to_string()),
                    KeyValue::new("final_hash", final_hash.to_string()),
                ],
            )
            .await;

            Ok(())
        }
    }
}

/// Main fuzz mode execution flow
#[instrument(level = "info", name = "run_fuzz_mode", skip(metrics))]
pub async fn run_fuzz_mode(
    mut seed: u64,
    iterations: usize,
    max_events: usize,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut count = 0;
    loop {
        let sim_config = SimulatorConfig::generate_fuzz_config(seed);
        let _ = run_simulation_mode::<PathBuf>(
            sim_config,
            None,
            max_events,
            seed,
            None,
            metrics.clone(),
        )
        .await;

        if iterations > 0 && count >= iterations {
            break;
        }

        seed += 1;
        count += 1;
    }
    Ok(())
}

/// Spawns parallel tasks for simulation processing
async fn spawn_simulation_tasks(
    event_bus: Arc<EventBus>,
    metrics: MetricsRecorder,
    num_events: usize,
    mut simulator: Simulator,
) -> (
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    tokio::task::JoinHandle<Result<(Simulator, String), Box<dyn std::error::Error + Send + Sync>>>,
) {
    let event_bus_clone = Arc::clone(&event_bus);
    let processor_handle = tokio::spawn(
        async move { process_events_from_bus(event_bus_clone, metrics).await }
            .instrument(tracing::info_span!("event_processor")),
    );

    let event_bus_sim = Arc::clone(&event_bus);
    let simulator_handle = tokio::spawn(async move {
        for event_id in 0..num_events {
            if let Some(event) = simulator.simulate_event(event_id) {
                event_bus_sim.send_blocking(event);
            }
        }
        let final_hash = simulator.finalize_hash();
        Ok((simulator, final_hash))
    });

    (processor_handle, simulator_handle)
}

/// Event processing core logic
#[instrument(level = "debug", skip(event_bus, metrics))]
async fn process_events_from_bus(
    event_bus: Arc<EventBus>,
    metrics: MetricsRecorder,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let signature_engine = SignatureEngine::new();

    while let Some(event) = event_bus.recv() {
        process_network_event(&event, &signature_engine, &metrics).await;
        debug!("Processed event from {:?}", event.source);
    }

    Ok(())
}

/// Network event processing implementation
#[instrument(level = "debug", skip(signature_engine, metrics))]
async fn process_network_event(
    event: &NetworkEvent,
    signature_engine: &SignatureEngine,
    metrics: &MetricsRecorder,
) {
    // Unified parser handling
    let parsers = [
        AnyParser::Mqtt(MqttParser::new()),
        AnyParser::Coap(CoapParser::new()),
        AnyParser::Modbus(ModbusParser::new()),
    ];

    for parser in parsers {
        match parser {
            AnyParser::Mqtt(p) => {
                if let Ok(packet) = p.parse(&event.payload) {
                    let start_time = Instant::now();
                    let matches = signature_engine.buffer_scan(packet.payload());
                    metrics
                        .detection_latency
                        .observe(start_time.elapsed().as_nanos() as f64);

                    if !matches.is_empty() {
                        handle_detection_match(metrics).await;
                    }
                    break;
                }
            }
            AnyParser::Coap(p) => {
                if let Ok(packet) = p.parse(&event.payload) {
                    let start_time = Instant::now();
                    let matches = signature_engine.buffer_scan(packet.payload());
                    metrics
                        .detection_latency
                        .observe(start_time.elapsed().as_nanos() as f64);

                    if !matches.is_empty() {
                        handle_detection_match(metrics).await;
                    }
                    break;
                }
            }
            AnyParser::Modbus(p) => {
                if let Ok(packet) = p.parse(&event.payload) {
                    let start_time = Instant::now();
                    let matches = signature_engine.buffer_scan(packet.payload());
                    metrics
                        .detection_latency
                        .observe(start_time.elapsed().as_nanos() as f64);

                    if !matches.is_empty() {
                        handle_detection_match(metrics).await;
                    }
                    break;
                }
            }
        }
    }
}

/// Detection match handler with firewall integration
#[instrument(level = "debug", skip(metrics))]
async fn handle_detection_match(metrics: &MetricsRecorder) {
    const BLOCK_IP: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

    match Firewall::new("eth0").and_then(|mut fw| fw.block_ip(BLOCK_IP)) {
        Ok(_) => {
            metrics.processed_events.inc();
            EventLogger::log_event(
                "firewall_block",
                vec![KeyValue::new("ip_address", BLOCK_IP.to_string())],
            )
            .await;
        }
        Err(e) => {
            error!("Firewall error: {}", e);
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

/// Validation and logging helpers
fn validate_simulation_hash(
    scenario: &Scenario,
    actual_hash: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if actual_hash != scenario.expected_hash {
        let error = format!(
            "Hash mismatch!\nExpected: {}\nActual: {}",
            scenario.expected_hash, actual_hash
        );
        generate_bug_report(&error);
        Err(error.into())
    } else {
        Ok(())
    }
}

fn validate_simulation_result(
    expected_hash: Option<&str>,
    actual_hash: &str,
    config: &VakthundConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(expected) = expected_hash {
        if actual_hash != expected {
            let error = format!(
                "Validation failed!\nExpected: {}\nActual: {}\nConfig: {:?}",
                expected, actual_hash, config
            );
            generate_bug_report(&error);
            return Err(error.into());
        }
    }
    Ok(())
}

async fn spawn_production_tasks(
    interface: String,
    buffer_size: usize,
    promiscuous: bool,
    event_bus: Arc<EventBus>,
    metrics: MetricsRecorder,
) -> (
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    let terminate = Arc::new(AtomicBool::new(false));
    let event_bus_clone = Arc::clone(&event_bus);

    // Inside spawn_production_tasks
    let interface_clone = interface.clone();
    let capture_handle = tokio::spawn(async move {
        capture::run_capture_loop(
            &interface_clone, // Use cloned String
            buffer_size,
            promiscuous,
            &terminate,
            |packet| {
                let event = NetworkEvent::new(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64,
                    packet.data.clone(),
                );
                event_bus_clone.send_blocking(event);
            },
        );
        Ok(())
    });
    // Spawn the processing task
    let processor_handle =
        tokio::spawn(async move { process_events_from_bus(event_bus, metrics).await });

    (processor_handle, capture_handle)
}

/// Generates a bug report file with diagnostic information
pub fn generate_bug_report(report: &str) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let filename = format!("bug_{}.yaml", timestamp);

    File::create(&filename)
        .and_then(|mut f| f.write_all(report.as_bytes()))
        .unwrap_or_else(|_| panic!("Failed to create bug report {}", filename));

    println!("Bug report saved to {}", filename);
}

/// Saves simulation state to a scenario file
pub fn save_scenario(simulator: &Simulator, seed: u64, config: &SimulatorConfig) {
    let scenario = Scenario {
        seed,
        config: config.clone(),
        events: simulator.get_recorded_events(),
        expected_hash: simulator.finalize_hash(),
    };

    scenario
        .save_to_file(format!("scenario_{}.vscenario", seed))
        .unwrap_or_else(|_| panic!("Failed to save scenario for seed {}", seed));
}
