//! # Vakthund Configuration System
//!
//! Hierarchical configuration management for the Vakthund IDPS following the
//! architecture's principles of determinism and safety.
//!
//! ## Features
//! - **Unified Configuration**: Single source of truth across all components
//! - **Validation**: Compile‑time and runtime validation of critical parameters
//! - **Environment Awareness**: Automatic configuration for production/simulation
//! - **Zero‑Copy Parsing**: Efficient handling of large configuration payloads

#![warn(unsafe_code)]
#![deny(rustdoc::broken_intra_doc_links)]

use std::path::{Path, PathBuf};

use figment::{
    providers::{Env, Format, Serialized, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

mod capture;
mod core;
mod error;
mod monitor;
mod prevention;
mod simulator;
mod telemetry;
mod validation;

pub use capture::CaptureConfig;
pub use core::CoreConfig;
pub use core::EventBusConfig;
pub use error::ConfigError;
pub use monitor::MonitorConfig;
pub use prevention::FirewallConfig;
pub use prevention::PreventionConfig;
pub use simulator::ChaosConfig;
pub use simulator::NetworkModelConfig;
pub use simulator::SimulatorConfig;
pub use telemetry::TelemetryConfig;

/// Top‑level configuration container for all Vakthund components.
#[derive(Debug, Serialize, Deserialize, Validate, Default)]
pub struct VakthundConfig {
    /// Core system configuration (event bus, memory, scheduling).
    #[validate(nested)]
    pub core: CoreConfig,

    /// Packet capture parameters (live and simulation modes).
    #[validate(nested)]
    pub capture: CaptureConfig,

    /// Telemetry and observability configuration.
    #[validate(nested)]
    pub telemetry: TelemetryConfig,

    /// Monitoring and alerting thresholds.
    #[validate(nested)]
    pub monitor: MonitorConfig,

    /// Prevention system parameters (firewall, rate limits).
    #[validate(nested)]
    pub prevention: PreventionConfig,
}

impl VakthundConfig {
    /// Load configuration from default files and environment.
    ///
    /// Hierarchy:
    /// 1. Default Values
    /// 2. `config/vakthund.yaml` - Base Vakthund settings. If missing, defaults are used.
    /// 3. `config/<environment>.yaml` - Environment‑specific overrides.
    /// 4. `VAKTHUND_*` environment variables.
    ///
    /// # Panics
    /// If validation fails on loaded configuration.
    pub fn load() -> Result<Self, ConfigError> {
        // Start with defaults.
        let mut figment = Figment::from(Serialized::defaults(VakthundConfig::default()));

        if Path::new("config/vakthund.yaml").exists() {
            figment = figment.merge(Yaml::file("config/vakthund.yaml"));
        } else {
            println!("config/vakthund.yaml not found, using default configuration");
        }

        let env = std::env::var("VAKTHUND_ENV").unwrap_or_else(|_| "production".into());
        let env_file = format!("config/{}.yaml", env);
        if Path::new(&env_file).exists() {
            figment = figment.merge(Yaml::file(env_file));
        }

        figment
            .merge(Env::prefixed("VAKTHUND_").split("__"))
            .extract()
            .map_err(ConfigError::from)
            .and_then(|config: Self| {
                config.validate()?;
                Ok(config)
            })
    }

    /// Load configuration from a specific path for testing/validation.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ConfigError::FileNotFound(PathBuf::from(
                path.to_string_lossy().to_string(),
            )));
        }

        Figment::new()
            .merge(Yaml::file(path))
            .merge(Env::prefixed("VAKTHUND_").split("__"))
            .extract()
            .map_err(ConfigError::from)
            .and_then(|config: Self| {
                config.validate()?;
                Ok(config)
            })
    }

    /// Load only SimulatorConfig from a specific path.
    pub fn load_simulator_config_from_path<P: AsRef<Path>>(
        path: P,
    ) -> Result<crate::simulator::SimulatorConfig, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            println!("Simulator config not found, loading defaults");
            return Ok(crate::simulator::SimulatorConfig::default());
        }

        Figment::new()
            .merge(Yaml::file(path))
            .extract()
            .map_err(ConfigError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_config_validation() {
        let config = VakthundConfig::default();
        config.validate().expect("Default config should validate");
    }

    #[test]
    fn environment_override() {
        // Override a field via environment variable.
        std::env::set_var("VAKTHUND_CORE__EVENT_BUS__CAPACITY", "8192");
        let config = VakthundConfig::load().unwrap();
        assert_eq!(config.core.event_bus.capacity, 8192);
    }
}
