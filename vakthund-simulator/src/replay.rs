use crate::virtual_clock::VirtualClock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use vakthund_config::SimulatorConfig;
use vakthund_core::events::NetworkEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub seed: u64,
    pub config: SimulatorConfig,
    pub events: Vec<ScenarioEvent>,
    pub expected_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScenarioEvent {
    NetworkEvent { delay_ns: u64, event: NetworkEvent },
    NetworkDelay(u64),
    PacketLoss(f64),
    FaultInjection(String),
    CustomEvent { type_name: String, data: Vec<u8> },
}

impl Scenario {
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut events = Vec::new();

        // Add hash generation logic
        let mut hasher = blake3::Hasher::new();

        for line in content.lines() {
            if let Ok(delay_ns) = line.trim().parse::<u64>() {
                let event = ScenarioEvent::NetworkEvent {
                    delay_ns,
                    event: NetworkEvent::new(delay_ns, bytes::Bytes::from("replayed event")),
                };
                hasher.update(&delay_ns.to_be_bytes());
                events.push(event);
            }
        }

        Ok(Scenario {
            seed: 0,
            config: SimulatorConfig::default(),
            events,
            expected_hash: hex::encode(hasher.finalize().as_bytes()), // Generate hash
        })
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let serialized = serde_yaml::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, serialized)
    }
}

#[derive(Clone)]
pub struct ReplayEngine {
    scenario: Scenario,
    clock: VirtualClock,
    position: Arc<AtomicUsize>,
}

impl ReplayEngine {
    pub fn new(scenario: Scenario, clock: VirtualClock) -> Self {
        Self {
            scenario,
            clock,
            position: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn next_event(&self) -> Option<NetworkEvent> {
        let pos = self.position.fetch_add(1, Ordering::Relaxed);
        let event = self.scenario.events.get(pos)?;

        match event {
            ScenarioEvent::NetworkEvent { delay_ns, event } => {
                self.clock.advance(*delay_ns);
                Some(event.clone())
            }
            ScenarioEvent::NetworkDelay(delay) => {
                self.clock.advance(*delay);
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn create_scenario() -> Scenario {
        Scenario {
            seed: 123,
            config: SimulatorConfig::default(),
            expected_hash: "hash".to_string(),
            events: vec![
                ScenarioEvent::NetworkEvent {
                    delay_ns: 1_000,
                    event: NetworkEvent {
                        timestamp: 0,
                        payload: Bytes::from("dummy"),
                        source: None,
                        destination: None,
                    },
                },
                ScenarioEvent::NetworkEvent {
                    delay_ns: 2_000,
                    event: NetworkEvent {
                        timestamp: 0,
                        payload: Bytes::from("dummy"),
                        source: None,
                        destination: None,
                    },
                },
            ],
        }
    }

    #[tokio::test]
    async fn test_replay_engine() {
        let scenario = create_scenario();
        let clock = VirtualClock::new(0);
        let engine = ReplayEngine::new(scenario, clock.clone());

        let _e1 = engine.next_event().await;
        let _e2 = engine.next_event().await;

        assert_eq!(clock.now_ns(), 3000);
    }
}
