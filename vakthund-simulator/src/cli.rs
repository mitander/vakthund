//! CLI module for the simulator.
// Note: This module might not be needed anymore as CLI args are handled in `vakthund-cli/src/commands.rs`
// Keeping it for potential future simulator-specific CLI needs.

use clap::Parser;

/// Command‑line arguments for the Vakthund Simulator.
#[derive(Parser, Debug, Clone)] // Make it derivable and cloneable if needed
#[command(author, version, about, long_about = None)]
pub struct SimulatorCli {
    /// Seed for the simulation
    #[arg(long)]
    pub seed: Option<u64>,

    /// Number of events to simulate
    #[arg(long, default_value_t = 10)]
    pub events: usize,

    /// Enable chaos fault injection
    #[arg(long, default_value_t = false)]
    pub chaos: bool,

    /// Path to a scenario file for replay (optional)
    #[arg(long)]
    pub replay: Option<String>,
}

pub fn parse_args() -> SimulatorCli {
    SimulatorCli::parse()
}
