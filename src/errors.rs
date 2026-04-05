#![allow(dead_code)]

use miette::Diagnostic;
use std::io::Error as StdIoError;
use std::result::Result as StdResult;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum RefineryError {
    #[error("{0}")]
    Generic(String),

    #[error("{0}")]
    Config(String),

    #[error("Failed to perform IO operation: {0}")]
    #[diagnostic(
        code(refinery::io_error),
        help("Check file permissions or if the directory exists.")
    )]
    Io(#[from] StdIoError),

    #[error("Failed to process interactive prompt: {0}")]
    #[diagnostic(code(refinery::prompt_error))]
    Prompt(#[from] inquire::InquireError),

    #[error("Workflow file '{0}' already exists.")]
    #[diagnostic(
        code(refinery::file_exists),
        help("Use the --force flag to overwrite existing workflow files.")
    )]
    FileExists(String),

    #[error("Failed to process YAML configuration: {0}")]
    #[diagnostic(code(refinery::yaml_error))]
    Yaml(#[from] serde_yaml::Error),

    #[error("Network operation failed: {0}")]
    #[diagnostic(code(refinery::network_error))]
    Network(#[from] reqwest::Error),
}

pub type Result<T> = StdResult<T, RefineryError>;
