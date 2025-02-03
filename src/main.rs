//! Main entry point for Vakthund IDPS
//!
//! This binary initializes the CLI, loads the configuration, sets up logging,
//! and then starts the unified pipeline that handles capture (live or simulation),
//! detection, prevention, monitoring, reporting, and replay if requested.

mod cli;
mod config;
mod message_bus;
mod pipeline;
mod protocols;
mod reporting;
mod simulation;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber;

use cli::Cli;
use config::Config;
use pipeline::Pipeline;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.validate()?; // Validate CLI for conflicts (live vs. simulation vs. replay)

    let config = Config::load(&cli.config)?;
    // Initialize tracing subscriber (console logging)
    tracing_subscriber::fmt().init();

    // Build and run the pipeline.
    let pipeline = Pipeline::new(config, cli)?;
    pipeline.run()?;
    Ok(())
}
