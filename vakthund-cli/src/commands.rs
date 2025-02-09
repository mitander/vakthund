use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use vakthund_engine::run_production_mode;
use vakthund_engine::run_simulation_mode;
use vakthund_telemetry::metrics::MetricsRecorder;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run in production mode (live capture using pcap)
    Run(RunArgs),
    /// Run deterministic simulation (or replay if a scenario file is provided)
    Simulate(SimulateArgs),
}

#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    #[arg(short, long)]
    pub interface: String,
}

#[derive(Args, Debug, Clone)]
pub struct SimulateArgs {
    /// Optional scenario file to replay; if not provided, a simulation will be run.
    #[arg(short, long)]
    pub scenario: Option<PathBuf>,
    /// Number of events to simulate (used when no scenario is provided)
    #[arg(long, default_value_t = 10)]
    pub events: usize,
    #[arg(long, default_value_t = 0)]
    pub seed: u64,
    #[arg(long)]
    pub validate_hash: Option<String>,
}

pub async fn run_command(cli: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let metrics = MetricsRecorder::new();
    match cli.command {
        Commands::Run(run_args) => run_production_mode(&run_args.interface, metrics).await,
        Commands::Simulate(sim_args) => {
            run_simulation_mode(
                sim_args.scenario,
                sim_args.events,
                sim_args.seed,
                sim_args.validate_hash.as_deref(),
                metrics,
            )
            .await
        }
    }
}
