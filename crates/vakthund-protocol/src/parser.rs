//! # Packet Parser
//!
//! Parses a raw packet (from `vakthund_common::packet::Packet`) into a structured format.
//! The expected format is as follows:
//!
//! For MQTT packets (example):
//!     ID:3 MQTT CONNECT alert/home_sim_3
//!
//! For COAP packets (example):
//!     ID:4 COAP GET sensor/alert_sim_4
//!
//! Otherwise, a Generic variant is returned.

use std::str::FromStr;
use vakthund_common::errors::PacketError;
use vakthund_common::packet::Packet;

#[derive(Debug, PartialEq, Eq)]
pub enum Protocol {
    MQTT,
    COAP,
    Other(String),
}

impl FromStr for Protocol {
    type Err = PacketError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mqtt" => Ok(Protocol::MQTT),
            "coap" => Ok(Protocol::COAP),
            other => Ok(Protocol::Other(other.to_string())),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MqttCommand {
    Connect,
    Other(String),
}

impl FromStr for MqttCommand {
    type Err = PacketError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("connect") {
            Ok(MqttCommand::Connect)
        } else {
            Ok(MqttCommand::Other(s.to_string()))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CoapMethod {
    Get,
    Other(String),
}

impl FromStr for CoapMethod {
    type Err = PacketError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("get") {
            Ok(CoapMethod::Get)
        } else {
            Ok(CoapMethod::Other(s.to_string()))
        }
    }
}

#[derive(Debug)]
pub enum ParsedPacket {
    Mqtt {
        command: MqttCommand,
        topic: String,
    },
    Coap {
        method: CoapMethod,
        resource: String,
    },
    Generic {
        header: String,
        payload: String,
    },
}

/// Parses a packet into a `ParsedPacket`.
pub fn parse_packet(packet: &Packet) -> Result<ParsedPacket, PacketError> {
    let s = packet.as_str().ok_or(PacketError::InvalidUtf8)?;
    let mut parts = s.split_whitespace();

    // The packet must start with an ID token.
    let id_token = parts
        .next()
        .ok_or_else(|| PacketError::FormatError("Missing ID token".to_string()))?;
    if !id_token.starts_with("ID:") {
        return Err(PacketError::FormatError(
            "Expected ID token at start".to_string(),
        ));
    }

    // Next, expect the protocol token.
    let protocol_token = parts
        .next()
        .ok_or_else(|| PacketError::FormatError("Missing protocol token".to_string()))?;
    let protocol = Protocol::from_str(protocol_token)?;

    // Next, expect the command token.
    let command_token = parts
        .next()
        .ok_or_else(|| PacketError::FormatError("Missing command token".to_string()))?;

    match protocol {
        Protocol::MQTT => {
            // For MQTT, if command is CONNECT, then a topic is expected.
            let cmd = MqttCommand::from_str(command_token)?;
            if cmd == MqttCommand::Connect {
                let topic = parts.next().ok_or_else(|| {
                    PacketError::FormatError("Missing topic for MQTT CONNECT".to_string())
                })?;
                Ok(ParsedPacket::Mqtt {
                    command: MqttCommand::Connect,
                    topic: topic.to_string(),
                })
            } else {
                Err(PacketError::FormatError(
                    "Unsupported MQTT command".to_string(),
                ))
            }
        }
        Protocol::COAP => {
            // For COAP, we expect a method and a resource.
            let method = CoapMethod::from_str(command_token)?;
            let resource = parts.next().ok_or_else(|| {
                PacketError::FormatError("Missing resource for COAP packet".to_string())
            })?;
            Ok(ParsedPacket::Coap {
                method,
                resource: resource.to_string(),
            })
        }
        Protocol::Other(_) => {
            // For unknown protocols, return the entire content as a Generic packet.
            let header = format!("{} {}", protocol_token, command_token);
            let payload = parts.collect::<Vec<&str>>().join(" ");
            Ok(ParsedPacket::Generic { header, payload })
        }
    }
}
