//! Packet Parser
//!
//! Proprietary and confidential. All rights reserved.
//!
//! This module parses a raw packet (from `vakthund_common::packet::Packet`) into a structured format.
//! The expected format is as follows:
//!
//!   ID:<number> <protocol> <command> [argument]
//!
//! For example, an MQTT packet might be:
//!
//!   ID:12 MQTT CONNECT alert_topic
//!
//! For COAP packets, a resource is expected after the method token.
//!
//! Unrecognized protocols yield a Generic packet.
//!
//! This parser uses nom version 8 for efficient, zero‑copy parsing.

use nom::{
    bytes::complete::{tag, take_while1},
    character::complete::{digit1, space1},
    combinator::{map_res, opt},
    IResult,
    Parser, // Import the Parser trait so that we can call .parse(input)
};

use std::str::FromStr;

use vakthund_common::errors::PacketError;
use vakthund_common::packet::Packet;

/// Supported protocols.
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

/// Supported MQTT commands.
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

/// Supported COAP methods.
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

/// Parsed packet types.
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

/// Parses a Packet into a ParsedPacket using nom.
pub fn parse_packet(packet: &Packet) -> Result<ParsedPacket, PacketError> {
    let s = packet.as_str().ok_or(PacketError::InvalidUtf8)?;
    match parse_nom(s) {
        Ok(("", result)) => Ok(result),
        Ok((remaining, _)) => Err(PacketError::FormatError(format!(
            "Unparsed data remaining: {}",
            remaining
        ))),
        Err(_) => Err(PacketError::FormatError("Failed to parse packet".into())),
    }
}

/// Nom-based parser implementation.
fn parse_nom(input: &str) -> IResult<&str, ParsedPacket> {
    // Parse the "ID:" token.
    let (input, _) = tag("ID:").parse(input)?;
    // Parse the packet number (digits).
    let (input, _id) = map_res(digit1, |s: &str| s.parse::<usize>()).parse(input)?;
    let (input, _) = space1.parse(input)?;

    // Parse the protocol token (alphanumeric).
    let (input, protocol_str) = take_while1(|c: char| c.is_alphanumeric()).parse(input)?;
    let protocol = Protocol::from_str(protocol_str).map_err(|_| {
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
    })?;
    let (input, _) = space1.parse(input)?;

    // Parse the command token (alphanumeric).
    let (input, command_str) = take_while1(|c: char| c.is_alphanumeric()).parse(input)?;
    let (input, _) = opt(space1).parse(input)?;

    match protocol {
        Protocol::MQTT => {
            let cmd = MqttCommand::from_str(command_str).map_err(|_| {
                nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
            })?;
            if let MqttCommand::Connect = cmd {
                // Parse the topic.
                let (input, topic) = take_while1(|c: char| !c.is_whitespace()).parse(input)?;
                Ok((
                    input,
                    ParsedPacket::Mqtt {
                        command: MqttCommand::Connect,
                        topic: topic.to_string(),
                    },
                ))
            } else {
                Err(nom::Err::Failure(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Tag,
                )))
            }
        }
        Protocol::COAP => {
            let method = CoapMethod::from_str(command_str).map_err(|_| {
                nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
            })?;
            let (input, resource) = take_while1(|c: char| !c.is_whitespace()).parse(input)?;
            Ok((
                input,
                ParsedPacket::Coap {
                    method,
                    resource: resource.to_string(),
                },
            ))
        }
        Protocol::Other(_) => {
            let header = format!("{} {}", protocol_str, command_str);
            let payload = input.trim();
            Ok((
                "",
                ParsedPacket::Generic {
                    header,
                    payload: payload.to_string(),
                },
            ))
        }
    }
}
