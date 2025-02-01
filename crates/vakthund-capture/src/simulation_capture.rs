//! # Simulation Capture Module
//!
//! Implements a deterministic packet capture simulation.
//! Runs continuously until termination is requested.
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use vakthund_common::logger::log_info;
use vakthund_common::packet::Packet;

pub fn simulate_capture_loop<F>(terminate: &Arc<AtomicBool>, callback: &mut F)
where
    F: FnMut(Packet),
{
    let mut i = 0;
    while !terminate.load(std::sync::atomic::Ordering::SeqCst) {
        let content = match i % 3 {
            0 => format!("MQTT CONNECT alert/home_sim_{}", i),
            1 => format!("COAP GET sensor/alert_sim_{}", i),
            _ => format!("INFO system_ok_sim_{}", i),
        };
        log_info(&format!("Simulated capture of packet {}: {}", i, content));
        callback(Packet::new(content.into_bytes()));
        i += 1;
        thread::sleep(Duration::from_millis(50));
    }
}
