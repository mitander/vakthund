//! Live Capture Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Implements live packet capture using the pcap crate. This module opens the specified
//! network interface in promiscuous mode (if requested) and reads packets in a loop.
//! Captured packets are converted to the common Packet type for further processing.

use pcap::Capture;

use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use vakthund_common::packet::Packet;

/// Captures live packets from the specified interface and calls `callback` for each packet.
/// - `interface`: the name of the network interface to capture packets from.
/// - `buffer_size`: the snapshot length for captured packets.
/// - `promiscuous`: whether to enable promiscuous mode.
/// - `terminate`: an atomic flag to indicate when to stop capturing.
/// - `callback`: a closure to process each captured packet.
pub fn live_capture_loop<F>(
    interface: &str,
    buffer_size: usize,
    promiscuous: bool,
    terminate: &Arc<AtomicBool>,
    callback: &mut F,
) where
    F: FnMut(Packet),
{
    // Open the device for live capture.
    let mut cap = Capture::from_device(interface)
        .expect("Device not found")
        .promisc(promiscuous)
        .snaplen(buffer_size as i32)
        .open()
        .expect("Failed to open device for capture");

    while !terminate.load(AtomicOrdering::SeqCst) {
        match cap.next_packet() {
            Ok(packet) => {
                // Convert the packet data (a &[u8]) to a Vec<u8> and then to our Packet type.
                let data = packet.data.to_vec();
                callback(Packet::new(data));
            }
            Err(e) => {
                // If no packet is available, sleep briefly.
                eprintln!("[ERROR] Pcap error: {}", e);
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
