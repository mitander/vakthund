//! ## vakthund-core::time
//! **Virtual clocks & scheduler**
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

// vakthund-core/src/time.rs
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct VirtualClock {
    // TODO: epoch
    // epoch: std::time::Instant,
    offset: Arc<AtomicU64>, // Nanoseconds
}

impl VirtualClock {
    pub fn new(seed: u64) -> Self {
        Self {
            // epoch: Instant::now(),
            offset: Arc::new(AtomicU64::new(seed)),
        }
    }

    /// TigerBeetle-style time access
    pub fn now_ns(&self) -> u64 {
        self.offset.load(Ordering::Acquire)
    }

    pub fn advance(&self, ns: u64) {
        self.offset.fetch_add(ns, Ordering::Release);
    }
}
