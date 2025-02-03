//! Reporting Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! This module encapsulates the functionality for generating extended snapshots of the current
//! monitor state (combined with recent event history), loading snapshots for replay, and generating
//! bug reports when errors occur. In simulation mode, the bug report will include the path to a snapshot
//! file that can later be reloaded for deterministic replay.

use chrono::Utc;
use serde_json::json;

use std::collections::VecDeque;
use std::error::Error;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use vakthund_common::config::Config;
use vakthund_common::logger::{log_error, log_info};
use vakthund_monitor::monitor::Monitor;

pub const BUG_REPORTS_FOLDER: &str = "bug_reports";

/// Generates an extended snapshot (as a JSON string) that includes the current monitor state
/// and the recent event history.
pub fn generate_extended_snapshot(
    monitor: &Arc<Mutex<Monitor>>,
    recent_events: &Arc<Mutex<VecDeque<String>>>,
) -> Result<String, Box<dyn Error>> {
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

/// Loads a snapshot from a JSON string and updates the monitor state.
/// The monitor type must implement Deserialize.
pub fn load_snapshot(
    monitor: &Arc<Mutex<Monitor>>,
    snapshot_data: &str,
) -> Result<(), Box<dyn Error>> {
    let new_state = serde_json::from_str(snapshot_data)?;
    let mut mon = monitor.lock().unwrap();
    *mon = new_state;
    log_info("Snapshot loaded and monitor state updated.");
    Ok(())
}

/// Generates a bug report as a JSON file in the BUG_REPORTS_FOLDER.
/// In simulation mode, it includes an extended snapshot (i.e. monitor state and recent events)
/// so that the bug report can be used later to replay the scenario.
pub fn generate_bug_report(
    config: &Config,
    monitor: &Arc<Mutex<Monitor>>,
    recent_events: &Arc<Mutex<VecDeque<String>>>,
    packet_id: usize,
    packet_content: &str,
    error: &str,
) {
    // Ensure the bug report folder exists.
    if !Path::new(BUG_REPORTS_FOLDER).exists() {
        if let Err(e) = create_dir_all(BUG_REPORTS_FOLDER) {
            log_error(&format!(
                "Failed to create {} folder: {}",
                BUG_REPORTS_FOLDER, e
            ));
            return;
        }
    }
    let timestamp = Utc::now().to_rfc3339();

    // In simulation mode, generate an extended snapshot.
    let snapshot_path = if config.capture.mode == vakthund_common::config::CaptureMode::Simulation {
        match generate_extended_snapshot(monitor, recent_events) {
            Ok(snapshot_data) => {
                let snapshot_file = format!("{}/snapshot_{}.json", BUG_REPORTS_FOLDER, timestamp);
                match File::create(&snapshot_file) {
                    Ok(mut file) => {
                        if let Err(e) = file.write_all(snapshot_data.as_bytes()) {
                            log_error(&format!("Failed to write snapshot file: {}", e));
                            None
                        } else {
                            Some(snapshot_file)
                        }
                    }
                    Err(e) => {
                        log_error(&format!("Failed to create snapshot file: {}", e));
                        None
                    }
                }
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

    let file_name = format!(
        "{}/bug_{}_packet_{}.json",
        BUG_REPORTS_FOLDER, timestamp, packet_id
    );
    match File::create(&file_name) {
        Ok(mut file) => {
            let report_str =
                serde_json::to_string_pretty(&bug_report).expect("Failed to serialize bug report");
            if let Err(e) = file.write_all(report_str.as_bytes()) {
                log_error(&format!("Failed to write bug report file: {}", e));
            } else {
                log_error(&format!("Bug report generated: {}", file_name));
            }
        }
        Err(e) => {
            log_error(&format!("Failed to create bug report file: {}", e));
        }
    }
}
