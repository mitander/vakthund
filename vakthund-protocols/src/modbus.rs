//! ## vakthund-protocols::modbus
//! Implements a zero-copy Modbus protocol parser.

use bytes::Bytes;
use thiserror::Error; // Add this line

/// Modbus-specific errors.
#[derive(Clone, Debug, PartialEq, Error)] // Add Error derive
pub enum ModbusParseError {
    /// The packet is too short to contain a valid header.
    #[error("Insufficient data to parse Modbus packet")]
    InsufficientData,
    /// The function code is invalid or not supported.
    #[error("Invalid Modbus function code")]
    InvalidFunctionCode,
    /// The packet is malformed or contains invalid data.
    #[error("Malformed Modbus packet")]
    MalformedPacket,
}

/// Represents a Modbus packet with zero-copy slices into the original data.
#[derive(Debug, Copy, Clone)]
pub struct ModbusPacket<'a> {
    /// The transaction ID (2 bytes).
    pub transaction_id: u16,
    /// The protocol ID (2 bytes, usually 0).
    pub protocol_id: u16,
    /// The length (2 bytes).
    pub length: u16,
    /// The unit ID (1 byte).
    pub unit_id: u8,
    /// The function code (1 byte).
    pub function_code: u8,
    /// The data payload.
    pub data: &'a [u8],
}

impl<'a> ModbusPacket<'a> {
    /// Returns the payload of the packet
    pub fn payload(&self) -> &'a [u8] {
        self.data
    }
}

/// A simple Modbus parser.
#[derive(Default, Debug, Copy, Clone)]
pub struct ModbusParser;

impl ModbusParser {
    /// Creates a new Modbus parser.
    pub fn new() -> Self {
        Self
    }

    /// Parses a Modbus packet from a Bytes slice.
    pub fn parse<'a>(&self, data: &'a Bytes) -> Result<ModbusPacket<'a>, ModbusParseError> {
        if data.len() < 8 {
            // Minimal Modbus header size is 8 bytes.
            return Err(ModbusParseError::InsufficientData);
        }

        let transaction_id = u16::from_be_bytes([data[0], data[1]]);
        let protocol_id = u16::from_be_bytes([data[2], data[3]]);
        let length = u16::from_be_bytes([data[4], data[5]]);
        let unit_id = data[6];
        let function_code = data[7];

        // Basic sanity checks.
        if protocol_id != 0 {
            // We expect the protocol ID to be 0.
            return Err(ModbusParseError::MalformedPacket);
        }
        if data.len() < 6 + length as usize {
            // Make sure length is sufficient to read all data.
            return Err(ModbusParseError::InsufficientData);
        }

        let data_start = 8;
        let data_end = 6 + length as usize;
        let data_slice = &data[data_start..data_end];

        Ok(ModbusPacket {
            transaction_id,
            protocol_id,
            length,
            unit_id,
            function_code,
            data: data_slice,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_valid_modbus_packet() {
        // Example Modbus packet (Read Holding Registers).
        let packet_bytes = Bytes::from(vec![
            0x00, 0x01, // Transaction ID
            0x00, 0x00, // Protocol ID
            0x00, 0x06, // Length
            0x01, // Unit ID
            0x03, // Function Code (Read Holding Registers)
            0x00, 0x00, // Start Address
            0x00, 0x01, // Quantity of Registers
        ]);

        let parser = ModbusParser::new();
        let result = parser.parse(&packet_bytes);
        assert!(result.is_ok());

        let packet = result.unwrap();
        assert_eq!(packet.transaction_id, 1);
        assert_eq!(packet.protocol_id, 0);
        assert_eq!(packet.length, 6);
        assert_eq!(packet.unit_id, 1);
        assert_eq!(packet.function_code, 3);
        assert_eq!(packet.payload(), &[0x00, 0x00, 0x00, 0x01]);
    }

    #[test]
    fn test_insufficient_data() {
        let packet_bytes = Bytes::from(vec![0x00, 0x01, 0x00, 0x00, 0x00]);
        let parser = ModbusParser::new();
        let result = parser.parse(&packet_bytes);
        assert!(matches!(result, Err(ModbusParseError::InsufficientData)));
    }

    #[test]
    fn test_invalid_protocol_id() {
        let packet_bytes = Bytes::from(vec![
            0x00, 0x01, // Transaction ID
            0x00, 0x01, // Protocol ID (Invalid)
            0x00, 0x06, // Length
            0x01, // Unit ID
            0x03, // Function Code (Read Holding Registers)
            0x00, 0x00, // Start Address
            0x00, 0x01, // Quantity of Registers
        ]);
        let parser = ModbusParser::new();
        let result = parser.parse(&packet_bytes);
        assert!(matches!(result, Err(ModbusParseError::MalformedPacket)));
    }

    #[test]
    fn test_invalid_data_length() {
        let packet_bytes = Bytes::from(vec![
            0x00, 0x01, // Transaction ID
            0x00, 0x00, // Protocol ID
            0x00, 0x07, // Length (Too Large)
            0x01, // Unit ID
            0x03, // Function Code (Read Holding Registers)
            0x00, 0x00, // Start Address
            0x00, 0x01, // Quantity of Registers
        ]);
        let parser = ModbusParser::new();
        let result = parser.parse(&packet_bytes);
        assert!(matches!(result, Err(ModbusParseError::InsufficientData)));
    }
}
