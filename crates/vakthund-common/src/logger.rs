//! # Logger Module
//!
//! Provides minimal logging utilities with a simple timestamp.

use std::time::{SystemTime, UNIX_EPOCH};

fn current_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn log_info(message: &str) {
    println!("[INFO {}] {}", current_timestamp(), message);
}

pub fn log_warn(message: &str) {
    println!("[WARN {}] {}", current_timestamp(), message);
}

pub fn log_error(message: &str) {
    println!("[ERROR {}] {}", current_timestamp(), message);
}
