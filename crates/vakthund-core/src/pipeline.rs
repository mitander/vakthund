//! Vakthund Pipeline
//!
//! Proprietary and confidential. All rights reserved.
//!
//! This module loads configuration, sets up monitoring, and runs the capture system in a unified,
//! event‑driven architecture. Both live capture (using pcap) and simulation capture push events into
//! a common event bus. A dedicated worker thread processes events via an EventProcessor, which
//! dispatches events (PacketCaptured, AlertRaised, PreventionAction, SnapshotTaken) to appropriate handlers.
//!
//! When an error occurs, a bug report is generated. In simulation mode, the bug report includes an extended
//! snapshot (monitor state and recent event history) that can later be loaded for replay.

use chrono::Utc;
use serde_json::json;

use std::collections::VecDeque;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;

use vakthund_capture::live_capture::live_capture_loop;
use vakthund_common::config::{CaptureMode, Config, CONFIG_FILE};
use vakthund_common::logger::{init_logger, log_error, log_info};
use vakthund_common::packet::Packet;
use vakthund_monitor::Monitor;
use vakthund_simulation::run_simulation;
use vakthund_simulation::storage::InMemoryStorage;

use crate::event_bus::{Event, EventBus};
use crate::event_processor::EventProcessor;

const RECENT_EVENTS_CAPACITY: usize = 100;

/// Generates an extended snapshot that includes monitor state and recent events.
fn generate_extended_snapshot(
    monitor: &Arc<Mutex<Monitor>>,
    recent_events: &Arc<Mutex<VecDeque<String>>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mon = monitor.lock().unwrap();
    let monitor_snapshot = serde_json::to_string(&*mon)?;
    let events = recent_events.lock().unwrap();
    let events_snapshot: Vec<String> = events.iter().cloned().collect();
    let extended_snapshot = json!({
        "monitor": serde_json::from_str::<serde_json::Value>(&monitor_snapshot)?,
        "recent_events": events_snapshot,
    });
    Ok(extended_snapshot.to_string())
}

/// Generates a bug report as a JSON file in the `bug_reports/` folder.
/// In simulation mode, it includes an extended snapshot with monitor state and recent events.
fn generate_bug_report(
    config: &Config,
    monitor: &Arc<Mutex<Monitor>>,
    recent_events: &Arc<Mutex<VecDeque<String>>>,
    packet_id: usize,
    packet_content: &str,
    error: &str,
) {
    let bug_folder = "bug_reports";
    if !Path::new(bug_folder).exists() {
        create_dir_all(bug_folder).expect("Failed to create bug_reports folder");
    }
    let timestamp = Utc::now().to_rfc3339();
    let snapshot_path = if config.capture.mode == CaptureMode::Simulation {
        match generate_extended_snapshot(monitor, recent_events) {
            Ok(snapshot_data) => {
                let snapshot_file = format!("{}/snapshot_{}.json", bug_folder, timestamp);
                let mut file =
                    File::create(&snapshot_file).expect("Failed to create snapshot file");
                file.write_all(snapshot_data.as_bytes())
                    .expect("Failed to write snapshot file");
                Some(snapshot_file)
            }
            Err(e) => {
                log_error(&format!("Failed to generate extended snapshot: {}", e));
                None
            }
        }
    } else {
        None
    };
    let bug_report = json!({
        "timestamp": timestamp,
        "config": config,
        "seed": config.capture.seed,
        "packet_id": packet_id,
        "packet_content": packet_content,
        "error": error,
        "snapshot": snapshot_path
    });
    let file_name = format!("{}/bug_{}_packet_{}.json", bug_folder, timestamp, packet_id);
    let mut file = File::create(&file_name).expect("Failed to create bug report file");
    let report_str =
        serde_json::to_string_pretty(&bug_report).expect("Failed to serialize bug report");
    file.write_all(report_str.as_bytes())
        .expect("Failed to write bug report file");
    log_error(&format!("Bug report generated: {}", file_name));
}

/// Extracts the packet ID from packet content (expects "ID:<number> " prefix).
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

// Helper function to load a snapshot from a JSON string and update the monitor state.
// This requires that your Monitor type implements Deserialize.
fn load_snapshot(
    monitor: &Arc<Mutex<Monitor>>,
    snapshot_data: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let new_state = serde_json::from_str(snapshot_data)?;
    let mut mon = monitor.lock().unwrap();
    *mon = new_state;
    log_info("Snapshot loaded and monitor state updated.");
    Ok(())
}

pub fn run_vakthund(sim_seed_override: Option<u64>, replay_hash: Option<&str>) {
    init_logger();

    // Load configuration from config.yaml.
    let mut config: Config = Config::load(CONFIG_FILE).unwrap_or_else(|e| {
        log_error(&format!("Failed to load config: {}", e));
        std::process::exit(1);
    });
    log_info(&format!("Configuration loaded: {:?}", config));

    // Override simulation seed if provided via CLI.
    if let Some(seed) = sim_seed_override {
        log_info(&format!(
            "Overriding simulation seed with CLI flag: {}",
            seed
        ));
        config.capture.seed = Some(seed);
    }

    // If replay_hash is provided, adjust config or internal state accordingly.
    if let Some(hash) = replay_hash {
        log_info(&format!("Replaying simulation for hash: {}", hash));
        // For example, set a replay flag or update config with replay target.
        // You might need to parse the hash (e.g., extract seed and packet id) and adjust your simulation accordingly.
        config.capture.mode = CaptureMode::Simulation; // Ensure simulation mode is set.
                                                       // Additional logic here: e.g., config.replay_target = Some(parsed_target);
    }

    log_info("Starting Vakthund IDPS pipeline.");

    let monitor = Arc::new(Mutex::new(Monitor::new(&config.monitor)));

    // If the replay_simulation flag is provided, load the snapshot from that file.
    if let Some(snapshot_path) = replay_hash {
        log_info(&format!(
            "Replay simulation flag provided: {}. Loading snapshot...",
            snapshot_path
        ));
        match std::fs::read_to_string(snapshot_path) {
            Ok(snapshot_data) => {
                if let Err(e) = load_snapshot(&monitor, &snapshot_data) {
                    log_error(&format!("Failed to load snapshot: {}", e));
                } else {
                    log_info("Snapshot loaded successfully.");
                }
            }
            Err(e) => {
                log_error(&format!(
                    "Failed to read snapshot file {}: {}",
                    snapshot_path, e
                ));
            }
        }
    }
    // Create a ring buffer to record recent events.
    let recent_events: Arc<Mutex<VecDeque<String>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(RECENT_EVENTS_CAPACITY)));

    // Create a termination flag.
    let terminate = Arc::new(AtomicBool::new(false));

    // Create a unified event bus.
    let event_bus = EventBus::new();
    let event_sender = event_bus.get_sender();

    // Create an EventProcessor instance.
    let event_processor = EventProcessor::new(config.clone(), monitor.clone());

    // Spawn a worker thread to process events.
    let recent_events_for_thread = recent_events.clone();
    thread::spawn(move || {
        for event in event_bus.get_receiver().iter() {
            {
                let mut events = recent_events_for_thread.lock().unwrap();
                events.push_back(format!("{:?}", event));
                if events.len() > RECENT_EVENTS_CAPACITY {
                    events.pop_front();
                }
            }
            match event {
                Event::PacketCaptured(packet) => {
                    event_processor.handle_packet(packet);
                }
                Event::AlertRaised { details, packet } => {
                    event_processor.handle_alert(&details, packet);
                }
                Event::PreventionAction { action, packet } => {
                    event_processor.handle_prevention(&action, packet);
                }
                Event::SnapshotTaken { snapshot_data } => {
                    event_processor.handle_snapshot(&snapshot_data);
                }
            }
        }
    });

    // Both live and simulation capture push PacketCaptured events into the event bus.
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
                    event_sender
                        .send(Event::PacketCaptured(packet))
                        .expect("Failed to send event");
                },
            );
        }
    }
    log_info("Vakthund IDPS pipeline execution complete.");

    // For demonstration: if GENERATE_BUG_REPORT is set, generate a dummy bug report.
    if env::var("GENERATE_BUG_REPORT").is_ok() {
        let dummy_packet_content = "ID:999 Dummy error packet";
        if let Some(packet_id) = extract_packet_id(dummy_packet_content) {
            generate_bug_report(
                &config,
                &monitor,
                &recent_events,
                packet_id,
                dummy_packet_content,
                "Dummy error triggered bug report",
            );
        }
    }
}
