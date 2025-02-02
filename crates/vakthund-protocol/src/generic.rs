//! Generic Parser
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Serves as a fallback parser that treats the entire packet content as a generic message.

use crate::parser::ParsedPacket;
use vakthund_common::packet::Packet;

pub fn parse_generic(packet: &Packet) -> Option<ParsedPacket> {
    let s = packet.as_str()?;
    Some(ParsedPacket::Generic {
        header: s.to_string(),
        payload: "".to_string(),
    })
}
