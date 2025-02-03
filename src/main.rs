//! # Vakthund IDPS Application
//!
//! Entry point for the Vakthund Intrusion Detection and Prevention System.
//! It initializes logging and runs the core pipeline.
use clap::{Arg, Command};

use vakthund_common::logger::log_info;
use vakthund_core::pipeline;

fn main() {
    log_info("Starting Vakthund IDPS application...");

    let matches = Command::new("Vakthund IDPS")
        .version("0.1.0")
        .author("Your Name <you@example.com>")
        .about("Unified event-driven IDPS with simulation and replay support")
        .arg(
            Arg::new("simulation_seed")
                .long("simulation-seed")
                .help("Overrides the simulation seed"),
        )
        .arg(
            Arg::new("replay_simulation")
                .long("replay-simulation")
                .help(
                "Replays a simulation by specifying a hash (seed + packet id) from a bug report",
            ),
        )
        .get_matches();

    let simulation_seed = matches
        .get_one::<String>("simulation_seed")
        .map(|s| s.parse::<u64>().expect("Invalid simulation seed"));
    let replay_simulation = matches
        .get_one::<String>("replay_simulation")
        .map(String::as_str);

    pipeline::run_vakthund(simulation_seed, replay_simulation);

    log_info("Vakthund IDPS application terminated.");
}
