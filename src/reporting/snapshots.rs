//! Snapshotting subsystem
//!
//! Captures and restores system state (e.g., monitor metrics, event history)
//! for deterministic replay and forensic analysis.

use anyhow::Result;
use bincode;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub timestamp: u64,
    pub state: Vec<u8>,
    pub config: Option<String>,
    pub recent_events: Option<Vec<String>>,
    pub checksum: [u8; 32],
}

pub fn init_snapshots(config: &crate::config::SnapshotConfig) -> Result<()> {
    let dir = Path::new(&config.directory);
    fs::create_dir_all(dir)?;
    Ok(())
}

pub fn save_snapshot(snapshot: &Snapshot, config: &crate::config::SnapshotConfig) -> Result<()> {
    let dir = Path::new(&config.directory);
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("snapshot_{}.bin", snapshot.timestamp));
    let data = bincode::serialize(snapshot)?;
    fs::write(path, data)?;
    Ok(())
}

pub fn load_snapshot<P: AsRef<Path>>(path: P) -> Result<Snapshot> {
    let data = fs::read(path)?;
    let snapshot = bincode::deserialize(&data)?;
    Ok(snapshot)
}
