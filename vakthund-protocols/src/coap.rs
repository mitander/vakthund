use bytes::Bytes;
use thiserror::Error;

/// CoAP-specific errors.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum CoapParseError {
    /// The packet is too short to contain a valid header.
    #[error("Insufficient data to parse CoAP packet")]
    InsufficientData,
    /// The version field in the header is not supported.
    #[error("Invalid CoAP version")]
    InvalidVersion,
    /// The option number is invalid.
    #[error("Invalid CoAP option number")]
    InvalidOptionNumber,
    /// The packet is malformed or contains invalid data.
    #[error("Malformed CoAP packet")]
    MalformedPacket,
}

/// Represents a CoAP packet with zero-copy slices into the original data.
#[derive(Debug, Copy, Clone)]
pub struct CoapPacket<'a> {
    /// The CoAP version (first 2 bits of the header).
    pub version: u8,
    /// The message type (next 2 bits of the header).
    pub message_type: u8,
    /// The token length (last 4 bits of the header).
    pub token_length: u8,
    /// The code (1 byte).
    pub code: u8,
    /// The message ID (2 bytes).
    pub message_id: u16,
    /// The options (variable-length bytes after token).
    pub options: &'a [u8],
    /// The payload (after 0xFF marker).
    pub payload: &'a [u8],
}

impl<'a> CoapPacket<'a> {
    /// Returns the payload of the packet.
    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }
}

/// A simple CoAP parser.
#[derive(Default, Debug, Copy, Clone)]
pub struct CoapParser;

impl CoapParser {
    /// Creates a new CoAP parser.
    pub fn new() -> Self {
        Self
    }

    /// Parses a CoAP packet from a Bytes slice.
    pub fn parse<'a>(&self, data: &'a Bytes) -> Result<CoapPacket<'a>, CoapParseError> {
        // Minimum CoAP header is 4 bytes: [VER+T+TKL, CODE, MSG_ID(2)]
        if data.len() < 4 {
            return Err(CoapParseError::InsufficientData);
        }

        // Parse first header byte
        let header = data[0];
        let version = (header >> 6) & 0x03; // First 2 bits
        let message_type = (header >> 4) & 0x03; // Next 2 bits
        let token_length = header & 0x0F; // Last 4 bits (TKL)

        // Validate supported version (CoAP v1)
        if version != 1 {
            return Err(CoapParseError::InvalidVersion);
        }

        // Parse remaining header fields
        let code = data[1];
        let message_id = u16::from_be_bytes([data[2], data[3]]);

        // Skip over token bytes if present
        let mut current_offset = 4;
        if current_offset + token_length as usize > data.len() {
            return Err(CoapParseError::InsufficientData);
        }
        current_offset += token_length as usize;

        // Find payload marker (0xFF) if exists
        let payload_marker = data[current_offset..].iter().position(|&x| x == 0xFF);

        let (options, payload) = match payload_marker {
            Some(pos) => {
                // Split at payload marker
                let marker_pos = current_offset + pos;
                (&data[current_offset..marker_pos], &data[marker_pos + 1..])
            }
            None => {
                // No payload - everything is options
                (&data[current_offset..], &[] as &[u8])
            }
        };

        Ok(CoapPacket {
            version,
            message_type,
            token_length,
            code,
            message_id,
            options,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_valid_coap_packet() {
        // Valid CoAP packet with:
        // Header: 0x40 (V=1, T=0, TKL=0)
        // Code: 0x02 (2.05 Content)
        // Message ID: 0x1234
        // Payload marker + "Hello"
        let packet_bytes = Bytes::from(vec![
            0x40, 0x02, 0x12, 0x34, 0xFF, 0x48, 0x65, 0x6c, 0x6c, 0x6f,
        ]);

        let parser = CoapParser::new();
        let result = parser.parse(&packet_bytes);
        assert!(result.is_ok());

        let packet = result.unwrap();
        assert_eq!(packet.version, 1);
        assert_eq!(packet.message_type, 0);
        assert_eq!(packet.token_length, 0);
        assert_eq!(packet.code, 0x02);
        assert_eq!(packet.message_id, 0x1234);
        assert_eq!(packet.payload(), b"Hello");
        assert!(packet.options.is_empty());
    }

    #[test]
    fn test_valid_coap_packet_without_payload() {
        // Valid CoAP packet without payload
        let packet_bytes = Bytes::from(vec![0x40, 0x02, 0x12, 0x34]);

        let parser = CoapParser::new();
        let packet = parser.parse(&packet_bytes).unwrap();
        assert_eq!(packet.version, 1);
        assert_eq!(packet.message_type, 0);
        assert_eq!(packet.token_length, 0);
        assert_eq!(packet.payload().len(), 0);
        assert!(packet.options.is_empty());
    }
}
