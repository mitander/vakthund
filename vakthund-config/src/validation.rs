// vakthund-config/src/validation.rs
//! Custom validation functions for configuration.
//!
//! Provides shared validation logic used across multiple configuration modules.

use ipnetwork::IpNetwork;
use validator::ValidationError;

/// Validate that the provided CIDR list does not contain any invalid ranges.
pub fn validate_cidr_list(cidrs: &[IpNetwork]) -> Result<(), ValidationError> {
    if cidrs.iter().any(|n| match n {
        IpNetwork::V4(net) => net.ip().octets() == [0, 0, 0, 0],
        IpNetwork::V6(_) => false,
    }) {
        return Err(ValidationError::new("invalid_cidr"));
    }
    Ok(())
}

/// Validate that an interface name follows Linux naming conventions.
pub fn validate_interface(name: &str) -> Result<(), ValidationError> {
    let valid = !name.is_empty()
        && name.len() <= 15
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');

    let re =
        regex::Regex::new("^[a-zA-Z0-9_]+$").map_err(|_| ValidationError::new("invalid_regex"))?;

    if valid && re.is_match(name) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_interface"))
    }
}

/// Validate that a given value is a power of two.
pub fn validate_power_of_two(value: usize) -> Result<(), ValidationError> {
    if value.is_power_of_two() {
        Ok(())
    } else {
        Err(ValidationError::new("must_be_power_of_two"))
    }
}

/// Validate alert severity level.
pub fn validate_severity(level: &str) -> Result<(), ValidationError> {
    let valid = ["low", "medium", "high", "critical"].contains(&level.to_lowercase().as_str());
    if valid {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_severity"))
    }
}

/// Validate capture mode.
pub fn validate_mode(mode: &str) -> Result<(), ValidationError> {
    let re = regex::Regex::new("^(xdp|pcap|simulated)$")
        .map_err(|_| ValidationError::new("invalid_regex"))?;
    if re.is_match(mode) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_capture_mode"))
    }
}
