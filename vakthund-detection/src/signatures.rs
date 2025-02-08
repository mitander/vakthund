//! ## vakthund-detection::signatures
//! **Aho-Corasick pattern matching with thread-safe updates**
//!
//! ### Expectations:
//! - Good detection latency (performance will depend on pattern set and input size)
//! - <0.1% false positive rate in validation corpus
//! - Thread-safe pattern updates
//! ### Components:
//! - `signatures/`: Aho-Corasick matcher
//! - `anomaly/`: Streaming PCA with incremental SVD
//! - `heuristics/`: Rule engine with WASM-based rules
//! ### Future:
//! - FPGA-accelerated pattern matching (if needed for extreme performance)
//! - Federated learning for anomaly models

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use parking_lot::RwLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Pattern compilation failed: {0}")]
    PatternError(String), // We'll use a generic string error for aho-corasick
}

pub struct SignatureEngine {
    patterns: RwLock<Vec<String>>, // Store patterns as Strings
    matcher: RwLock<Option<AhoCorasick>>,
}

impl SignatureEngine {
    pub fn new() -> Self {
        Self {
            patterns: RwLock::new(Vec::new()),
            matcher: RwLock::new(None),
        }
    }

    /// Add pattern using Tigerbeetle-style *_verb
    pub fn pattern_add(&self, pattern: &str) -> Result<(), DetectionError> {
        {
            let mut patterns = self.patterns.write();
            patterns.push(pattern.to_string());
        } // The write lock is dropped here.
        self.rebuild_matcher()
    }
    /// Rebuild Aho-Corasick matcher when patterns change
    fn rebuild_matcher(&self) -> Result<(), DetectionError> {
        let patterns = self.patterns.read();
        let matcher = AhoCorasickBuilder::new()
            .build(patterns.iter()) // Build from iterator of &String
            .map_err(|e| DetectionError::PatternError(e.to_string()))?; // Convert error

        *self.matcher.write() = Some(matcher);
        Ok(())
    }

    /// Scan buffer against current patterns
    #[inline] // Inlining for performance
    pub fn buffer_scan(&self, data: &[u8]) -> Vec<usize> {
        // Return Vec<usize> of match indices
        let matcher_read_guard = self.matcher.read(); // Acquire read lock once
        matcher_read_guard.as_ref().map_or(Vec::new(), |matcher| {
            matcher
                .find_overlapping_iter(data) // Find overlapping matches
                .map(|m| m.pattern().as_usize()) // Get pattern index as usize
                .collect()
        })
    }
}

impl Default for SignatureEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let engine = SignatureEngine::new();
        engine.pattern_add("test").unwrap();

        let matches = engine.buffer_scan(b"this is a test");
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_no_match() {
        let engine = SignatureEngine::new();
        engine.pattern_add("test").unwrap();

        let matches = engine.buffer_scan(b"no match here");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_multiple_patterns() {
        let engine = SignatureEngine::new();
        engine.pattern_add("test").unwrap();
        engine.pattern_add("example").unwrap();

        let matches = engine.buffer_scan(b"this is a test with an example");
        assert_eq!(matches.len(), 2); // Expecting two matches
        assert!(matches.contains(&0)); // Index 0 for "test"
        assert!(matches.contains(&1)); // Index 1 for "example"
    }
}
