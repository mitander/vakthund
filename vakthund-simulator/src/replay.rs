// vakthund-simulator/src/replay.rs

/*!
# Vakthund Replay Engine

This module provides functionality to replay recorded simulation scenarios deterministically.
It uses a virtual clock and a sequence of events with associated delays to reproduce system behavior.

## Key Components:
- `Scenario`: Represents a sequence of network events with associated delay times.
- `NetworkEventWithDelay`: Associates a network event with a delay in nanoseconds.
- `ReplayEngine`: Drives the replay by advancing the virtual clock and providing events in order.

A simple stub function `load_from_file` is provided to load a scenario from a file.
*/

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use vakthund_core::events::NetworkEvent;
use vakthund_core::time::VirtualClock;

/// Represents a network event along with a delay (in nanoseconds) before it should be processed.
#[derive(Clone, Debug)]
pub struct NetworkEventWithDelay {
    pub event: NetworkEvent,
    pub delay_ns: u64,
}

/// A scenario is a sequence of network events with associated delays.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub events: Vec<NetworkEventWithDelay>,
}

impl Scenario {
    /// Loads a scenario from a file.
    /// This stub implementation assumes that each non-empty line in the file contains a delay (in nanoseconds).
    /// For each delay, a dummy network event is created.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let mut events = Vec::new();
        for line in content.lines() {
            if let Ok(delay_ns) = line.trim().parse::<u64>() {
                events.push(NetworkEventWithDelay {
                    event: NetworkEvent::new(delay_ns, bytes::Bytes::from("replayed event")),
                    delay_ns,
                });
            }
        }
        Ok(Scenario { events })
    }
}

/// The ReplayEngine replays a given scenario deterministically using a virtual clock.
#[derive(Clone)]
pub struct ReplayEngine {
    scenario: Scenario,
    clock: VirtualClock,
    position: Arc<AtomicUsize>,
}

impl ReplayEngine {
    /// Creates a new ReplayEngine with the provided scenario and virtual clock.
    pub fn new(scenario: Scenario, clock: VirtualClock) -> Self {
        Self {
            scenario,
            clock,
            position: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Retrieves the next event in the scenario, advancing the virtual clock by the specified delay.
    /// Returns `None` if the scenario is complete.
    pub async fn next_event(&self) -> Option<NetworkEvent> {
        let pos = self.position.fetch_add(1, Ordering::Relaxed);
        let event_with_delay = self.scenario.events.get(pos)?;
        self.clock.advance(event_with_delay.delay_ns);
        Some(event_with_delay.event.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn create_dummy_event() -> NetworkEvent {
        NetworkEvent {
            timestamp: 0,
            payload: Bytes::from("dummy"),
            source: Some("127.0.0.1:0".parse().unwrap()),
            destination: Some("127.0.0.1:0".parse().unwrap()),
        }
    }

    #[tokio::test]
    async fn test_replay_engine() {
        let events = vec![
            NetworkEventWithDelay {
                event: create_dummy_event(),
                delay_ns: 1_000,
            },
            NetworkEventWithDelay {
                event: create_dummy_event(),
                delay_ns: 2_000,
            },
        ];
        let scenario = Scenario { events };
        let clock = VirtualClock::new(0);
        let engine = ReplayEngine::new(scenario, clock.clone());
        let _e1 = engine.next_event().await;
        let _e2 = engine.next_event().await;
        // After two events, the clock should have advanced by 3000 ns.
        assert_eq!(clock.now_ns(), 3000);
    }
}
