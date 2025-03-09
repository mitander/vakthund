use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use vakthund_engine::engine::default_driver::DefaultSimulationDriver;
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

            // Create a dummy simulator that won't be used in production mode
            let simulator = vakthund_simulator::Simulator::new(
                0,     // seed
                false, // chaos
                0,     // latency
                0,     // jitter
                None,  // no event bus yet
            );

            // Create a driver with dummy values since it won't be used in production mode
            let driver = DefaultSimulationDriver::new(simulator, 0);

            let runtime =
                std::sync::Arc::new(vakthund_engine::SimulationRuntime::new(config, driver));
            runtime
                .run_production(&run_args.interface)
                .await
                .map_err(|e| e.into())
        }
        Commands::Simulate(sim_args) => {
            let config = vakthund_config::VakthundConfig::load()?;

            // Use original config for simulator parameters
            let simulator = vakthund_simulator::Simulator::new(
                sim_args.seed,
                false,
                config.monitor.thresholds.packet_rate as u64,
                config.monitor.thresholds.connection_rate as u64,
                None, // We'll set this after runtime creation
            );

            // Create the driver with the simulator
            let driver = DefaultSimulationDriver::new(simulator, sim_args.events);

            // Create the runtime with the driver
            let runtime = std::sync::Arc::new(vakthund_engine::SimulationRuntime::new(
                config.clone(),
                driver,
            ));

            // Run the simulation
            let result = runtime.run_simulation(sim_args.events).await?;

            println!("Simulation completed with hash: {}", result);
            Ok(())
        }
        Commands::Fuzz(fuzz_args) => {
            let config = vakthund_config::VakthundConfig::load()?;

            // Create a dummy simulator for the driver
            let simulator = vakthund_simulator::Simulator::new(
                fuzz_args.seed,
                false,
                0,    // Doesn't matter for fuzz testing
                0,    // Doesn't matter for fuzz testing
                None, // We'll set this after runtime creation
            );

            // Create a driver with the simulator
            let driver = DefaultSimulationDriver::new(simulator, 0); // Events don't matter for fuzz mode

            // Create and wrap the runtime
            let runtime = vakthund_engine::SimulationRuntime::new(config, driver);
            let runtime_arc = std::sync::Arc::new(runtime);

            runtime_arc
                .run_fuzz_testing(fuzz_args.seed, fuzz_args.iterations, fuzz_args.max_events)
                .await
                .map_err(|e| e.into())
        }
    }
}
