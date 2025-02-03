//! Packet acquisition layer with zero-copy buffer management
//!
//! Supports live capture (pcap) and deterministic simulation/replay.
use crate::{cli::Cli, config::Config, message_bus::Event};
use anyhow::Result;
use crossbeam_channel::Sender;

pub fn start(config: &Config, cli: &Cli, tx: Sender<Event>) -> Result<()> {
    match (cli.seed, &cli.replay, &cli.interface) {
        (Some(seed), _, _) => {
            crate::simulation::engine::start(seed, tx);
        }
        (_, Some(replay_path), _) => {
            crate::simulation::replay::start(replay_path.clone(), tx);
        }
        (_, _, Some(interface)) => {
            live_capture(interface, config, tx);
        }
        _ => {
            // Default to simulation with seed 42 if no mode is specified.
            crate::simulation::engine::start(42, tx);
        }
    }
    Ok(())
}

/// Live capture using pcap. For each captured packet, wrap the data in Bytes and send an Event::Packet.
fn live_capture(interface: &str, _config: &Config, tx: Sender<Event>) {
    assert!(!interface.is_empty(), "Interface name cannot be empty");

    let mut cap = match pcap::Capture::from_device(interface) {
        Ok(dev) => match dev.promisc(true).snaplen(65535).open() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to open device {}: {:?}", interface, e);
                return;
            }
        },
        Err(e) => {
            eprintln!("Device {} not found: {:?}", interface, e);
            return;
        }
    };

    while let Ok(packet) = cap.next_packet() {
        let data = bytes::Bytes::copy_from_slice(packet.data);
        let timestamp = now_ns();
        if let Err(e) = tx.send(Event::Packet { timestamp, data }) {
            eprintln!("Error sending packet: {:?}", e);
        }
    }
}

#[inline(always)]
fn now_ns() -> u64 {
    unsafe {
        let mut ts = std::mem::MaybeUninit::uninit();
        libc::clock_gettime(libc::CLOCK_MONOTONIC, ts.as_mut_ptr());
        let ts = ts.assume_init();
        (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
    }
}
