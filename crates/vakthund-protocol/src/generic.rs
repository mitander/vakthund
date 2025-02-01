//! # Generic Parser
//!
//! Parses packets using a generic format: "<HEADER> <PAYLOAD>".

use crate::parser::ParsedPacket;
use vakthund_common::packet::Packet;

pub fn parse_generic(packet: &Packet) -> Option<ParsedPacket> {
    let s = packet.as_str()?;
    let mut parts = s.splitn(2, ' ');
    let header = parts.next()?;
    let payload = parts.next()?;
    Some(ParsedPacket::Generic { header, payload })
}
