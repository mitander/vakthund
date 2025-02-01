//! # MQTT Parser
//!
//! Provides a specialized function to parse MQTT packets.
use crate::parser::{parse_packet, ParsedPacket};
use vakthund_common::packet::Packet;

pub fn parse_mqtt(packet: &Packet) -> Option<ParsedPacket> {
    let s = packet.as_str()?;
    if s.to_lowercase().contains("mqtt") {
        parse_packet(packet).ok()
    } else {
        None
    }
}
