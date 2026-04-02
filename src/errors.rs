//! Error management for Refinery-RS.
//!
//! Provides a centralized error type using `thiserror` and `miette`
//! for beautiful diagnostic reporting.

use miette::Diagnostic;
use thiserror::Error;

/// Core error type for the refinery CLI.
#[derive(Debug, Error, Diagnostic)]
pub enum RefineryError {
    /// IO-related failures when managing workflow files.
    #[error("Failed to perform IO operation: {0}")]
    #[diagnostic(code(refinery::io_error), help("Check file permissions or if the directory exists."))]
    Io(#[from] std::io::Error),

    /// Errors occurring during the interactive prompt session.
    #[error("Failed to process interactive prompt: {0}")]
    #[diagnostic(code(refinery::prompt_error))]
    Prompt(#[from] inquire::InquireError),

    /// Error thrown when a workflow file already exists and force is not used.
    #[error("Workflow file '{0}' already exists.")]
    #[diagnostic(
        code(refinery::file_exists),
        help("Use the --force flag to overwrite existing workflow files.")
    )]
    FileExists(String),

    /// Serialization/Deserialization errors for YAML.
    #[error("Failed to process YAML configuration: {0}")]
    #[diagnostic(code(refinery::yaml_error))]
    Yaml(#[from] serde_yaml::Error),

    /// Network related errors for updates or external API calls.
    #[error("Network operation failed: {0}")]
    #[diagnostic(code(refinery::network_error))]
    Network(#[from] reqwest::Error),
}

/// Type alias for Refinery results.
pub type Result<T> = std::result::Result<T, RefineryError>;
