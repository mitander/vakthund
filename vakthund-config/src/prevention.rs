//! Prevention system configuration.
//!
//! Parameters for real‑time mitigation systems:
//! - eBPF‑based firewall
//! - Rate limiting
//! - Device quarantine

use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use validator::{self, Validate};

use crate::monitor::QuarantineConfig;
use crate::validation;

#[derive(Default, Debug, Serialize, Deserialize, Validate, Clone)]
pub struct RateLimitConfig {}

/// Prevention system configuration.
#[derive(Default, Debug, Serialize, Deserialize, Validate, Clone)]
pub struct PreventionConfig {
    /// eBPF firewall configuration.
    #[validate(nested)]
    pub firewall: FirewallConfig,

    /// Rate limiting parameters.
    #[validate(nested)]
    pub rate_limits: RateLimitConfig,

    /// Quarantine parameters.
    #[validate(nested)]
    pub quarantine: QuarantineConfig,
}

/// eBPF firewall configuration.
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct FirewallConfig {
    /// Network interface for XDP program.
    #[validate(custom(function = validation::validate_interface))]
    #[serde(default = "default_interface")]
    pub interface: String,

    /// Maximum firewall rules.
    #[validate(range(min = 100, max = 100000))]
    #[serde(default = "default_max_rules")]
    pub max_rules: usize,

    /// Whitelisted IP ranges.
    #[serde(default)]
    pub whitelist: Vec<IpNetwork>,
}

fn default_interface() -> String {
    "eth0".into()
}
fn default_max_rules() -> usize {
    10000
}

impl Default for FirewallConfig {
    fn default() -> Self {
        Self {
            interface: default_interface(),
            max_rules: default_max_rules(),
            whitelist: Vec::new(),
        }
    }
}
