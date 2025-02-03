//! Detection engine
//!
//! Parses packets using MQTT and CoAP parsers, generating alerts or snapshots if a rule match occurs.
use crate::pipeline::monitor::{get_current_state, get_recent_events};
use crate::reporting::snapshots::{save_snapshot, Snapshot};
use crate::{
    config::Config,
    message_bus::Event,
    protocols::mqtt::MqttParser,
    reporting::alerts::{send_alert, AlertLevel},
};
use anyhow::Result;
use crossbeam_channel::Receiver;
use sha2::{Digest, Sha256};

/// Start the detection engine.
pub fn start(config: &Config, rx: Receiver<Event>) -> Result<()> {
    let mqtt_parser = MqttParser::new();
    let coap_parser = crate::protocols::coap::CoapParser::new();
    let config_clone = config.clone();
    std::thread::spawn(move || {
        for event in rx.iter() {
            if let Event::Packet { timestamp: _, data } = event {
                // Check for critical error marker.
                if data.windows(14).any(|w| w == b"CRITICAL_ERROR") {
                    tracing::error!("Critical error detected in packet!");
                    tracing::error!("Current configuration: {:?}", config_clone);
                    let monitor_state = get_current_state();
                    tracing::error!("Current monitor state: {}", monitor_state);
                    let recent_events = get_recent_events();
                    tracing::error!("Recent events: {:?}", recent_events);

                    let state_bytes = monitor_state.into_bytes();
                    let mut hasher = Sha256::new();
                    hasher.update(&state_bytes);
                    let checksum: [u8; 32] = hasher.finalize().into();
                    let snapshot = Snapshot {
                        timestamp: now_ns(),
                        state: state_bytes,
                        config: Some(format!("{:?}", config_clone)),
                        recent_events: Some(recent_events),
                        checksum,
                    };

                    if let Err(e) = save_snapshot(&snapshot, &config_clone.reporting.snapshots) {
                        tracing::error!("Failed to save snapshot: {:?}", e);
                    } else {
                        tracing::warn!("Snapshot saved as bug report.");
                    }
                } else {
                    if let Some(rule_id) = mqtt_parser.parse(&data) {
                        let alert_msg = format!("MQTT alert triggered: {}", rule_id);
                        send_alert(crate::reporting::alerts::Alert {
                            message: alert_msg,
                            level: AlertLevel::Warn,
                            packet: data.clone(),
                        });
                    }
                    if let Some(rule_id) = coap_parser.parse(&data) {
                        let alert_msg = format!("CoAP alert triggered: {}", rule_id);
                        send_alert(crate::reporting::alerts::Alert {
                            message: alert_msg,
                            level: AlertLevel::Warn,
                            packet: data.clone(),
                        });
                    }
                }
            }
        }
    });
    Ok(())
}

fn now_ns() -> u64 {
    unsafe {
        let mut ts = std::mem::MaybeUninit::uninit();
        libc::clock_gettime(libc::CLOCK_MONOTONIC, ts.as_mut_ptr());
        let ts = ts.assume_init();
        (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
    }
}
