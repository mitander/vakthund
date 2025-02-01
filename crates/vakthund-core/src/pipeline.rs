// File: vakthund-core/src/pipeline.rs

//! # Vakthund Pipeline
//!
//! This module implements the main business pipeline for the Vakthund IDPS.
//! It loads configuration from a YAML file, sets up monitoring, and then either
//! runs live capture or a deterministic simulation. In simulation mode, the simulation
//! engine (in `simulation_engine.rs`) is used to generate packet events with embedded event IDs.
//! If a packet fails to parse, a bug report is generated with full contextual metadata.

use chrono::Utc;
use serde_json::json;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use vakthund_common::config::{CaptureMode, Config, CONFIG_FILE};
use vakthund_common::logger::{log_error, log_info, log_warn};
use vakthund_common::packet::Packet;
use vakthund_detection::analyzer::{analyze_packet, DetectionResult};
use vakthund_monitor::monitor::{Monitor, MonitorConfig};
use vakthund_protocol::parser::parse_packet;

// Import the simulation engine. Make sure your module structure re-exports it.
use crate::simulation_engine::{run_simulation, SimEvent};

/// A simple in-memory storage implementation for simulation events.
pub struct InMemoryStorage {
    events: Vec<SimEvent>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::simulation_engine::Storage for InMemoryStorage {
    fn record_event(&mut self, event: SimEvent) {
        self.events.push(event);
    }
    fn get_events(&self) -> &[SimEvent] {
        &self.events
    }
}

/// Runs the Vakthund IDPS pipeline.
pub fn run_vakthund() {
    // Load configuration.
    let config: Config = Config::load(CONFIG_FILE).unwrap_or_else(|e| {
        log_error(&format!("Failed to load config: {}", e));
        std::process::exit(1);
    });
    log_info(&format!("Configuration loaded: {:?}", config));
    log_info("Starting Vakthund IDPS pipeline.");

    // Set up monitoring.
    let mon_config = MonitorConfig::new(
        config.monitor.quarantine_timeout,
        config.monitor.thresholds.packet_rate,
        config.monitor.thresholds.data_volume,
        config.monitor.thresholds.port_entropy,
        config.monitor.whitelist.clone(),
    );
    let mut monitor = Monitor::new(&mon_config);

    // Create a termination flag.
    let terminate = Arc::new(AtomicBool::new(false));

    // Check the capture mode.
    match config.capture.mode {
        CaptureMode::Simulation => {
            // In simulation mode, run the simulation engine.
            // Optionally, you can supply a replay target event ID (e.g., Some(3)) to stop at a specific event.
            let replay_target: Option<usize> = None; // Set to Some(3) to replay the bug.
                                                     // Create an in-memory storage instance to record simulation events.
            let storage = InMemoryStorage::new();
            // Run the simulation.
            // run_simulation takes: terminate flag, optional seed, optional replay target, storage, and a callback.
            run_simulation(
                &terminate,
                config.capture.seed,
                replay_target,
                storage,
                |content: String| {
                    // For each simulated packet (content as String), convert it into a Packet and process it.
                    let packet = Packet::new(content.into_bytes());
                    monitor.process_packet(&packet);
                    if let Some(src_ip) =
                        vakthund_monitor::monitor::Monitor::extract_src_ip(&packet)
                    {
                        if monitor.is_quarantined(&src_ip) {
                            log_warn(&format!("Packet from quarantined IP {} dropped.", src_ip));
                            return;
                        }
                    }
                    match parse_packet(&packet) {
                        Ok(parsed) => match analyze_packet(&parsed) {
                            DetectionResult::ThreatDetected(details) => {
                                log_warn(&format!("Threat detected: {}", details));
                                prevent_threat(&details);
                            }
                            DetectionResult::NoThreat => {
                                log_info("Packet processed with no threat.");
                            }
                        },
                        Err(e) => {
                            let packet_content = packet.as_str().unwrap_or("<invalid UTF-8>");
                            log_error(&format!(
                                "Parsing failed for packet: {}, error: {}",
                                packet_content, e
                            ));
                            if let Some(packet_id) = extract_packet_id(packet_content) {
                                generate_bug_report(
                                    &config,
                                    packet_id,
                                    packet_content,
                                    &e.to_string(),
                                );
                            }
                        }
                    }
                },
            );
        }
        CaptureMode::Live => {
            // For live mode, you would call your live capture function.
            // Here we just log that live mode is not implemented.
            log_info("Live capture mode is not implemented in this simulation example.");
        }
    }
    log_info("Vakthund IDPS pipeline execution complete.");
}

fn prevent_threat(details: &str) {
    log_info(&format!(
        "Executing prevention action for threat: {}",
        details
    ));
}

/// Extracts the packet ID from the packet content.
/// Expects the packet content to start with "ID:<number> ".
fn extract_packet_id(content: &str) -> Option<usize> {
    if content.starts_with("ID:") {
        content
            .split_whitespace()
            .next()
            .and_then(|id_str| id_str.trim_start_matches("ID:").parse().ok())
    } else {
        None
    }
}

/// Generates a bug report as a JSON file in the `bug_reports/` folder.
fn generate_bug_report(config: &Config, packet_id: usize, packet_content: &str, error: &str) {
    let bug_folder = "bug_reports";
    if !Path::new(bug_folder).exists() {
        create_dir_all(bug_folder).expect("Failed to create bug_reports folder");
    }
    let timestamp = Utc::now().to_rfc3339();
    let file_name = format!("{}/bug_{}_packet_{}.json", bug_folder, timestamp, packet_id);
    let bug_report = json!({
        "timestamp": timestamp,
        "config": config,
        "seed": config.capture.seed,
        "packet_id": packet_id,
        "packet_content": packet_content,
        "error": error
    });
    let mut file = File::create(&file_name).expect("Failed to create bug report file");
    let report_str =
        serde_json::to_string_pretty(&bug_report).expect("Failed to serialize bug report");
    file.write_all(report_str.as_bytes())
        .expect("Failed to write bug report file");
    log_error(&format!("Bug report generated: {}", file_name));
}
