//! Simulation Capture Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Implements deterministic simulation capture using a seeded RNG. Each generated event
//! is tagged with an event ID and computed hash. A bug is injected at event ID 3 (malformed packet).
//! Supports replay by stopping at a specified event.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::info;
use vakthund_common::packet::Packet;

/// Computes a SHA-256 hash based on the seed and event ID.
pub fn compute_event_hash(seed: u64, event_id: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", seed, event_id));
    let result = hasher.finalize();
    hex::encode(result)
}

struct Event {
    time: Instant,
    action: Box<dyn FnOnce() + Send>,
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
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

/// A deterministic simulator for generating packet events.
pub struct DeterministicSimulator {
    events: BinaryHeap<Event>,
    rng: StdRng,
}

impl DeterministicSimulator {
    pub fn new(seed: u64) -> Self {
        Self {
            events: BinaryHeap::new(),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn schedule<F>(&mut self, delay: Duration, action: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let event = Event {
            time: Instant::now() + delay,
            action: Box::new(action),
        };
        self.events.push(event);
    }

    pub fn run(&mut self, terminate: &Arc<AtomicBool>) {
        while !terminate.load(AtomicOrdering::SeqCst) {
            if let Some(event) = self.events.pop() {
                let now = Instant::now();
                if event.time > now {
                    thread::sleep(event.time - now);
                }
                (event.action)();
            } else {
                break;
            }
        }
    }

    pub fn generate_packet_content(&mut self, event_id: usize) -> String {
        let base = if event_id == 3 {
            // Inject bug: malformed packet (missing topic)
            "MQTT CONNECT".to_string()
        } else {
            let r: u8 = self.rng.gen_range(0..3);
            match r {
                0 => format!("MQTT CONNECT alert/home_sim_{}", event_id),
                1 => format!("COAP GET sensor/alert_sim_{}", event_id),
                _ => format!("INFO system_ok_sim_{}", event_id),
            }
        };
        format!("ID:{} {}", event_id, base)
    }
}

/// Runs the simulation capture loop until termination or until an optional replay target event is reached.
/// Each generated event is passed to the callback as a Packet.
pub fn simulate_capture_loop<F>(
    terminate: &Arc<AtomicBool>,
    seed: Option<u64>,
    replay_target: Option<usize>,
    callback: &mut F,
) where
    F: FnMut(Packet),
{
    let seed = seed.unwrap_or(42);
    vakthund_common::sim_logging::init_simulation_logging(seed);
    let mut simulator = DeterministicSimulator::new(seed);
    let mut event_id = 0;
    info!("Starting simulation capture with seed: {}", seed);
    while !terminate.load(AtomicOrdering::SeqCst) {
        let delay = Duration::from_millis(50);
        let content = simulator.generate_packet_content(event_id);
        let event_hash = compute_event_hash(seed, event_id);
        println!(
            "{{\"timestamp\": \"{}\", \"seed\": {}, \"event_id\": {}, \"event_hash\": \"{}\", \"content\": \"{}\"}}",
            chrono::Utc::now().to_rfc3339(),
            seed,
            event_id,
            event_hash,
            content
        );
        if let Some(target) = replay_target {
            if event_id == target {
                callback(Packet::new(content.into_bytes()));
                break;
            }
        } else {
            callback(Packet::new(content.into_bytes()));
        }
        event_id += 1;
        thread::sleep(delay);
    }
}
