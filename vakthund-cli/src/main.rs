//! ## vakthund-cli
//! **Unified operational interface**
//! Vakthund main entrypoint with deterministic simulation engine
//! and live (pcap-based) capture mode.
//!
//! ### Expectations:
//! - POSIX-compliant argument parsing
//! - Configuration templating
//! - Audit logging for all commands
//!
//! ### Future:
//! - Plugin system for custom commands
//! - SSH-based remote administration

use clap::Parser;
use vakthund_telemetry::logging::EventLogger;
use vakthund_telemetry::metrics::MetricsRecorder;

mod commands; // New module to handle commands

use commands::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    EventLogger::init();
    let metrics = MetricsRecorder::new();
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(run_args) => commands::run_production_mode(run_args, metrics).await,
        Commands::Simulate(sim_args) => commands::run_simulation_mode(sim_args, metrics).await,
    }
}
