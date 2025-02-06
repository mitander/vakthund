//! ## vakthund-core::events
//! **Event bus using crossbeam's segmented queue for lock-free multi-producer handling**
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
use bytes::Bytes;
use crossbeam::queue::SegQueue;
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum EventError {
    #[error("Event queue capacity exceeded")]
    QueueFull,
}

/// Unified event type carrying protocol-agnostic payload
#[derive(Clone, Debug)]
pub struct NetworkEvent {
    pub timestamp: u64,
    pub payload: Bytes,
}

pub struct EventBus {
    queue: SegQueue<NetworkEvent>,
    capacity: usize,
}

impl EventBus {
    /// Create new event bus with fixed capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: SegQueue::new(),
            capacity,
        }
    }

    /// Enqueue event using Tigerbeetle-style *_verb naming
    pub fn event_enqueue(&self, event: NetworkEvent) -> Result<(), EventError> {
        if self.queue.len() >= self.capacity {
            return Err(EventError::QueueFull);
        }
        self.queue.push(event);
        Ok(())
    }

    /// Dequeue event with timeout
    pub fn event_dequeue(&self) -> Option<NetworkEvent> {
        self.queue.pop()
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::EventBus;
    use crate::prelude::NetworkEvent;
    use bytes::Bytes;

    #[test]
    fn enqueue_dequeue_roundtrip() {
        let bus = EventBus::with_capacity(1000);
        for i in 0..1000 {
            let event = NetworkEvent {
                timestamp: i as u64,
                payload: Bytes::from(vec![i as u8]),
            };
            bus.event_enqueue(event).unwrap();
        }

        for i in 0..1000 {
            let event = bus.event_dequeue().unwrap();
            assert_eq!(event.timestamp, i as u64);
            assert_eq!(event.payload[0], i as u8);
        }
    }
}
