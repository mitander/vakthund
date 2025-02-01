//! # pcap_capture Module
//!
//! Uses libpcap (via the pcap crate) to capture real packets continuously until termination.
use pcap::{Capture, Device};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use vakthund_common::logger::{log_error, log_info};
use vakthund_common::packet::Packet;

pub fn capture_packets_loop<F>(
    terminate: &Arc<AtomicBool>,
    interface: &str,
    buffer_size: usize,
    promiscuous: bool,
    callback: &mut F,
) where
    F: FnMut(Packet),
{
    log_info(&format!(
        "Starting live capture on interface: {}",
        interface
    ));
    // Find the device.
    let device = Device::list()
        .unwrap()
        .into_iter()
        .find(|d| d.name == interface)
        .expect("Interface not found");
    // Open a live capture.
    let mut cap = Capture::from_device(device)
        .unwrap()
        .promisc(promiscuous)
        .snaplen(buffer_size as i32)
        .open()
        .unwrap();

    // Loop continuously until termination is requested.
    while !terminate.load(std::sync::atomic::Ordering::SeqCst) {
        match cap.next_packet() {
            Ok(packet) => {
                log_info(&format!(
                    "Captured packet with length: {}",
                    packet.header.len
                ));
                callback(Packet::new(packet.data.to_vec()));
            }
            Err(e) => {
                log_error(&format!("Error capturing packet: {}", e));
                // Sleep briefly on error.
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
