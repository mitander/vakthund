//! Command-line interface (CLI)
//!
//! Parses arguments and validates that only one operational mode is selected.
use clap::Parser;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Parser, Debug)]
#[command(
    name = "vakthund",
    version,
    about = "A deterministic, high‑performance IDPS"
)]
pub struct Cli {
    /// Path to the configuration YAML file
    #[arg(short, long, default_value = "config.yaml")]
    pub config: PathBuf,

    /// Seed for deterministic simulation (cannot be combined with interface or replay)
    #[arg(long)]
    pub seed: Option<u64>,

    /// Replay snapshot file for simulation replay (cannot be combined with seed or interface)
    #[arg(long)]
    pub replay: Option<PathBuf>,

    /// Network interface name for live capture (cannot be combined with seed or replay)
    #[arg(short, long)]
    pub interface: Option<String>,

    /// Path to an external snapshot file to load system state
    #[arg(long)]
    pub snapshot: Option<PathBuf>,
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error(
        "Conflicting operational modes specified (choose only one of: live, simulation, replay)"
    )]
    ModeConflict,
}

impl Cli {
    /// Validate CLI arguments to ensure that only one capture mode is specified.
    pub fn validate(&self) -> Result<(), CliError> {
        let mode_count = [
            self.seed.is_some(),
            self.replay.is_some(),
            self.interface.is_some(),
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        if mode_count > 1 {
            return Err(CliError::ModeConflict);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cli_validation_conflict() {
        let cli = Cli {
            config: "config.yaml".into(),
            seed: Some(42),
            replay: Some("snapshot.bin".into()),
            interface: None,
            snapshot: None,
        };
        assert!(cli.validate().is_err());
    }
    #[test]
    fn test_cli_validation_ok() {
        let cli = Cli {
            config: "config.yaml".into(),
            seed: Some(42),
            replay: None,
            interface: None,
            snapshot: None,
        };
        assert!(cli.validate().is_ok());
    }
}
