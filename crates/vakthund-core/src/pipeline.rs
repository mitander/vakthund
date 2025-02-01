//! # Pipeline Module
//!
//! Contains the main pipeline logic for capturing, monitoring, parsing, analyzing,
//! and executing prevention actions in the Vakthund IDPS.
//!
//! The pipeline loads configuration from a YAML file (using `CONFIG_FILE` from the config module),
//! selects the appropriate packet source (live or simulation), and processes each packet as it arrives.

use vakthund_capture::start_capture;
use vakthund_common::config::{Config, CONFIG_FILE};
use vakthund_common::logger::{log_error, log_info, log_warn};
use vakthund_detection::analyzer::{analyze_packet, DetectionResult};
use vakthund_monitor::monitor::{Monitor, MonitorConfig};
use vakthund_protocol::parser::parse_packet;

pub fn run_vakthund() {
    // Load configuration from the constant CONFIG_FILE.
    let config: Config = Config::load(CONFIG_FILE).unwrap_or_else(|e| {
        log_error(&format!("Failed to load config: {}", e));
        std::process::exit(1);
    });
    log_info(&format!("Configuration loaded: {:?}", config));
    log_info("Starting Vakthund IDPS pipeline.");

    // Set up monitoring using the monitor configuration.
    let mon_config = MonitorConfig::new(
        config.monitor.quarantine_timeout,
        config.monitor.thresholds.packet_rate,
        config.monitor.thresholds.data_volume,
        config.monitor.thresholds.port_entropy,
        config.monitor.whitelist,
    );
    let mut monitor = Monitor::new(&mon_config);

    // Start capture with the unified interface.
    start_capture(
        &config.capture.mode,
        &config.capture.interface,
        config.capture.buffer_size,
        config.capture.promiscuous,
        |packet| {
            // Update monitor and check for quarantine.
            monitor.process_packet(&packet);
            if let Some(src_ip) = vakthund_monitor::monitor::Monitor::extract_src_ip(&packet) {
                if monitor.is_quarantined(&src_ip) {
                    log_warn(&format!("Packet from quarantined IP {} dropped.", src_ip));
                    return; // Drop the packet.
                }
            }
            // Parse and analyze the packet.
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
                    log_error(&format!(
                        "Parsing failed for packet: {:?}, error: {}",
                        packet.as_str(),
                        e
                    ));
                }
            }
        },
    );
    log_info("Vakthund IDPS pipeline execution complete.");
}

/// Executes a prevention action for a detected threat.
///
/// In a full implementation, this might block offending traffic,
/// update firewall rules, or perform other countermeasures. Here we log a message.
fn prevent_threat(details: &str) {
    log_info(&format!(
        "Executing prevention action for threat: {}",
        details
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use vakthund_common::packet::Packet;
    use vakthund_detection::analyzer::DetectionResult;

    #[test]
    fn test_pipeline_with_valid_packets() {
        // This test validates that packets can be parsed and analyzed.
        let packet1 = Packet::new("MQTT CONNECT alert/home_sim_test".as_bytes().to_vec());
        let packet2 = Packet::new("INFO system_ok_sim_test".as_bytes().to_vec());
        for packet in [packet1, packet2] {
            let res = parse_packet(&packet);
            assert!(res.is_ok());
        }
        if let Ok(parsed) = parse_packet(&Packet::new(
            "MQTT CONNECT alert/home_sim_test".as_bytes().to_vec(),
        )) {
            let detection = analyze_packet(&parsed);
            assert_eq!(
                detection,
                DetectionResult::ThreatDetected("MQTT CONNECT alert".into())
            );
        }
    }
}
