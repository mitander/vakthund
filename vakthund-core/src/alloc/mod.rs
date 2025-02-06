//! ## vakthund-core::alloc
//! **Memory pools and arena allocators using `bumpalo`**
//!
//! ### Expectations (Production):
//! - Zero heap allocations in packet processing paths
//! - High-performance memory allocation/deallocation
//! - Memory safety and deterministic behavior
//!
//! ### Key Submodules:
//! - `pool/`: Fixed-size memory pools for common data structures
//! - `arena/`: Arena allocators using `bumpalo` for larger, temporary allocations
//! - `stats/`: Memory usage tracking and statistics
//!
//! ### Future:
//! - ARM-optimized memory allocators
//! - Integration with hardware memory management units (MMUs)

pub mod arena;
pub mod pool;
pub mod stats;
