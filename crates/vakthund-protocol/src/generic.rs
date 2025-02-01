//! # Generic Parser
//!
//! Parses packets using a generic format by simply treating the entire packet content as a string.
//! This function returns a Generic variant of ParsedPacket, with header and payload both as owned Strings.

use crate::parser::ParsedPacket;
use vakthund_common::packet::Packet;

pub fn parse_generic(packet: &Packet) -> Option<ParsedPacket> {
    let s = packet.as_str()?;
    Some(ParsedPacket::Generic {
        header: s.to_string(),
        payload: "".to_string(),
    })
}
