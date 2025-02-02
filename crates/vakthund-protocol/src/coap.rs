//! COAP Parser
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Provides a wrapper for parsing COAP packets using the generic parser.

use crate::parser::{parse_packet, ParsedPacket};
use vakthund_common::packet::Packet;

pub fn parse_coap(packet: &Packet) -> Option<ParsedPacket> {
    let s = packet.as_str()?;
    if s.to_lowercase().contains("coap") {
        parse_packet(packet).ok()
    } else {
        None
    }
}
