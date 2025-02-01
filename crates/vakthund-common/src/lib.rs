//! # Vakthund Common
//!
//! Contains shared types, custom errors, and utilities for all Vakthund modules.
//! Implements zero‑copy packet data using `Arc<[u8]>` and minimal logging utilities.

pub mod config;
pub mod errors;
pub mod logger;
pub mod packet;
pub mod sim_logging;
