//! Unified processing pipeline
//!
//! Combines capture, detection, prevention, and monitoring stages.
pub mod capture;
pub mod detect;
pub mod monitor;
pub mod prevent;

use crate::{cli::Cli, config::Config, message_bus::MessageBus};
use anyhow::Result;

pub struct Pipeline {
    bus: MessageBus,
}

impl Pipeline {
    pub fn new(config: Config, cli: Cli) -> Result<Self> {
        let bus = MessageBus::new(1024);
        capture::start(&config, &cli, bus.tx.clone())?;
        detect::start(&config, bus.rx.clone())?;
        prevent::start(&config, bus.rx.clone())?;
        monitor::start(&config, bus.rx.clone())?;
        Ok(Self { bus })
    }

    pub fn run(self) -> Result<()> {
        // Main thread waits indefinitely; in a real system, add graceful shutdown.
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
