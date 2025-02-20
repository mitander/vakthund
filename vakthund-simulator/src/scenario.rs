#[derive(Serialize, Deserialize)]
pub struct Scenario {
    pub seed: u64,
    pub config: SimulatorConfig,
    pub events: Vec<ScenarioEvent>,
}

#[derive(Serialize, Deserialize)]
pub enum ScenarioEvent {
    NetworkDelay(u64),
    PacketLoss(f64),
    FaultInjection(String),
    CustomEvent { type: String, data: Vec<u8> },
}
