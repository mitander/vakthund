//! Monitor Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Provides monitoring and quarantine functionality based on traffic thresholds.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use vakthund_common::config::MonitorConfig;
use vakthund_common::packet::Packet;

#[derive(Debug, PartialEq, Eq)]
pub enum DetectionResult {
    ThreatDetected(String),
    NoThreat,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TrafficStats {
    packet_count: usize,
    total_bytes: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Monitor {
    config: MonitorConfig,
    stats: HashMap<String, TrafficStats>,
    #[serde(skip)]
    quarantined: HashMap<String, Instant>,
}

impl Monitor {
    /// Creates a new Monitor using the provided configuration.
    pub fn new(config: &MonitorConfig) -> Self {
        Self {
            config: config.clone(),
            stats: HashMap::new(),
            quarantined: HashMap::new(),
        }
    }

    /// Processes a packet, updating statistics and applying quarantine if thresholds are exceeded.
    pub fn process_packet(&mut self, packet: &Packet) {
        if let Some(src_ip) = Self::extract_src_ip(packet) {
            if self.config.whitelist.contains(&src_ip) {
                return;
            }
            let stats = self.stats.entry(src_ip.clone()).or_insert(TrafficStats {
                packet_count: 0,
                total_bytes: 0,
            });
            stats.packet_count += 1;
            stats.total_bytes += packet.data.len();
            if (stats.packet_count as f64) > self.config.thresholds.packet_rate
                || (stats.total_bytes as f64) > self.config.thresholds.data_volume
            {
                self.quarantined.insert(src_ip, Instant::now());
            }
        }
    }

    /// Checks if the given source IP is currently quarantined.
    pub fn is_quarantined(&mut self, src_ip: &str) -> bool {
        if let Some(&start) = self.quarantined.get(src_ip) {
            if Instant::now().duration_since(start)
                < Duration::from_secs(self.config.quarantine_timeout)
            {
                return true;
            } else {
                self.quarantined.remove(src_ip);
                self.stats.remove(src_ip);
            }
        }
        false
    }

    /// Extracts the source IP from a packet.
    pub fn extract_src_ip(packet: &Packet) -> Option<String> {
        let s = packet.as_str()?;
        let mut parts = s.split_whitespace();
        if parts.next()? != "IP" {
            return None;
        }
        Some(parts.next()?.to_string())
    }
}
