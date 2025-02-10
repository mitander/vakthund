//! Error types for configuration loading and validation

use std::path::PathBuf;
use thiserror::Error;
use validator::ValidationErrors;

/// Unified configuration error type.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// File not found error.
    #[error("Configuration file not found: {0}")]
    FileNotFound(PathBuf),

    /// Configuration validation error.
    #[error("Invalid configuration:\n{}", format_validation_errors(.0))]
    Validation(#[source] ValidationErrors),

    /// Figment parsing error.
    #[error("Configuration parsing error: {0}")]
    Parsing(#[from] figment::Error),

    /// I/O error.
    #[error("Configuration I/O error: {0}")]
    Io(#[from] std::io::Error),
}

fn format_validation_errors(errors: &ValidationErrors) -> String {
    use std::fmt::Write;

    let mut output = String::new();
    for (field, errors) in errors.field_errors() {
        let _ = writeln!(output, "Field '{}':", field);
        for error in errors {
            let message = match &error.message {
                Some(msg) => msg.to_string(),
                None => error.code.to_string(),
            };
            let _ = writeln!(output, "  - {}", message);
        }
    }
    output
}

impl From<ValidationErrors> for ConfigError {
    fn from(errors: ValidationErrors) -> Self {
        ConfigError::Validation(errors)
    }
}
