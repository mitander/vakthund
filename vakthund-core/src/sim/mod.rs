//! ## vakthund-core::sim
//! **Deterministic simulation core with virtual clock**
//!
//! ### Expectations (Production):
//! - <2ms startup time for embedded deployments
//! - Zero heap allocations in packet processing paths
//! - Lock-free synchronization primitives
//!
//! ### Key Submodules:
//! - `alloc/`: Memory pools and arena allocators using `bumpalo`
//! - `events/`: Tokio-powered event bus with MPSC ringbuffer
//! - `sim/`: Deterministic simulation core with virtual clock
//! - `network/`: Network condition models (latency/jitter/packet loss)
//! - `time/`: `VirtualClock` using atomic counters + scheduler
//!
//! ### Future:
//! - ARM-optimized memory allocators
//! - Hardware timestamping support

use crate::events::NetworkEvent;
use crate::prelude::VirtualClock;
use std::sync::atomic::{AtomicUsize, Ordering}; // Correctly import atomic Ordering
use std::sync::Arc;

// TODO: can we avoid clone?
#[derive(Clone)]
pub struct ReplayEngine {
    scenario: Scenario, // Assuming Scenario is defined elsewhere or will be
    clock: VirtualClock,
    position: Arc<AtomicUsize>,
}

// Assuming Scenario struct definition for compilation.
// In real implementation, Scenario would be properly defined and loaded.
#[derive(Clone)]
pub struct Scenario {
    pub events: Vec<NetworkEventWithDelay>,
}

#[derive(Clone)]
pub struct NetworkEventWithDelay {
    pub event: NetworkEvent,
    pub delay_ns: u64,
}

impl ReplayEngine {
    pub fn new(scenario: Scenario, clock: VirtualClock) -> Self {
        Self {
            scenario,
            clock,
            position: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Get next event with virtual timing
    pub async fn next_event(&self) -> Option<NetworkEvent> {
        let pos = self.position.fetch_add(1, Ordering::Relaxed);
        let event_with_delay = self.scenario.events.get(pos)?;
        let event = &event_with_delay.event;

        // Advance clock exactly as recorded
        self.clock.advance(event_with_delay.delay_ns);
        Some(event.clone())
    }
}

// Long-running simulation harness
//
// async fn run_continuous_simulation() {
//     let scenarios = load_scenarios_from_s3().await;
//     let mut handles = Vec::new();

//     for scenario in scenarios {
//         let handle = tokio::spawn(async move {
//             let hash = run_simulation(scenario.path, scenario.seed, None).await;
//             store_simulation_result(hash).await;
//         });
//         handles.push(handle);
//     }

//     futures::future::join_all(handles).await;
// }
