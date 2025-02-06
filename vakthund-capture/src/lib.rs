//! vakthund‑capture
//!
//! Provides a unified capture interface for Vakthund.
//! Currently, only live capture (using pcap) is implemented.

pub mod live_capture;
pub mod packet;

pub use live_capture::live_capture_loop; // Re-export for easier use
pub use packet::Packet; // Re-export Packet type
