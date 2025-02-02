//! Logger Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Provides minimal logging functions.

pub fn log_info(message: &str) {
    println!("[INFO] {}", message);
}

pub fn log_warn(message: &str) {
    println!("[WARN] {}", message);
}

pub fn log_error(message: &str) {
    eprintln!("[ERROR] {}", message);
}
