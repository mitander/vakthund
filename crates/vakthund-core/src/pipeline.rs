//! Vakthund Pipeline
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Loads configuration, sets up monitoring, and runs capture (live or simulation).
//! In simulation mode, deterministic events are generated and errors trigger bug reports.

use chrono::Utc;
use serde_json::json;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use vakthund_common::config::{CaptureMode, Config, CONFIG_FILE};
use vakthund_common::logger::{log_error, log_info, log_warn};
use vakthund_common::packet::Packet;
use vakthund_detection::analyzer::{analyze_packet, DetectionResult};
use vakthund_monitor::monitor::{Monitor, MonitorConfig};
use vakthund_protocol::parser::parse_packet;
// For live capture, assume a separate implementation (not provided here).
use vakthund_capture::simulate_capture_loop;
use vakthund_simulation::storage::InMemoryStorage;
use vakthund_simulation::{compute_event_hash, run_simulation};

pub fn run_vakthund() {
    let config: Config = Config::load(CONFIG_FILE).unwrap_or_else(|e| {
        log_error(&format!("Failed to load config: {}", e));
        std::process::exit(1);
    });
    log_info(&format!("Configuration loaded: {:?}", config));
    log_info("Starting Vakthund IDPS pipeline.");

    let mon_config = MonitorConfig::new(
        config.monitor.quarantine_timeout,
        config.monitor.thresholds.packet_rate,
        config.monitor.thresholds.data_volume,
        config.monitor.thresholds.port_entropy,
        config.monitor.whitelist.clone(),
    );
    let mut monitor = Monitor::new(&mon_config);
    let terminate = Arc::new(AtomicBool::new(false));

    match config.capture.mode {
        CaptureMode::Simulation => {
            let replay_target: Option<usize> = None;
            let storage = InMemoryStorage::new();
            run_simulation(
                &terminate,
                config.capture.seed,
                replay_target,
                storage,
                |content: String| {
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
            // Live capture not implemented in this example.
            log_info("Live capture mode not implemented in this example.");
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

/// Extracts the packet ID from content (expects "ID:<number> ").
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
