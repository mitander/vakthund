//! Configuration provider trait for loading Vakthund configuration from various sources.

use figment::Figment;

use crate::ConfigError;

/// Trait for loading Vakthund configuration from different sources.
pub trait ConfigProvider {
    /// Loads the configuration and returns a Figment instance.
    fn load(&self) -> Result<Figment, ConfigError>;
}
