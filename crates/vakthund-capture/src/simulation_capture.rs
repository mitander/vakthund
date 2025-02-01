//! # Simulation Capture Module with Replay Support
//!
//! Implements a deterministic packet capture simulation using a seeded RNG and a discrete event scheduler.
//! Each packet includes an event ID (prefixed as "ID:<counter>") and is logged with its computed hash.
//! At packet counter 3, a bug is injected by generating a malformed packet (i.e. "MQTT CONNECT" with no topic).
//! Optionally, if a replay target hash is provided, the simulation stops when that event is reached.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::info;
use vakthund_common::packet::Packet;

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

struct DeterministicSimulator {
    events: BinaryHeap<Event>,
    rng: StdRng,
}

impl DeterministicSimulator {
    /// Creates a new simulator with the provided seed.
    pub fn new(seed: u64) -> Self {
        DeterministicSimulator {
            events: BinaryHeap::new(),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Schedules an event after the given delay.
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

    /// Runs scheduled events until termination is signaled.
    pub fn run(&mut self, terminate: &Arc<AtomicBool>) {
        while !terminate.load(std::sync::atomic::Ordering::SeqCst) {
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

    /// Generates a simulated packet content with the packet counter embedded.
    /// When the counter equals 3, a bug is injected by returning a malformed packet.
    pub fn generate_packet_content(&mut self, i: usize) -> String {
        let base = if i == 3 {
            // Inject bug: malformed packet (missing topic)
            "MQTT CONNECT".to_string()
        } else {
            let r: u8 = self.rng.gen_range(0..3);
            match r {
                0 => format!("MQTT CONNECT alert/home_sim_{}", i),
                1 => format!("COAP GET sensor/alert_sim_{}", i),
                _ => format!("INFO system_ok_sim_{}", i),
            }
        };
        format!("ID:{} {}", i, base)
    }
}

/// Computes a SHA-256 hash for an event using the simulation seed and packet counter.
fn compute_event_hash(seed: u64, event_id: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", seed, event_id));
    let result = hasher.finalize();
    hex::encode(result)
}

/// Runs the deterministic simulation continuously until termination.
/// If a replay_target hash is provided, the simulation stops when an event's hash matches.
pub fn simulate_capture_loop<F>(
    terminate: &Arc<AtomicBool>,
    seed: Option<u64>,
    replay_target: Option<String>,
    callback: &mut F,
) where
    F: FnMut(Packet),
{
    let seed = seed.unwrap_or(42);
    vakthund_common::sim_logging::init_simulation_logging(seed);

    let mut simulator = DeterministicSimulator::new(seed);
    let mut packet_counter = 0;

    info!("Starting simulation capture with seed: {}", seed);

    while !terminate.load(std::sync::atomic::Ordering::SeqCst) {
        let delay = Duration::from_millis(50);
        let content = simulator.generate_packet_content(packet_counter);
        let event_hash = compute_event_hash(seed, packet_counter);
        info!(
            seed = seed,
            event_id = packet_counter,
            event_hash = %event_hash,
            content = %content,
            "Simulated event generated"
        );
        // If replay_target is provided and matches, immediately process the event and exit.
        if let Some(ref target) = replay_target {
            if &event_hash == target {
                info!(%event_hash, "Replay target event reached. Exiting simulation loop.");
                callback(Packet::new(content.into_bytes()));
                break;
            }
        }
        simulator.schedule(delay, || {});
        simulator.run(terminate);
        callback(Packet::new(content.into_bytes()));
        packet_counter += 1;
    }
}
