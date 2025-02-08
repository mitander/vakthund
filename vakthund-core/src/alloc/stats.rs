//! ## vakthund-core::alloc::stats
//! **Memory allocation statistics and tracking**
//!
//! This module provides functionality for tracking and reporting
//! memory allocation statistics within Vakthund's allocation system.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Global memory statistics tracker.
///
/// This struct uses atomic operations for thread-safe statistics tracking.
pub struct MemoryStats {
    pool_allocations: AtomicUsize,
    pool_deallocations: AtomicUsize,
    arena_allocations: AtomicUsize,
    arena_resets: AtomicUsize,
    // Add more stats as needed (e.g., bytes allocated, peak usage, etc.)
}

impl MemoryStats {
    /// Creates a new `MemoryStats` instance with all counters initialized to zero.
    pub fn new() -> Self {
        MemoryStats {
            pool_allocations: AtomicUsize::new(0),
            pool_deallocations: AtomicUsize::new(0),
            arena_allocations: AtomicUsize::new(0),
            arena_resets: AtomicUsize::new(0),
        }
    }

    /// Increments the count of memory pool allocations.
    #[inline]
    pub fn increment_pool_allocations(&self) {
        self.pool_allocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the count of memory pool deallocations.
    #[inline]
    pub fn increment_pool_deallocations(&self) {
        self.pool_deallocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the count of arena allocations.
    #[inline]
    pub fn increment_arena_allocations(&self) {
        self.arena_allocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the count of arena resets.
    #[inline]
    pub fn increment_arena_resets(&self) {
        self.arena_resets.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the current count of memory pool allocations.
    pub fn pool_allocations(&self) -> usize {
        self.pool_allocations.load(Ordering::Relaxed)
    }

    /// Returns the current count of memory pool deallocations.
    pub fn pool_deallocations(&self) -> usize {
        self.pool_deallocations.load(Ordering::Relaxed)
    }

    /// Returns the current count of arena allocations.
    pub fn arena_allocations(&self) -> usize {
        self.arena_allocations.load(Ordering::Relaxed)
    }

    /// Returns the current count of arena resets.
    pub fn arena_resets(&self) -> usize {
        self.arena_resets.load(Ordering::Relaxed)
    }

    // You can add methods to calculate derived stats or format output here.
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self::new()
    }
}

// You might consider using a global static instance of `MemoryStats`
// or passing it around as a dependency where needed.
//
// Example (using a static, be mindful of initialization order if using statics):
//
// ```
// static GLOBAL_MEMORY_STATS: MemoryStats = MemoryStats::new();
// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_stats_increment_and_read() {
        let stats = MemoryStats::new();
        assert_eq!(stats.pool_allocations(), 0);
        assert_eq!(stats.arena_allocations(), 0);

        stats.increment_pool_allocations();
        stats.increment_arena_allocations();

        assert_eq!(stats.pool_allocations(), 1);
        assert_eq!(stats.arena_allocations(), 1);
    }

    #[test]
    fn test_memory_stats_multiple_increments() {
        let stats = MemoryStats::new();
        for _ in 0..100 {
            stats.increment_pool_allocations();
            stats.increment_arena_allocations();
            stats.increment_pool_deallocations();
            stats.increment_arena_resets();
        }

        assert_eq!(stats.pool_allocations(), 100);
        assert_eq!(stats.arena_allocations(), 100);
        assert_eq!(stats.pool_deallocations(), 100);
        assert_eq!(stats.arena_resets(), 100);
    }
}
