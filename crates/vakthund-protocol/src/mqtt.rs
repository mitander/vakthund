//! # MQTT Parser
//!
//! Contains functions to parse MQTT packet formats.

use crate::parser::{MqttCommand, ParsedPacket};
use vakthund_common::packet::Packet;

/// Attempts to parse an MQTT packet.
/// Expected format: "MQTT <COMMAND> <TOPIC> ..."
pub fn parse_mqtt(packet: &Packet) -> Option<ParsedPacket> {
    let s = packet.as_str()?;
    let mut parts = s.splitn(3, ' ');
    let protocol = parts.next()?;
    if protocol != "MQTT" {
        return None;
    }
    let command_str = parts.next()?;
    let topic = parts.next()?;
    let command = if command_str == "CONNECT" {
        MqttCommand::Connect
    } else {
        MqttCommand::Other(command_str)
    };
    Some(ParsedPacket::Mqtt { command, topic })
}
