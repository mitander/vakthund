//! Vakthund Pipeline
//!
//! Proprietary and confidential. All rights reserved.
//!
//! This module loads configuration, sets up monitoring, and runs the capture system in a unified,
//! event‑driven architecture. Both live capture (using pcap) and simulation capture push events into
//! a common event bus. A dedicated worker thread processes events via an EventProcessor, which
//! dispatches events (PacketCaptured, AlertRaised, PreventionAction, SnapshotTaken) to their handlers.
//!
//! When an error occurs during packet processing, the bug report callback is invoked to generate a bug report.
//! In simulation mode, the bug report includes an extended snapshot (monitor state and recent event history)
//! that can later be loaded for replay.

use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;

use vakthund_capture::live_capture::live_capture_loop;
use vakthund_common::config::{CaptureMode, Config, CONFIG_FILE};
use vakthund_common::logger::{init_logger, log_error, log_info};
use vakthund_common::packet::Packet;
use vakthund_monitor::monitor::Monitor;
use vakthund_simulation::run_simulation;
use vakthund_simulation::storage::InMemoryStorage;

use crate::event_bus::{Event, EventBus};
use crate::event_processor::{BugReporter, EventProcessor};
use crate::reporting::{generate_bug_report, load_snapshot};

const RECENT_EVENTS_CAPACITY: usize = 100;

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

    // Create the monitor (using config from common).
    let monitor = Arc::new(Mutex::new(Monitor::new(&config.monitor)));

    // If replay_hash is provided, load the snapshot from that file.
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

    log_info("Starting Vakthund IDPS pipeline.");

    // Create a ring buffer to record recent events.
    let recent_events: Arc<Mutex<VecDeque<String>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(RECENT_EVENTS_CAPACITY)));

    // Create a termination flag.
    let terminate = Arc::new(AtomicBool::new(false));

    // Create a unified event bus.
    let event_bus = EventBus::new();
    let event_sender = event_bus.get_sender();

    // Clone necessary values for the bug report callback.
    let config_for_bug = config.clone();
    let monitor_for_bug = monitor.clone();
    let recent_events_for_bug = recent_events.clone();

    // Create a bug report callback as an Arc<Box<dyn BugReporter>>.
    let bug_report_cb: Arc<Box<dyn BugReporter>> = Arc::new(Box::new(
        move |packet_id: usize, packet_content: &str, error: &str| {
            generate_bug_report(
                &config_for_bug,
                &monitor_for_bug,
                &recent_events_for_bug,
                packet_id,
                packet_content,
                error,
            );
        },
    ));

    // Create an EventProcessor instance with the bug report callback.
    let event_processor = EventProcessor::new(config.clone(), monitor.clone(), Some(bug_report_cb));

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
}
