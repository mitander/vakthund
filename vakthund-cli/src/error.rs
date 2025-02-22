use vakthund_engine::engine::SimulationError;

impl From<SimulationError> for Box<dyn std::error::Error + Send + Sync> {
    fn from(err: SimulationError) -> Self {
        Box::new(err) as Box<dyn std::error::Error + Send + Sync>
    }
}
