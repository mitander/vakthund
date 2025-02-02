//! Simulation Storage Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Defines the Storage trait for simulation events and an in‑memory implementation.

use super::SimEvent;

pub trait Storage {
    fn record_event(&mut self, event: SimEvent);
    fn get_events(&self) -> &[SimEvent];
}

#[derive(Debug)]
pub struct InMemoryStorage {
    pub events: Vec<SimEvent>,
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
