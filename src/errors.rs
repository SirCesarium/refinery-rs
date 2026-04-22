#![allow(dead_code)]

use std::io::Error as StdIoError;
use std::result::Result as StdResult;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RefineryError {
    #[error("{0}")]
    Generic(#[from] anyhow::Error),

    #[error("{0}")]
    Config(String),

    #[error("Failed to perform IO operation: {0}")]
    Io(#[from] StdIoError),

    #[error("Workflow file '{0}' already exists.")]
    FileExists(String),

    #[error("Failed to process YAML configuration: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Failed to process TOML configuration: {0}")]
    Toml(#[from] toml_edit::TomlError),

    #[cfg(feature = "pretty-cli")]
    #[error("Failed to process interactive prompt: {0}")]
    Prompt(#[from] inquire::InquireError),

    #[cfg(feature = "ci")]
    #[error("Network operation failed: {0}")]
    Network(#[from] reqwest::Error),
}

pub type Result<T> = StdResult<T, RefineryError>;
