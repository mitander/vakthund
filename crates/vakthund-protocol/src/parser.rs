//! Packet Parser
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Parses a raw packet into a structured format. Expects the packet content to start with an ID token,
//! followed by a protocol token and command token. For MQTT, a topic is expected; for COAP, a resource is expected.
//! Unrecognized protocols yield a Generic packet.

use std::str::FromStr;
use vakthund_common::errors::PacketError;
use vakthund_common::packet::Packet;

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

pub fn parse_packet(packet: &Packet) -> Result<ParsedPacket, PacketError> {
    let s = packet.as_str().ok_or(PacketError::InvalidUtf8)?;
    let mut parts = s.split_whitespace();

    let id_token = parts
        .next()
        .ok_or_else(|| PacketError::FormatError("Missing ID token".to_string()))?;
    if !id_token.starts_with("ID:") {
        return Err(PacketError::FormatError(
            "Expected ID token at start".to_string(),
        ));
    }

    let protocol_token = parts
        .next()
        .ok_or_else(|| PacketError::FormatError("Missing protocol token".to_string()))?;
    let protocol = Protocol::from_str(protocol_token)?;

    let command_token = parts
        .next()
        .ok_or_else(|| PacketError::FormatError("Missing command token".to_string()))?;

    match protocol {
        Protocol::MQTT => {
            let cmd = MqttCommand::from_str(command_token)?;
            if let MqttCommand::Connect = cmd {
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
            let header = format!("{} {}", protocol_token, command_token);
            let payload = parts.collect::<Vec<&str>>().join(" ");
            Ok(ParsedPacket::Generic { header, payload })
        }
    }
}
