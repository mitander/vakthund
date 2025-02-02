//! Custom Errors
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Defines custom error types for Vakthund modules using `thiserror`.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("Invalid UTF-8 sequence in packet data")]
    InvalidUtf8,
    #[error("Packet format error: {0}")]
    FormatError(String),
}
