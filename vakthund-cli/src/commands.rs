use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use vakthund_telemetry::logging::EventLogger;

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
    /// Run continuous fuzz testing with generated scenarios
    Fuzz(FuzzArgs),
}

#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    /// Network interface to monitor
    #[arg(short, long)]
    pub interface: String,

    /// Validate config before running
    #[arg(long, default_value_t = true)]
    pub validate: bool,
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

#[derive(Args, Debug, Clone)]
pub struct FuzzArgs {
    /// Initial seed for fuzzing (will auto-increment)
    #[arg(long, default_value_t = 1)]
    pub seed: u64,
    /// Number of fuzzing iterations (0 for unlimited)
    #[arg(long, default_value_t = 0)]
    pub iterations: usize,
    /// Maximum events per scenario
    #[arg(long, default_value_t = 1000)]
    pub max_events: usize,
}

pub async fn run_command(cli: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    EventLogger::init();

    match cli.command {
        Commands::Run(run_args) => {
            let config = vakthund_config::VakthundConfig::load()?;
            let runtime = std::sync::Arc::new(vakthund_engine::SimulationRuntime::new(config));
            runtime
                .run_production(&run_args.interface)
                .await
                .map_err(|e| e.into())
        }
        Commands::Simulate(sim_args) => {
            let config = vakthund_config::VakthundConfig::load()?;
            let runtime_config = config.clone();

            let runtime =
                std::sync::Arc::new(vakthund_engine::SimulationRuntime::new(runtime_config));

            // Use original config for simulator parameters
            let simulator = vakthund_simulator::Simulator::new(
                sim_args.seed,
                false,
                config.monitor.thresholds.packet_rate as u64,
                config.monitor.thresholds.connection_rate as u64,
                Some(runtime.event_bus.clone()),
            );

            runtime.run_simulator(simulator, sim_args.events).await?;
            Ok(())
        }
        Commands::Fuzz(fuzz_args) => {
            let config = vakthund_config::VakthundConfig::load()?;
            let runtime = std::sync::Arc::new(vakthund_engine::SimulationRuntime::new(config));
            runtime
                .run_fuzz_testing(fuzz_args.seed, fuzz_args.iterations, fuzz_args.max_events)
                .await
                .map_err(|e| e.into())
        }
    }
}
