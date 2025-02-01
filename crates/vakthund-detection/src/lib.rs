//! # Vakthund Detection
//!
//! Provides an engine to analyze parsed packets for potential threats.

pub mod analyzer;

pub use analyzer::analyze_packet;
