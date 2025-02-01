//! # Simulation Engine for Vakthund
//!
//! This module implements a deterministic simulation engine using a seeded RNG and
//! a simple event scheduler. Each event (representing a packet) is recorded in a storage
//! implementation (here, we demonstrate an in-memory storage). The simulation is deterministic:
//! the same seed produces the same sequence of events. A bug is injected at event ID 3
//! (by returning a malformed packet). Each event is tagged with a computed hash so that
//! a particular event can later be replayed.
//!
//! In this example, the simulation engine runs until termination (e.g. via Ctrl‑C) or
//! until a replay target event ID is reached, at which point the simulation stops and
//! invokes a callback with that event’s content.

use chrono::Utc;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Represents a simulation event.
#[derive(Debug, Clone)]
pub struct SimEvent {
    pub event_id: usize,
    pub timestamp: i64, // Unix timestamp in seconds.
    pub content: String,
    pub event_hash: String,
}

/// A trait for storing simulation events.
pub trait Storage {
    /// Record an event.
    fn record_event(&mut self, event: SimEvent);
    /// Retrieve all recorded events.
    fn get_events(&self) -> &[SimEvent];
}

/// A simple in-memory storage for simulation events.
#[derive(Debug)]
pub struct InMemoryStorage {
    events: Vec<SimEvent>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }
}

impl Storage for InMemoryStorage {
    fn record_event(&mut self, event: SimEvent) {
        self.events.push(event);
    }
    fn get_events(&self) -> &[SimEvent] {
        &self.events
    }
}

/// A scheduled event for the simulator.
struct Event {
    time: Instant,
    action: Box<dyn FnOnce() + Send>,
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order: earlier events have higher priority.
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

/// The deterministic simulation engine.
pub struct SimulationEngine<S: Storage> {
    pub storage: S,
    pub seed: u64,
    pub rng: StdRng,
    event_queue: BinaryHeap<Event>,
}

impl<S: Storage> SimulationEngine<S> {
    /// Creates a new simulation engine with the given seed and storage.
    pub fn new(seed: u64, storage: S) -> Self {
        Self {
            storage,
            rng: StdRng::seed_from_u64(seed),
            event_queue: BinaryHeap::new(),
            seed,
        }
    }

    /// Schedules an event to occur after the specified delay.
    pub fn schedule_event<F>(&mut self, delay: Duration, action: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let event = Event {
            time: Instant::now() + delay,
            action: Box::new(action),
        };
        self.event_queue.push(event);
    }

    /// Runs scheduled events until the termination flag is set.
    pub fn run(&mut self, terminate: &Arc<AtomicBool>) {
        while !terminate.load(AtomicOrdering::SeqCst) {
            if let Some(event) = self.event_queue.pop() {
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

    /// Generates a simulated packet content for a given event ID.
    /// A bug is injected when the event ID equals 3 (by returning a malformed packet).
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
        // Embed the event ID at the start for traceability.
        format!("ID:{} {}", event_id, base)
    }
}

/// Computes a SHA-256 hash based on the simulation seed and event ID.
pub fn compute_event_hash(seed: u64, event_id: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", seed, event_id));
    let result = hasher.finalize();
    hex::encode(result)
}

/// Runs the simulation engine until termination (or until the replay target is reached).
/// - `terminate`: A flag to indicate when the simulation should stop.
/// - `seed`: An optional simulation seed; defaults to 42 if not provided.
/// - `replay_target`: An optional event ID at which to stop the simulation (to replay a specific event).
/// - `storage`: A storage implementation to record events.
/// - `callback`: A closure that receives each event’s content.
pub fn run_simulation<S, F>(
    terminate: &Arc<AtomicBool>,
    seed: Option<u64>,
    replay_target: Option<usize>,
    storage: S,
    mut callback: F,
) where
    S: Storage,
    F: FnMut(String),
{
    let seed = seed.unwrap_or(42);
    let mut engine = SimulationEngine::new(seed, storage);
    let mut event_id = 0;

    // Log a simulation header.
    println!("Starting simulation capture with seed: {}", seed);

    while !terminate.load(AtomicOrdering::SeqCst) {
        let delay = Duration::from_millis(50);
        let content = engine.generate_packet_content(event_id);
        let event_hash = compute_event_hash(seed, event_id);
        let timestamp = Utc::now().timestamp();
        let sim_event = SimEvent {
            event_id,
            timestamp,
            content: content.clone(),
            event_hash: event_hash.clone(),
        };
        engine.storage.record_event(sim_event);
        println!(
            "{{\"timestamp\": \"{}\", \"seed\": {}, \"event_id\": {}, \"event_hash\": \"{}\", \"content\": \"{}\"}}",
            Utc::now().to_rfc3339(),
            seed,
            event_id,
            event_hash,
            content
        );

        if let Some(target) = replay_target {
            if event_id == target {
                callback(content);
                break;
            }
        } else {
            callback(content);
        }
        event_id += 1;
        thread::sleep(delay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    // A simple in-memory storage for testing.
    struct TestStorage {
        events: Vec<SimEvent>,
    }
    impl TestStorage {
        fn new() -> Self {
            Self { events: Vec::new() }
        }
    }
    impl Storage for TestStorage {
        fn record_event(&mut self, event: SimEvent) {
            self.events.push(event);
        }
        fn get_events(&self) -> &[SimEvent] {
            &self.events
        }
    }

    #[test]
    fn test_simulation_engine_replay() {
        let terminate = Arc::new(AtomicBool::new(false));
        let storage = TestStorage::new();
        let seed = Some(42);
        let replay_target = Some(3); // We want to stop at event 3 (where the bug is injected)
        let mut events: Vec<String> = Vec::new();
        run_simulation(&terminate, seed, replay_target, storage, |content| {
            events.push(content);
        });
        // Verify that the last event (event 3) contains the injected bug.
        assert!(events.last().unwrap().contains("MQTT CONNECT"));
    }
}
