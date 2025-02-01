//! # Simulation Module
//!
//! Implements a discrete event simulation for packet capture, inspired by Tigerbeetle’s techniques.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::thread;
use std::time::{Duration, Instant};
use vakthund_common::logger::log_info;
use vakthund_common::packet::Packet;

/// An event scheduled to occur at a specific simulation time.
struct Event {
    time: Instant,
    action: Box<dyn FnOnce() + Send>,
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse the order so that the earliest time has the highest priority.
        other.time.cmp(&self.time)
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for Event {}

/// A simple simulator that processes scheduled events.
struct Simulator {
    events: BinaryHeap<Event>,
}

impl Simulator {
    fn new() -> Self {
        Simulator {
            events: BinaryHeap::new(),
        }
    }

    /// Schedules an event to occur after a given delay.
    fn schedule<F>(&mut self, delay: Duration, action: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let event = Event {
            time: Instant::now() + delay,
            action: Box::new(action),
        };
        self.events.push(event);
    }

    /// Runs the simulation by processing events in order.
    fn run(mut self) {
        while let Some(event) = self.events.pop() {
            let now = Instant::now();
            if event.time > now {
                thread::sleep(event.time - now);
            }
            (event.action)();
        }
    }
}

/// Simulates the capture of network packets. After running the simulation,
/// it returns a fixed set of packets.
pub fn simulate_capture() -> Vec<Packet> {
    let mut simulator = Simulator::new();

    // Schedule events to simulate the capture of 5 packets.
    for i in 0..5 {
        simulator.schedule(Duration::from_millis(50 * i as u64), move || {
            let content = match i % 3 {
                0 => format!("MQTT CONNECT home/temperature_{}", i),
                1 => format!("COAP GET sensor/humidity_{}", i),
                _ => format!("normal PacketData_{}", i),
            };
            log_info(&format!("Simulated capture of packet {}: {}", i, content));
        });
    }

    // Run the simulation.
    simulator.run();

    // For simplicity, generate packets after simulation.
    let mut captured_packets = Vec::with_capacity(5);
    for i in 0..5 {
        let content = match i % 3 {
            0 => format!("MQTT CONNECT home/temperature_{}", i),
            1 => format!("COAP GET sensor/humidity_{}", i),
            _ => format!("normal PacketData_{}", i),
        };
        captured_packets.push(Packet::new(content.into_bytes()));
    }
    captured_packets
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_simulate_capture_count() {
        let packets = simulate_capture();
        assert_eq!(packets.len(), 5);
    }
}
