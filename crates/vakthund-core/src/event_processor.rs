//! Event Processor Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! This module dispatches transient events to their appropriate handlers. It does not maintain
//! a heavyweight global state but simply processes events as they arrive. The processor handles
//! packet capture, alerts, prevention actions, and snapshots. Future external integrations (e.g.,
//! notifications, firewall updates) can be added by extending the respective handler functions.

use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde_json;
use std::error::Error;
use std::process::Command;
use std::sync::{Arc, Mutex};
use vakthund_common::config::Config;
use vakthund_common::logger::{log_error, log_info, log_warn};
use vakthund_common::packet::Packet;
use vakthund_detection::analyzer::{analyze_packet, DetectionResult};
use vakthund_monitor::monitor::Monitor;
use vakthund_protocol::parser::parse_packet;

/// The EventProcessor dispatches events to the appropriate handler functions.
pub struct EventProcessor {
    pub config: Config,
    pub monitor: Arc<Mutex<Monitor>>,
}

impl EventProcessor {
    /// Creates a new EventProcessor.
    pub fn new(config: Config, monitor: Arc<Mutex<Monitor>>) -> Self {
        Self { config, monitor }
    }

    /// Handles a PacketCaptured event.
    pub fn handle_packet(&self, packet: Packet) {
        // Update monitor state.
        {
            let mut mon = self.monitor.lock().unwrap();
            mon.process_packet(&packet);
            if let Some(src_ip) = Monitor::extract_src_ip(&packet) {
                if mon.is_quarantined(&src_ip) {
                    log_warn(&format!("Packet from quarantined IP {} dropped.", src_ip));
                    return;
                }
            }
        }
        // Parse and analyze the packet.
        match parse_packet(&packet) {
            Ok(parsed) => match analyze_packet(&parsed) {
                DetectionResult::ThreatDetected(details) => {
                    log_warn(&format!("Threat detected: {}", details));
                    self.handle_alert(&details, packet.clone());
                    self.handle_prevention("Drop", packet.clone());
                    self.handle_snapshot("Snapshot taken after prevention action");
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
                // You could generate a bug report here if desired.
            }
        }
    }

    /// Handles an AlertRaised event by sending an email notification.
    pub fn handle_alert(&self, details: &str, packet: Packet) {
        log_warn(&format!("ALERT: {}. Packet: {:?}", details, packet));
        if let Err(e) = send_mail_alarm(details, &packet) {
            log_error(&format!("Failed to send alert email: {}", e));
        }
    }

    /// Handles a PreventionAction event by applying a firewall rule.
    pub fn handle_prevention(&self, action: &str, packet: Packet) {
        log_info(&format!(
            "Prevention action triggered: {} on packet: {:?}",
            action, packet
        ));
        if let Some(src_ip) = Monitor::extract_src_ip(&packet) {
            if let Err(e) = apply_firewall_rule(action, &src_ip) {
                log_error(&format!("Failed to apply firewall rule: {}", e));
            }
        }
    }

    /// Handles a SnapshotTaken event by generating a snapshot.
    pub fn handle_snapshot(&self, snapshot_info: &str) {
        log_info(&format!("Snapshot event: {}", snapshot_info));
        match generate_snapshot(&self.monitor) {
            Ok(snapshot_data) => {
                log_info(&format!("Snapshot generated: {}", snapshot_data));
                // Future: persist or load this snapshot as needed.
            }
            Err(e) => {
                log_error(&format!("Failed to generate snapshot: {}", e));
            }
        }
    }
}

/// Sends an alert email using lettre.
fn send_mail_alarm(details: &str, packet: &Packet) -> Result<(), Box<dyn Error>> {
    let email = Message::builder()
        .from("alert@vakthund.com".parse()?)
        .to("admin@vakthund.com".parse()?)
        .subject("Vakthund Alert Notification")
        .body(format!("Alert details: {}\nPacket: {:?}", details, packet))?;

    let creds = Credentials::new("username".into(), "password".into());
    let mailer = SmtpTransport::relay("smtp.vakthund.com")?
        .credentials(creds)
        .build();

    mailer.send(&email)?;
    log_info("Alert email sent successfully.");
    Ok(())
}

/// Applies a firewall rule using iptables. For "Drop", adds a rule to drop packets from the given IP.
fn apply_firewall_rule(action: &str, src_ip: &str) -> Result<(), Box<dyn Error>> {
    let rule = match action {
        "Drop" => format!("-A INPUT -s {} -j DROP", src_ip),
        "Allow" => format!("-D INPUT -s {} -j DROP", src_ip),
        _ => return Err("Unknown firewall action".into()),
    };
    let output = Command::new("iptables")
        .args(rule.split_whitespace())
        .output()?;
    if !output.status.success() {
        return Err(format!("iptables command failed: {:?}", output).into());
    }
    log_info(&format!("Firewall rule applied: {}", rule));
    Ok(())
}

/// Generates a snapshot of the current monitor state as a JSON string.
/// Assumes the Monitor implements Serialize.
fn generate_snapshot(monitor: &Arc<Mutex<Monitor>>) -> Result<String, Box<dyn Error>> {
    let mon = monitor.lock().unwrap();
    let snapshot = serde_json::to_string(&*mon)?;
    Ok(snapshot)
}
