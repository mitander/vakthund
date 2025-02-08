use crate::packet::Packet;
use pcap::{Capture, Device};
use std::sync::atomic::{AtomicBool, Ordering}; // Use crate-local Packet

/// The type for the callback function: it will receive a reference to a Packet.
pub type PacketCallback = dyn FnMut(&Packet) + Send;

/// Run a live capture loop on the specified interface.
/// This function will block until `terminate` is set to true.
pub fn run<F>(
    interface: &str,
    buffer_size: usize,
    promiscuous: bool,
    terminate: &AtomicBool,
    mut callback: F, // Use generic callback to avoid dyn FnMut cost if possible
) where
    F: FnMut(&Packet) + Send,
{
    // List available devices and select the one matching the interface name.
    let device = Device::list()
        .expect("Failed to list devices")
        .into_iter()
        .find(|d| d.name == interface)
        .unwrap_or_else(|| panic!("Device '{}' not found", interface)); // More informative panic

    // Open the capture on the selected device.
    let mut cap = Capture::from_device(device)
        .expect("Failed to open device")
        .promisc(promiscuous)
        .snaplen(buffer_size as i32)
        .timeout(1000) // timeout in ms (adjust as needed)
        .open()
        .expect("Failed to open capture");

    // Capture loop
    while !terminate.load(Ordering::Relaxed) {
        match cap.next_packet() {
            Ok(packet) => {
                let pkt = Packet {
                    data: packet.data.to_vec(),
                };
                callback(&pkt);
            }
            Err(pcap::Error::TimeoutExpired) => {
                // No packet received in this timeout window; just continue.
                continue;
            }
            Err(e) => {
                eprintln!("Error capturing packet: {:?}", e);
                break;
            }
        }
    }
}
