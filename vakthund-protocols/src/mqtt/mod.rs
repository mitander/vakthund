//! ## vakthund-protocols::mqtt
//!
//! A combined MQTT protocol parser that preserves the simplicity of a
//! fixed‑offset parser but adds features like error handling and proper
//! variable‑length decoding. It assumes that when the first byte (header)
//! equals 0x10, the next 4 bytes of the variable header represent a topic.
//! For other packet types, the entire variable header is treated as the payload.
//!
//! ### Expectations:
//! - <100ns per byte parsing throughput
//! - Protocol validation via const assertions
//! - Fuzz-tested against RFC edge cases
//!
//! ### Future:
//! - QUIC/UDP-based protocol support
//! - Autogenerated parsers from formal specifications
use bytes::Bytes;
use hex;

/// Errors that can occur while parsing an MQTT packet.
#[derive(Clone, Debug, PartialEq)]
pub enum MqttParseError {
    InsufficientData,
    InvalidHeader,
    RemainingLengthMalformed,
    PacketIncomplete,
}

/// Represents an MQTT packet as zero‑copy slices into the original data.
#[derive(Debug, Copy, Clone)]
pub struct MqttPacket<'a> {
    pub header: u8,
    /// For header 0x10, this is the topic (4 bytes); for other packets this is empty.
    pub topic: &'a [u8],
    /// The remaining bytes of the packet (variable header and payload).
    pub payload: &'a [u8],
}

impl<'a> MqttPacket<'a> {
    /// Generates a rule ID string based on the packet contents.
    /// For header 0x10, it produces "MQTT_{hex‑encoded topic}",
    /// otherwise it returns "MQTT_GENERIC".
    pub fn rule_id(&self) -> String {
        if self.header == 0x10 && self.topic.len() == 4 {
            format!("MQTT_{}", hex::encode(self.topic))
        } else {
            "MQTT_GENERIC".to_string()
        }
    }
}

/// A simple MQTT parser that works on zero‑copy data.
pub struct MqttParser;

impl MqttParser {
    pub fn new() -> Self {
        Self
    }

    /// Parses an MQTT packet from a `Bytes` slice.
    ///
    /// The parser expects:
    /// 1. A fixed header (1 byte).
    /// 2. A variable‑length encoded “remaining length” field.
    /// 3. For header 0x10, a 4‑byte topic field; otherwise, the whole
    ///    variable header is treated as payload.
    ///
    /// Returns a structured `MqttPacket` on success.
    pub fn parse<'a>(&self, data: &'a Bytes) -> Result<MqttPacket<'a>, MqttParseError> {
        if data.len() < 2 {
            return Err(MqttParseError::InsufficientData);
        }
        let header = data[0];

        // Decode the remaining length field (which can be 1-4 bytes).
        let (remaining_length, length_field_size) = Self::decode_remaining_length(&data[1..])?;
        let fixed_header_length = 1 + length_field_size;

        // Check that the total packet is present.
        if data.len() < fixed_header_length + (remaining_length as usize) {
            return Err(MqttParseError::PacketIncomplete);
        }

        // For header 0x10, assume the next 4 bytes represent the topic.
        if header == 0x10 {
            if remaining_length < 4 {
                return Err(MqttParseError::InsufficientData);
            }
            let topic = &data[fixed_header_length..fixed_header_length + 4];
            let payload =
                &data[fixed_header_length + 4..fixed_header_length + (remaining_length as usize)];
            Ok(MqttPacket {
                header,
                topic,
                payload,
            })
        } else {
            // For other packet types, we do not extract a topic.
            let payload =
                &data[fixed_header_length..fixed_header_length + (remaining_length as usize)];
            Ok(MqttPacket {
                header,
                topic: &[],
                payload,
            })
        }
    }

    /// Decodes MQTT’s variable‑length “remaining length” field.
    ///
    /// Returns a tuple of (decoded_value, number_of_bytes_used).
    fn decode_remaining_length(input: &[u8]) -> Result<(u32, usize), MqttParseError> {
        let mut multiplier: u32 = 1;
        let mut value: u32 = 0;
        let mut i = 0;
        for byte in input.iter() {
            let byte_val = *byte;
            value += u32::from(byte_val & 0x7F) * multiplier;
            i += 1;
            // Prevent overflow (MQTT spec limits the length field to 4 bytes)
            if multiplier > 128 * 128 * 128 {
                return Err(MqttParseError::RemainingLengthMalformed);
            }
            if (byte_val & 0x80) == 0 {
                return Ok((value, i));
            }
            multiplier *= 128;
        }
        Err(MqttParseError::RemainingLengthMalformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_valid_connect_packet() {
        // Build a packet with:
        // - header 0x10,
        // - remaining length = 7 (4 bytes for topic + 3 bytes for payload),
        // - topic "test" (4 bytes),
        // - payload "abc".
        // The remaining length is encoded in one byte (0x07).
        let mut packet = vec![0x10, 0x07];
        packet.extend_from_slice(b"test");
        packet.extend_from_slice(b"abc");
        let bytes = Bytes::from(packet);
        let parser = MqttParser::new();
        let mqtt_packet = parser.parse(&bytes).unwrap();
        assert_eq!(mqtt_packet.header, 0x10);
        assert_eq!(mqtt_packet.topic, b"test");
        assert_eq!(mqtt_packet.payload, b"abc");
        assert_eq!(mqtt_packet.rule_id(), "MQTT_74657374");
    }

    #[test]
    fn test_valid_generic_packet() {
        // Build a packet with:
        // - header 0x20,
        // - remaining length = 3,
        // - payload "xyz".
        let mut packet = vec![0x20, 0x03];
        packet.extend_from_slice(b"xyz");
        let bytes = Bytes::from(packet);
        let parser = MqttParser::new();
        let mqtt_packet = parser.parse(&bytes).unwrap();
        assert_eq!(mqtt_packet.header, 0x20);
        assert_eq!(mqtt_packet.topic.len(), 0);
        assert_eq!(mqtt_packet.payload, b"xyz");
        assert_eq!(mqtt_packet.rule_id(), "MQTT_GENERIC");
    }

    #[test]
    fn test_incomplete_packet() {
        // A packet that claims to have more bytes than are provided.
        let packet = vec![0x10, 0x07, b'a'];
        let bytes = Bytes::from(packet);
        let parser = MqttParser::new();
        assert!(matches!(
            parser.parse(&bytes),
            Err(MqttParseError::PacketIncomplete)
        ));
    }

    #[test]
    fn test_malformed_remaining_length() {
        // A packet with a remaining length field that does not terminate.
        let packet = vec![0x10, 0xFF, 0xFF, 0xFF, 0xFF];
        let bytes = Bytes::from(packet);
        let parser = MqttParser::new();
        assert!(matches!(
            parser.parse(&bytes),
            Err(MqttParseError::RemainingLengthMalformed)
        ));
    }
}
