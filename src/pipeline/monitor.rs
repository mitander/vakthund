//! Monitoring subsystem
//!
//! Maintains real-time state and recent events.
use crate::config::Config;
use crate::message_bus::Event;
use anyhow::Result;
use crossbeam_channel::Receiver;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// SystemMonitor stores real-time metrics.
#[derive(Debug, Default)]
pub struct SystemMonitor {
    pub packet_count: usize,
    pub recent_events: Vec<String>,
}

/// Global monitor state.
pub static MONITOR_STATE: Lazy<Mutex<SystemMonitor>> =
    Lazy::new(|| Mutex::new(SystemMonitor::default()));

/// Start the monitoring thread.
pub fn start(_config: &Config, rx: Receiver<Event>) -> Result<()> {
    std::thread::spawn(move || {
        for event in rx.iter() {
            if let Event::Packet { .. } = event {
                let mut m = MONITOR_STATE.lock();
                m.packet_count += 1;
                let count = m.packet_count; // avoid multiple borrows
                m.recent_events.push(format!("Packet #{}", count));
                if m.recent_events.len() > 10 {
                    m.recent_events.remove(0);
                }
            }
        }
    });
    Ok(())
}

/// Returns a string representing the current monitor state.
pub fn get_current_state() -> String {
    let m = MONITOR_STATE.lock();
    format!("Packet count: {}", m.packet_count)
}

/// Returns a vector of recent event descriptions.
pub fn get_recent_events() -> Vec<String> {
    let m = MONITOR_STATE.lock();
    m.recent_events.clone()
}
