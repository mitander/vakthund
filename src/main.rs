//! # Vakthund IDPS Application
//!
//! Entry point for the Vakthund Intrusion Detection and Prevention System.
//! It initializes logging and runs the core pipeline.
use vakthund_common::logger::log_info;
use vakthund_core::pipeline;

fn main() {
    log_info("Starting Vakthund IDPS application...");
    pipeline::run_vakthund();
    log_info("Vakthund IDPS application terminated.");
}
