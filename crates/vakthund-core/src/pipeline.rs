//! # Pipeline Module
//!
//! Contains the main pipeline logic for capturing, parsing, analyzing, and preventing threats.
//!
//! This is the heart of the Vakthund IDPS. It integrates packet capture, protocol parsing,
//! threat analysis, and prevention actions. When a threat is detected, a prevention action is taken.

use vakthund_capture::simulate_capture;
use vakthund_common::logger::{log_error, log_info, log_warn};
use vakthund_detection::analyzer::{analyze_packet, DetectionResult};
use vakthund_protocol::parser::parse_packet;

pub fn run_vakthund() {
    log_info("Starting Vakthund IDPS pipeline.");
    let packets = simulate_capture();
    for packet in packets {
        match parse_packet(&packet) {
            Ok(parsed) => {
                match analyze_packet(&parsed) {
                    DetectionResult::ThreatDetected(details) => {
                        log_warn(&format!("Threat detected: {}", details));
                        // Borrow details as &str to match the function signature.
                        prevent_threat(&details);
                    }
                    DetectionResult::NoThreat => {
                        log_info("Packet processed with no threat.");
                    }
                }
            }
            Err(e) => {
                log_error(&format!(
                    "Parsing failed for packet: {:?}, error: {}",
                    packet.as_str(),
                    e
                ));
            }
        }
    }
    log_info("Vakthund IDPS pipeline execution complete.");
}

/// Executes a prevention action for a detected threat.
///
/// In a full implementation, this function might block offending traffic,
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
        // Create a packet that should be parsed as an MQTT alert.
        let packet1 = Packet::new("MQTT CONNECT alert/home".as_bytes().to_vec());
        // Create a generic packet with no threat.
        let packet2 = Packet::new("INFO system_ok".as_bytes().to_vec());
        let packets = [packet1, packet2];

        // Test that each packet can be parsed.
        for packet in packets.iter() {
            let res = parse_packet(packet);
            assert!(res.is_ok());
        }

        // Specifically, test that the first packet triggers threat detection.
        if let Ok(parsed) = parse_packet(&packets[0]) {
            let detection = analyze_packet(&parsed);
            assert_eq!(
                detection,
                DetectionResult::ThreatDetected("MQTT CONNECT alert".into())
            );
        }
    }
}
