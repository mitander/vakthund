//! vakthund‑capture
//!
//! Provides a unified capture interface for Vakthund.
//! Currently, only live capture (using pcap) is implemented.

pub mod capture;
pub mod packet;

pub use capture::run;
pub use packet::Packet;
