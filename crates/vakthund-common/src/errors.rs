//! # Custom Errors
//!
//! Defines custom error types for Vakthund modules.

#[derive(Debug)]
pub enum PacketError {
    InvalidUtf8,
    FormatError(String),
}

impl std::fmt::Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketError::InvalidUtf8 => write!(f, "Invalid UTF-8 sequence in packet data"),
            PacketError::FormatError(msg) => write!(f, "Packet format error: {}", msg),
        }
    }
}

impl std::error::Error for PacketError {}
