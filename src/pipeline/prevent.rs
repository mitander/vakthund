//! Prevention subsystem
//!
//! Processes alert events and, if severity is Critical, applies a prevention action.
use crate::config::Config;
use crate::message_bus::{AlertLevel, Event, PreventionAction};
use anyhow::Result;
use crossbeam_channel::Receiver;

pub fn start(_config: &Config, rx: Receiver<Event>) -> Result<()> {
    std::thread::spawn(move || {
        for event in rx.iter() {
            if let Event::Alert {
                rule_id,
                packet: _,
                severity,
            } = event
            {
                if let AlertLevel::Critical = severity {
                    let action = PreventionAction::Block;
                    tracing::error!(
                        "Applying prevention action {:?} for rule {}",
                        action,
                        rule_id
                    );
                    // Here you would perform the actual blocking, e.g. call an OS API.
                }
            }
        }
    });
    Ok(())
}
