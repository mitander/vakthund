//! # Vakthund Capture
//!
//! Provides a unified packet capture interface.
//! Depending on the configured mode (Live or Simulation), it either captures real packets using libpcap
//! or uses a deterministic simulation. It runs continuously until Ctrl‑C is pressed.

pub mod pcap_capture;
pub mod simulation_capture;

use crate::pcap_capture::capture_packets_loop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vakthund_common::config::CaptureMode;
use vakthund_common::packet::Packet;

/// Starts packet capture and calls the provided callback for each packet received.
/// Runs continuously until Ctrl‑C is pressed.
pub fn start_capture<F>(
    mode: &CaptureMode,
    interface: &str,
    buffer_size: usize,
    promiscuous: bool,
    seed: Option<u64>,
    replay_target: Option<String>,
    mut callback: F,
) where
    F: FnMut(Packet),
{
    // Set up a termination flag and a Ctrl-C handler.
    let terminate_flag = Arc::new(AtomicBool::new(false));
    {
        let flag = terminate_flag.clone();
        ctrlc::set_handler(move || {
            flag.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }

    match mode {
        CaptureMode::Simulation => simulation_capture::simulate_capture_loop(
            &terminate_flag,
            seed,
            replay_target,
            &mut callback,
        ),
        CaptureMode::Live => capture_packets_loop(
            &terminate_flag,
            interface,
            buffer_size,
            promiscuous,
            &mut callback,
        ),
    }
}
