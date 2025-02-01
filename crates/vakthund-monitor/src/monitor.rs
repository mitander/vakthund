//! # Monitor Module
//!
//! Implements a simple traffic monitor that tracks per-IP statistics and
//! applies quarantine if thresholds are exceeded. The monitor uses the configuration
//! provided in the `monitor` section.
//!
//! For simplicity, this implementation assumes that packets beginning with "IP" contain
//! an IP header with the source IP as the second token. In a real system, a proper packet
//! parser would extract IP headers.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use vakthund_common::logger::log_warn;
use vakthund_common::packet::Packet;

#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub quarantine_timeout: u64,
    pub packet_rate: f64,
    pub data_volume: f64,
    pub port_entropy: f64,
    pub whitelist: HashSet<String>,
}

impl MonitorConfig {
    pub fn new(
        quarantine_timeout: u64,
        packet_rate: f64,
        data_volume: f64,
        port_entropy: f64,
        whitelist: Vec<String>,
    ) -> Self {
        Self {
            quarantine_timeout,
            packet_rate,
            data_volume,
            port_entropy,
            whitelist: whitelist.into_iter().collect(),
        }
    }
}

/// Maintains statistics per source IP and a list of quarantined IPs.
pub struct Monitor {
    config: MonitorConfig,
    stats: HashMap<String, TrafficStats>,
    quarantined: HashMap<String, Instant>,
}

#[derive(Default)]
struct TrafficStats {
    packet_count: usize,
    total_bytes: usize,
    // In a real implementation, you would store port distribution for entropy.
}

impl Monitor {
    /// Creates a new Monitor with the provided configuration.
    pub fn new(config: &MonitorConfig) -> Self {
        Monitor {
            config: config.clone(),
            stats: HashMap::new(),
            quarantined: HashMap::new(),
        }
    }

    /// Processes a packet: extracts source IP, updates stats, and applies quarantine if needed.
    pub fn process_packet(&mut self, packet: &Packet) {
        if let Some(src_ip) = Self::extract_src_ip(packet) {
            // Skip if IP is whitelisted.
            if self.config.whitelist.contains(&src_ip) {
                return;
            }

            let entry = self.stats.entry(src_ip.clone()).or_default();
            entry.packet_count += 1;
            entry.total_bytes += packet.data.len();

            // For demonstration, we skip actual entropy calculation.
            // Check if thresholds are exceeded.
            if (entry.packet_count as f64) > self.config.packet_rate
                || (entry.total_bytes as f64) > self.config.data_volume
            {
                log_warn(&format!(
                    "Threshold exceeded for IP {}. Quarantining.",
                    src_ip
                ));
                self.quarantined.insert(src_ip, Instant::now());
            }
        }
    }

    /// Checks whether the source IP is currently quarantined.
    pub fn is_quarantined(&mut self, src_ip: &str) -> bool {
        if let Some(&start) = self.quarantined.get(src_ip) {
            if Instant::now().duration_since(start)
                < Duration::from_secs(self.config.quarantine_timeout)
            {
                return true;
            } else {
                // Quarantine period expired; remove from quarantine and reset stats.
                self.quarantined.remove(src_ip);
                self.stats.remove(src_ip);
            }
        }
        false
    }

    /// Attempts to extract the source IP from the packet.
    /// For this demo, we expect packets to start with "IP <src_ip> <dst_ip> ..."
    pub fn extract_src_ip(packet: &Packet) -> Option<String> {
        let s = packet.as_str()?;
        let mut parts = s.split_whitespace();
        if parts.next()? != "IP" {
            return None;
        }
        Some(parts.next()?.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vakthund_common::packet::Packet;

    #[test]
    fn test_extract_src_ip_positive() {
        let pkt = Packet::new(
            "IP 192.168.1.100 10.0.0.1 some other data"
                .as_bytes()
                .to_vec(),
        );
        let ip = Monitor::extract_src_ip(&pkt);
        assert_eq!(ip, Some("192.168.1.100".into()));
    }

    #[test]
    fn test_extract_src_ip_negative() {
        let pkt = Packet::new("NOIP data".as_bytes().to_vec());
        let ip = Monitor::extract_src_ip(&pkt);
        assert!(ip.is_none());
    }

    #[test]
    fn test_quarantine_flow() {
        let config = MonitorConfig::new(2, 1.0, 100.0, 2.5, vec!["192.168.1.1".into()]);
        let mut monitor = Monitor::new(&config);
        // Create a packet from an IP not in whitelist.
        let pkt = Packet::new("IP 192.168.1.50 10.0.0.1 payload".as_bytes().to_vec());
        // Process multiple packets to exceed packet_rate threshold.
        for _ in 0..2 {
            monitor.process_packet(&pkt);
        }
        assert!(monitor.is_quarantined("192.168.1.50"));
    }
}
