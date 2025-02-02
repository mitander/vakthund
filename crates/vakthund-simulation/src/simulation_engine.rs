//! Deterministic Simulation Engine
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Implements a deterministic simulation engine using a seeded RNG and an event scheduler.
//! Each event is recorded with an event ID and computed hash. A bug is injected at event ID 3.
//! Events are recorded via the Storage trait and can be replayed deterministically.

use crate::storage::Storage;
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
use tracing::info;
use vakthund_common::packet::Packet;

#[derive(Debug, Clone)]
pub struct SimEvent {
    pub event_id: usize,
    pub timestamp: i64,
    pub content: String,
    pub event_hash: String,
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

pub struct SimulationEngine<S: Storage> {
    pub storage: S,
    pub seed: u64,
    pub rng: StdRng,
    pub event_queue: BinaryHeap<Event>,
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

    /// Schedules an event to occur after a specified delay.
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

    /// Generates packet content for a given event ID.
    /// Injects a bug at event ID 3 by producing a malformed packet.
    pub fn generate_packet_content(&mut self, event_id: usize) -> String {
        let base = if event_id == 3 {
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

/// Computes a SHA-256 hash from the seed and event ID.
pub fn compute_event_hash(seed: u64, event_id: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", seed, event_id));
    let result = hasher.finalize();
    hex::encode(result)
}

/// Runs the simulation engine until termination or until a replay target event is reached.
/// Each event is recorded in the provided storage and passed to the callback as a Packet.
pub fn run_simulation<S, F>(
    terminate: &Arc<AtomicBool>,
    seed: Option<u64>,
    replay_target: Option<usize>,
    mut storage: S,
    mut callback: F,
) where
    S: Storage,
    F: FnMut(String),
{
    let seed = seed.unwrap_or(42);
    let mut engine = SimulationEngine::new(seed, storage);
    let mut event_id = 0;
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
    use crate::storage::{InMemoryStorage, Storage};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn test_simulation_engine_replay() {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut storage = InMemoryStorage::new();
        let seed = Some(42);
        let replay_target = Some(3);
        let mut events: Vec<String> = Vec::new();
        run_simulation(&terminate, seed, replay_target, storage, |content| {
            events.push(content);
        });
        assert!(events.last().unwrap().contains("MQTT CONNECT"));
    }
}
