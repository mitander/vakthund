//! Vakthund Pipeline
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Implements the main business pipeline for the Vakthund IDPS. Loads configuration, sets up monitoring,
//! and runs either live capture or simulation. In simulation mode, captured packets are sent as events via
//! an event bus; a separate worker thread processes these events. Live capture uses pcap to capture packets
//! in real time.
//!
//! Parsing errors generate bug reports with detailed metadata.

use chrono::Utc;
use serde_json::json;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use vakthund_capture::live_capture::live_capture_loop;
use vakthund_common::config::{CaptureMode, Config, CONFIG_FILE};
use vakthund_common::logger::{log_error, log_info, log_warn};
use vakthund_common::packet::Packet;
use vakthund_detection::analyzer::{analyze_packet, DetectionResult};
use vakthund_monitor::monitor::MonitorConfig;
use vakthund_protocol::parser::parse_packet;
use vakthund_simulation::run_simulation;
use vakthund_simulation::storage::InMemoryStorage;

// Import the EventBus from our event bus module.
use crate::event_bus::{Event, EventBus};

pub fn run_vakthund() {
    // Load configuration.
    let config: Config = Config::load(CONFIG_FILE).unwrap_or_else(|e| {
        log_error(&format!("Failed to load config: {}", e));
        std::process::exit(1);
    });
    log_info(&format!("Configuration loaded: {:?}", config));
    log_info("Starting Vakthund IDPS pipeline.");

    // Create monitor configuration and wrap the monitor in an Arc<Mutex<_>> for sharing.
    let mon_config = MonitorConfig::new(
        config.monitor.quarantine_timeout,
        config.monitor.thresholds.packet_rate,
        config.monitor.thresholds.data_volume,
        config.monitor.thresholds.port_entropy,
        config.monitor.whitelist.clone(),
    );
    let monitor = Arc::new(Mutex::new(vakthund_monitor::monitor::Monitor::new(
        &mon_config,
    )));

    // Create a termination flag.
    let terminate = Arc::new(AtomicBool::new(false));

    match config.capture.mode {
        CaptureMode::Simulation => {
            let event_bus = EventBus::new();
            let event_sender = event_bus.get_sender();
            let monitor_for_thread = monitor.clone();
            let config_for_thread = config.clone();

            // Spawn a thread to process events.
            thread::spawn(move || {
                for crate::event_bus::Event::PacketCaptured(packet) in
                    event_bus.get_receiver().iter()
                {
                    // Lock the monitor and process the packet.
                    {
                        let mut mon = monitor_for_thread.lock().unwrap();
                        mon.process_packet(&packet);
                        if let Some(src_ip) =
                            vakthund_monitor::monitor::Monitor::extract_src_ip(&packet)
                        {
                            if mon.is_quarantined(&src_ip) {
                                log_warn(&format!(
                                    "Packet from quarantined IP {} dropped.",
                                    src_ip
                                ));
                                continue;
                            }
                        }
                    }
                    // Process parsing and analysis.
                    match parse_packet(&packet) {
                        Ok(parsed) => match analyze_packet(&parsed) {
                            vakthund_detection::analyzer::DetectionResult::ThreatDetected(
                                details,
                            ) => {
                                log_warn(&format!("Threat detected: {}", details));
                                prevent_threat(&details);
                            }
                            vakthund_detection::analyzer::DetectionResult::NoThreat => {
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
                                    &config_for_thread,
                                    packet_id,
                                    packet_content,
                                    &e.to_string(),
                                );
                            }
                        }
                    }
                }
            });
            // Run the simulation capture loop, sending events to the event bus.
            let replay_target: Option<usize> = None;
            let storage = InMemoryStorage::new();
            run_simulation(
                &terminate,
                config.capture.seed,
                replay_target,
                storage,
                |content: String| {
                    let packet = Packet::new(content.into_bytes());
                    event_sender
                        .send(Event::PacketCaptured(packet))
                        .expect("Failed to send event");
                },
            );
        }
        CaptureMode::Live => {
            live_capture_loop(
                &config.capture.interface,
                config.capture.buffer_size,
                config.capture.promiscuous,
                &terminate,
                &mut |packet: Packet| {
                    // Lock monitor for processing.
                    {
                        let mut mon = monitor.lock().unwrap();
                        mon.process_packet(&packet);
                        if let Some(src_ip) =
                            vakthund_monitor::monitor::Monitor::extract_src_ip(&packet)
                        {
                            if mon.is_quarantined(&src_ip) {
                                log_warn(&format!(
                                    "Packet from quarantined IP {} dropped.",
                                    src_ip
                                ));
                                return;
                            }
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
    }
    log_info("Vakthund IDPS pipeline execution complete.");
}

fn prevent_threat(details: &str) {
    log_info(&format!(
        "Executing prevention action for threat: {}",
        details
    ));
}

/// Extracts the packet ID from the packet content (expects content to start with "ID:<number> ").
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
