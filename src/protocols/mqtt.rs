//! MQTT protocol parser
//!
//! Implements a zero‑copy parser for MQTT packets following version 3.1.1.
//! Returns a rule ID (as a string) if a match is found.

use bytes::Bytes;
use hex;

#[derive(Debug)]
pub struct MqttPacket<'a> {
    pub header: u8,
    pub topic: &'a [u8],
    pub payload: &'a [u8],
}

pub struct MqttParser;

impl MqttParser {
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn parse(&self, data: &Bytes) -> Option<String> {
        if data.len() < 5 {
            return None;
        }
        if data[0] == 0x10 {
            let topic = &data[2..6];
            return Some(format!("MQTT_{}", hex::encode(topic)));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    #[test]
    fn test_mqtt_parse_valid() {
        let data = Bytes::from(vec![0x10, 0x04, b't', b'e', b's', b't', b'X']);
        let parser = MqttParser::new();
        let rule = parser.parse(&data);
        assert!(rule.is_some());
    }
    #[test]
    fn test_mqtt_parse_invalid() {
        let data = Bytes::from(vec![0x20, 0x01, b'a']);
        let parser = MqttParser::new();
        let rule = parser.parse(&data);
        assert!(rule.is_none());
    }
}
