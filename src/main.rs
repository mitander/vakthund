//! # Vakthund IDPS Application
//!
//! This binary is the entry point for the Vakthund Intrusion Detection and Prevention System (IDPS).
//! It initializes logging and then runs the pipeline defined in vakthund-core.

use vakthund_common::logger::log_info;
use vakthund_core::run_vakthund;

fn main() {
    log_info("Starting Vakthund IDPS application...");
    run_vakthund();
    log_info("Vakthund IDPS application terminated.");
}
