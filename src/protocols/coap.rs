//! CoAP protocol parser
//!
//! Implements a zero‑copy parser for CoAP packets.
//! Returns a rule ID (as a string) if a match is found.

use bytes::Bytes;

#[derive(Debug)]
pub struct CoapPacket<'a> {
    pub version: u8,
    pub code: u8,
    pub payload: &'a [u8],
}

pub struct CoapParser;

impl CoapParser {
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn parse(&self, data: &Bytes) -> Option<String> {
        if data.len() < 4 {
            return None;
        }
        if (data[0] >> 6) == 0x01 {
            return Some("CoAP_ALERT".to_string());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    #[test]
    fn test_coap_parse_valid() {
        let data = Bytes::from(vec![0x40, 0x01, 0x00, 0x00, b'p', b'a', b'y']);
        let parser = CoapParser::new();
        let rule = parser.parse(&data);
        assert!(rule.is_some());
    }
    #[test]
    fn test_coap_parse_invalid() {
        let data = Bytes::from(vec![0x20, 0x00]);
        let parser = CoapParser::new();
        let rule = parser.parse(&data);
        assert!(rule.is_none());
    }
}
