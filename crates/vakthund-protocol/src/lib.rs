//! # Vakthund Protocol
//!
//! Provides functions to parse raw packet data into structured messages.
//! Supports MQTT, CoAP, and generic parsing.

pub mod coap;
pub mod generic;
pub mod mqtt;
pub mod parser;

pub use parser::parse_packet;
