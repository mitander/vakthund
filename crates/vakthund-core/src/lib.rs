//! # Vakthund Core
//!
//! Integrates all Vakthund modules into a complete IDS pipeline.

pub mod pipeline;

pub use pipeline::run_vakthund;
