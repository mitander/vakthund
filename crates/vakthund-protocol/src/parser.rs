//! # Packet Parser
//!
//! Provides the public interface for parsing packets. It attempts MQTT parsing first,
//! then CoAP, then generic parsing. The parsed packet is zero‑copy (borrowing from the packet).

use crate::coap;
use crate::generic;
use crate::mqtt;
use vakthund_common::errors::PacketError;
use vakthund_common::packet::Packet;

/// Enum representing the MQTT command.
#[derive(Debug, PartialEq, Eq)]
pub enum MqttCommand<'a> {
    Connect,
    Other(&'a str),
}

/// Enum representing the CoAP method.
#[derive(Debug, PartialEq, Eq)]
pub enum CoapMethod<'a> {
    GET,
    Other(&'a str),
}

/// The parsed packet type.
#[derive(Debug, PartialEq, Eq)]
pub enum ParsedPacket<'a> {
    Mqtt {
        command: MqttCommand<'a>,
        topic: &'a str,
    },
    Coap {
        method: CoapMethod<'a>,
        resource: &'a str,
    },
    Generic {
        header: &'a str,
        payload: &'a str,
    },
}

/// Attempts to parse a packet into a [`ParsedPacket`]. Returns a custom error if parsing fails.
pub fn parse_packet(packet: &Packet) -> Result<ParsedPacket, PacketError> {
    if let Some(mqtt_packet) = mqtt::parse_mqtt(packet) {
        return Ok(mqtt_packet);
    }
    if let Some(coap_packet) = coap::parse_coap(packet) {
        return Ok(coap_packet);
    }
    if let Some(generic_packet) = generic::parse_generic(packet) {
        return Ok(generic_packet);
    }
    Err(PacketError::FormatError("Packet format is invalid".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vakthund_common::packet::Packet;

    #[test]
    fn test_parse_packet_mqtt_positive() {
        let packet = Packet::new("MQTT CONNECT home/livingroom".as_bytes().to_vec());
        if let Ok(ParsedPacket::Mqtt { command, topic }) = parse_packet(&packet) {
            assert_eq!(command, MqttCommand::Connect);
            assert_eq!(topic, "home/livingroom");
        } else {
            panic!("MQTT packet should parse correctly");
        }
    }

    #[test]
    fn test_parse_packet_coap_positive() {
        let packet = Packet::new("COAP GET sensor/kitchen".as_bytes().to_vec());
        if let Ok(ParsedPacket::Coap { method, resource }) = parse_packet(&packet) {
            assert_eq!(method, CoapMethod::GET);
            assert_eq!(resource, "sensor/kitchen");
        } else {
            panic!("CoAP packet should parse correctly");
        }
    }

    #[test]
    fn test_parse_packet_generic_positive() {
        let packet = Packet::new("INFO SystemRunning".as_bytes().to_vec());
        if let Ok(ParsedPacket::Generic { header, payload }) = parse_packet(&packet) {
            assert_eq!(header, "INFO");
            assert_eq!(payload, "SystemRunning");
        } else {
            panic!("Generic packet should parse correctly");
        }
    }

    #[test]
    fn test_parse_packet_negative() {
        let packet = Packet::new("incomplete".as_bytes().to_vec());
        assert!(parse_packet(&packet).is_err());
    }
}
