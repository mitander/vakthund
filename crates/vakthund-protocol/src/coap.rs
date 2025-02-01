//! # CoAP Parser
//!
//! Contains functions to parse CoAP packet formats.

use crate::parser::{CoapMethod, ParsedPacket};
use vakthund_common::packet::Packet;

/// Attempts to parse a CoAP packet.
/// Expected format: "COAP <METHOD> <RESOURCE> ..."
pub fn parse_coap(packet: &Packet) -> Option<ParsedPacket> {
    let s = packet.as_str()?;
    let mut parts = s.splitn(3, ' ');
    let protocol = parts.next()?;
    if protocol != "COAP" {
        return None;
    }
    let method_str = parts.next()?;
    let resource = parts.next()?;
    let method = if method_str == "GET" {
        CoapMethod::GET
    } else {
        CoapMethod::Other(method_str)
    };
    Some(ParsedPacket::Coap { method, resource })
}
