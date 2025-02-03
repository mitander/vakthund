//! Event Processor Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! This module dispatches events to their respective handlers. It integrates alert functionality
//! via multiple channels (syslog or email), active prevention via iptables, and snapshot generation.
//! The alert method is configurable via an enum. This design is modular and extensible.

use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde_json;

use std::error::Error;
use std::process::Command;
use std::sync::{Arc, Mutex};

use vakthund_common::config::{AlertMethod, Config};
use vakthund_common::logger::{log_error, log_info, log_warn};
use vakthund_common::packet::Packet;
use vakthund_detection::analyzer::{analyze_packet, DetectionResult};
use vakthund_monitor::monitor::Monitor;
use vakthund_protocol::parser::parse_packet;

/// The EventProcessor dispatches events to appropriate handlers.
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
            }
        }
    }

    /// Handles an AlertRaised event by sending an alert using the configured method.
    pub fn handle_alert(&self, details: &str, packet: Packet) {
        log_warn(&format!("ALERT: {}. Packet: {:?}", details, packet));
        if let Err(e) = send_alert(&self.config.alert_methods, details, &packet) {
            log_error(&format!("Failed to send alert: {}", e));
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
            }
            Err(e) => {
                log_error(&format!("Failed to generate snapshot: {}", e));
            }
        }
    }
}

/// Sends an alert using all configured alert methods.
fn send_alert(
    alert_methods: &Vec<AlertMethod>,
    details: &str,
    packet: &Packet,
) -> Result<(), Box<dyn Error>> {
    for method in alert_methods {
        match method {
            AlertMethod::Syslog => send_syslog_alert(details, packet)?,
            AlertMethod::Email => send_mail_alert(details, packet)?,
        }
    }
    Ok(())
}
/// Sends an alert via syslog.
fn send_syslog_alert(details: &str, packet: &Packet) -> Result<(), Box<dyn Error>> {
    use syslog::{Facility, Formatter3164};
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: "vakthund".into(),
        pid: 0,
    };
    let mut logger = syslog::unix(formatter)?;
    logger.err(&format!("ALERT: {}. Packet: {:?}", details, packet))?;
    Ok(())
}

/// Sends an alert email using lettre.
fn send_mail_alert(details: &str, packet: &Packet) -> Result<(), Box<dyn Error>> {
    let email = Message::builder()
        .from("alert@kapsel.com".parse()?)
        .to("admin@kapsel.com".parse()?)
        .subject("Vakthund Alert Notification")
        .body(format!("Alert details: {}\nPacket: {:?}", details, packet))?;

    let creds = Credentials::new("username".into(), "password".into());
    let mailer = SmtpTransport::relay("smtp.kapsel.com")?
        .credentials(creds)
        .build();

    mailer.send(&email)?;
    Ok(())
}

/// Applies a firewall rule using iptables.
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
    Ok(())
}

/// Generates a snapshot of the current monitor state as a JSON string.
fn generate_snapshot(monitor: &Arc<Mutex<Monitor>>) -> Result<String, Box<dyn Error>> {
    let mon = monitor.lock().unwrap();
    let snapshot = serde_json::to_string(&*mon)?;
    Ok(snapshot)
}
