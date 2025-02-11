//! Crate for parsing network protocols like MQTT, CoAP, and Modbus.

use std::fmt::Debug;

pub mod coap;
pub mod modbus;
pub mod mqtt;

pub use coap::{CoapPacket, CoapParseError, CoapParser};
pub use modbus::{ModbusPacket, ModbusParseError, ModbusParser};
pub use mqtt::{MqttPacket, MqttParseError, MqttParser};

/// A trait for a protocol-specific packet.
pub trait ProtocolPacket<'a> {
    /// Returns the rule ID for this packet.
    fn rule_id(&self) -> String;
    /// Returns the payload of the packet.
    fn payload(&self) -> &'a [u8];
}

impl<'a> ProtocolPacket<'a> for MqttPacket<'a> {
    fn rule_id(&self) -> String {
        self.rule_id()
    }
    fn payload(&self) -> &'a [u8] {
        self.payload()
    }
}

impl<'a> ProtocolPacket<'a> for CoapPacket<'a> {
    fn rule_id(&self) -> String {
        "Coap_GENERIC".to_string()
    }
    fn payload(&self) -> &'a [u8] {
        self.payload
    }
}

impl<'a> ProtocolPacket<'a> for ModbusPacket<'a> {
    fn rule_id(&self) -> String {
        "Modbus_GENERIC".to_string()
    }
    fn payload(&self) -> &'a [u8] {
        self.payload()
    }
}

#[derive(Debug, Clone, Copy)] // Add Debug and Copy
pub enum AnyParser {
    Mqtt(MqttParser),
    Coap(CoapParser),
    Modbus(ModbusParser),
}
