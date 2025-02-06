//! # Vakthund Prevention Modules
//!
//! Crate for implementing prevention mechanisms. This crate provides
//! a no-op implementation when eBPF is not supported.

pub mod firewall;
// TODO: pub mod quarantine;
// TODO: pub mod rate_limit;

pub use firewall::Firewall;
