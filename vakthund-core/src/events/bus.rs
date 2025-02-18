//! Thread-safe event bus implementation for high-frequency messaging.
//!
//! This module provides a lock-free, single-producer single-consumer (SPSC) event bus
//! using a circular buffer and atomic operations.
//!
//! Inspired by LMAX Disruptor pattern with optimizations for:
//! - Single Producer Single Consumer (SPSC) workloads
//! - Cache-line aware data layout
//! - Backpressure signaling

use super::network::NetworkEvent;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::error;

/// Event bus error conditions.
#[derive(Error, Debug)]
pub enum EventError {
    #[error("Event queue capacity exceeded")]
    QueueFull,
    #[error("Invalid capacity (must be a power of two)")]
    InvalidCapacity,
}

/// Cache-line aligned atomic counter to prevent false sharing
#[repr(align(64))]
struct AlignedCounter(AtomicU64);

impl AlignedCounter {
    #[inline]
    fn new(value: u64) -> Self {
        Self(AtomicU64::new(value))
    }
}

struct InnerBus {
    buffer: Box<[std::cell::UnsafeCell<Option<NetworkEvent>>]>,
    head: AlignedCounter,
    tail: AlignedCounter,
    mask: usize,
}

/// Thread-safe event bus for high-frequency messaging
pub struct EventBus {
    inner: Arc<InnerBus>,
}

impl EventBus {
    /// Creates new event bus with specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Must be a power of two for efficient modulo operations.
    pub fn with_capacity(capacity: usize) -> Result<Self, EventError> {
        if !capacity.is_power_of_two() {
            return Err(EventError::InvalidCapacity);
        }

        let buffer = (0..capacity)
            .map(|_| std::cell::UnsafeCell::new(None))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Ok(Self {
            inner: Arc::new(InnerBus {
                buffer,
                head: AlignedCounter::new(0),
                tail: AlignedCounter::new(0),
                mask: capacity - 1,
            }),
        })
    }

    /// Creates new handle to shared event bus.
    #[inline]
    pub fn share(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Attempts to send event onto the bus.
    ///
    /// # Safety
    ///
    /// Uses unsafe code for interior mutability guarded by atomic counters.
    #[inline]
    pub fn send(&self, event: NetworkEvent) -> Result<(), EventError> {
        let head = self.inner.head.0.load(Ordering::Relaxed);
        let tail = self.inner.tail.0.load(Ordering::Acquire);

        if head - tail >= self.inner.buffer.len() as u64 {
            return Err(EventError::QueueFull);
        }

        // SAFETY: Exclusive write access ensured by atomic counters
        unsafe {
            let idx = (head as usize) & self.inner.mask;
            *self.inner.buffer[idx].get() = Some(event)
        }

        self.inner.head.0.store(head + 1, Ordering::Release);
        Ok(())
    }

    /// Send event to event bus, blocks if queue is full.
    #[inline]
    pub fn send_blocking(&self, event: NetworkEvent) {
        loop {
            match self.send(event.clone()) {
                Ok(_) => break,
                Err(EventError::QueueFull) => {
                    std::thread::yield_now();
                }
                Err(e) => {
                    error!("Unexpected error during blocking push: {:?}", e);
                    break;
                }
            }
        }
    }

    /// Attempts to receive a event from the bus.
    ///
    /// Returns `None` if the queue is empty.
    #[inline]
    pub fn recv(&self) -> Option<NetworkEvent> {
        let tail = self.inner.tail.0.load(Ordering::Relaxed);
        let head = self.inner.head.0.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        // SAFETY: Exclusive read access ensured by atomic counters
        let event = unsafe {
            let idx = (tail as usize) & self.inner.mask;
            (*self.inner.buffer[idx].get()).take()
        };

        self.inner.tail.0.store(tail + 1, Ordering::Release);
        event
    }
}

// SAFETY: Thread safety ensured by atomic counters and Arc
unsafe impl Send for InnerBus {}
unsafe impl Sync for InnerBus {}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn test_event(seq: u64) -> NetworkEvent {
        NetworkEvent::new(seq, Bytes::from(format!("test-{}", seq)))
    }

    #[test]
    fn rejects_non_power_of_two() {
        assert!(matches!(
            EventBus::with_capacity(3),
            Err(EventError::InvalidCapacity)
        ));
    }

    #[test]
    fn handles_single_element() {
        let bus = EventBus::with_capacity(2).unwrap();
        let event = test_event(1);
        bus.send(event.clone()).unwrap();
        assert_eq!(bus.recv().unwrap().timestamp, 1);
    }

    #[test]
    fn signals_queue_full() {
        let bus = EventBus::with_capacity(2).unwrap();
        bus.send(test_event(1)).unwrap();
        bus.send(test_event(2)).unwrap();
        assert!(matches!(
            bus.send(test_event(3)),
            Err(EventError::QueueFull)
        ));
    }

    #[test]
    fn maintains_ordering() {
        let bus = EventBus::with_capacity(4).unwrap();
        bus.send(test_event(1)).unwrap();
        bus.send(test_event(2)).unwrap();
        assert_eq!(bus.recv().unwrap().timestamp, 1);
        assert_eq!(bus.recv().unwrap().timestamp, 2);
    }

    #[test]
    fn wraps_buffer_correctly() {
        let bus = EventBus::with_capacity(4).unwrap();
        for cycle in 0..2 {
            for i in 0..4 {
                bus.send(test_event(i + cycle * 4)).unwrap();
            }
            for i in 0..4 {
                assert_eq!(bus.recv().unwrap().timestamp, i + cycle * 4);
            }
        }
    }
}
