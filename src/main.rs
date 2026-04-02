//! Refinery-RS CLI tool for surgical workflow management.

#![warn(clippy::pedantic, clippy::nursery)]
#![deny(clippy::unwrap_used, clippy::expect_used)]

mod commands;
mod errors;
mod macros;
mod models;
mod ui;
mod writer;

use clap::{Parser, Subcommand};
use miette::IntoDiagnostic;

#[derive(Parser)]
#[command(name = "refinery")]
#[command(bin_name = "refinery")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new refinery workflow.
    Init {
        /// Force overwrite of existing workflow files.
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
    /// Check for the latest version of Refinery on crates.io.
    Check,
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { force } => {
            commands::init::run(force).into_diagnostic()?;
        }
        Commands::Check => {
            commands::check::run().await.into_diagnostic()?;
        }
    }

    Ok(())
}
