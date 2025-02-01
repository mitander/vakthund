//! # Packet Analyzer
//!
//! Analyzes parsed packets for suspicious content using enums instead of magic strings.
//! In the IDPS system, when a threat is detected, it will be passed to the prevention layer.

use vakthund_protocol::parser::{CoapMethod, MqttCommand, ParsedPacket};

#[derive(Debug, PartialEq, Eq)]
pub enum DetectionResult {
    ThreatDetected(String),
    NoThreat,
}

pub fn analyze_packet(packet: &ParsedPacket) -> DetectionResult {
    match packet {
        ParsedPacket::Mqtt { command, topic } => {
            // For an IDPS, we define a threat as an MQTT CONNECT with a topic starting with "alert"
            if *command == MqttCommand::Connect && topic.starts_with("alert") {
                DetectionResult::ThreatDetected("MQTT CONNECT alert".into())
            } else {
                DetectionResult::NoThreat
            }
        }
        ParsedPacket::Coap { method, resource } => {
            // For CoAP, a GET request with a resource containing "alert" is considered a threat.
            if *method == CoapMethod::Get && resource.contains("alert") {
                DetectionResult::ThreatDetected("COAP GET alert".into())
            } else {
                DetectionResult::NoThreat
            }
        }
        ParsedPacket::Generic { header, .. } => {
            // For generic packets, any header containing "alert" is flagged.
            if header.contains("alert") {
                DetectionResult::ThreatDetected(header.to_string())
            } else {
                DetectionResult::NoThreat
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vakthund_protocol::parser::{CoapMethod, MqttCommand, ParsedPacket};

    #[test]
    fn test_detection_mqtt_positive() {
        let packet = ParsedPacket::Mqtt {
            command: MqttCommand::Connect,
            topic: "alert/home",
        };
        let result = analyze_packet(&packet);
        assert_eq!(
            result,
            DetectionResult::ThreatDetected("MQTT CONNECT alert".into())
        );
    }

    #[test]
    fn test_detection_mqtt_negative() {
        let packet = ParsedPacket::Mqtt {
            command: MqttCommand::Other("PUBLISH"),
            topic: "home",
        };
        let result = analyze_packet(&packet);
        assert_eq!(result, DetectionResult::NoThreat);
    }

    #[test]
    fn test_detection_coap_positive() {
        let packet = ParsedPacket::Coap {
            method: CoapMethod::GET,
            resource: "sensor/alert",
        };
        let result = analyze_packet(&packet);
        assert_eq!(
            result,
            DetectionResult::ThreatDetected("COAP GET alert".into())
        );
    }

    #[test]
    fn test_detection_coap_negative() {
        let packet = ParsedPacket::Coap {
            method: CoapMethod::Other("POST"),
            resource: "sensor/data",
        };
        let result = analyze_packet(&packet);
        assert_eq!(result, DetectionResult::NoThreat);
    }

    #[test]
    fn test_detection_generic_negative() {
        let packet = ParsedPacket::Generic {
            header: "INFO",
            payload: "system_ok",
        };
        let result = analyze_packet(&packet);
        assert_eq!(result, DetectionResult::NoThreat);
    }
}
