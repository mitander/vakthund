use async_trait::async_trait;
use vakthund_core::events::network::NetworkEvent;
use vakthund_core::SimulationError;

use rand::Rng;
use vakthund_engine::engine::runtime_trait::SimulationDriver;

pub struct RandomizedEventDriver {
    event_count: usize,
    current_event: usize,
}

impl RandomizedEventDriver {
    pub fn new(event_count: usize) -> Self {
        RandomizedEventDriver {
            event_count,
            current_event: 0,
        }
    }
}

#[async_trait]
impl SimulationDriver for RandomizedEventDriver {
    async fn run(&mut self) -> Result<String, SimulationError> {
        println!("Running Random Event Driver");
        Ok("OK".to_string())
    }

    async fn next_event(&mut self) -> Result<Option<NetworkEvent>, SimulationError> {
        if self.current_event >= self.event_count {
            return Ok(None);
        }
        let mut rng = rand::thread_rng();
        let delay = rng.gen_range(1..1000);
        self.current_event += 1;

        Ok(Some(NetworkEvent::new(delay, "Random Packet".into())))
    }
}
