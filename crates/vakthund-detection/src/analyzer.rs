//! Analyzer Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Implements threat analysis logic for parsed packets. Uses protocol-specific heuristics
//! to determine if a packet represents a threat.

use vakthund_protocol::parser::{CoapMethod, MqttCommand, ParsedPacket};

#[derive(Debug, PartialEq, Eq)]
pub enum DetectionResult {
    ThreatDetected(String),
    NoThreat,
}

pub fn analyze_packet(packet: &ParsedPacket) -> DetectionResult {
    match packet {
        ParsedPacket::Mqtt { command, topic } => {
            if let MqttCommand::Connect = command {
                if topic.contains("alert") {
                    return DetectionResult::ThreatDetected("MQTT CONNECT alert".into());
                }
            }
            DetectionResult::NoThreat
        }
        ParsedPacket::Coap { method, resource } => {
            if let CoapMethod::Get = method {
                if resource.contains("alert") {
                    return DetectionResult::ThreatDetected("COAP GET alert".into());
                }
            }
            DetectionResult::NoThreat
        }
        ParsedPacket::Generic { header, .. } => {
            if header.contains("alert") {
                DetectionResult::ThreatDetected(header.clone())
            } else {
                DetectionResult::NoThreat
            }
        }
    }
}
