//! Simulation Logging Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Initializes structured JSON logging for simulation runs. Logs are written to a file
//! in the `simulation_logs/` folder named "simulation_<seed>.log". Each log entry includes
//! the simulation seed, event ID, and timestamp.

use std::fs::{create_dir_all, OpenOptions};
use std::io::BufWriter;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::Mutex;
use tracing::info;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::EnvFilter;

pub fn init_simulation_logging(seed: u64) {
    let log_folder = "simulation_logs";
    if !Path::new(log_folder).exists() {
        create_dir_all(log_folder).expect("Failed to create simulation_logs folder");
    }
    let file_name = format!("{}/simulation_{}.log", log_folder, seed);
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o644)
        .open(&file_name)
        .expect("Failed to create log file");
    let writer = BufWriter::new(file);
    let make_writer = BoxMakeWriter::new(Mutex::new(writer));

    tracing_subscriber::fmt()
        .with_writer(make_writer)
        .json()
        .with_timer(UtcTime::rfc_3339())
        .with_env_filter(EnvFilter::new("info"))
        .init();

    info!(seed, "Simulation logging initialized");
}
