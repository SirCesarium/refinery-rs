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
    /// Prepare the local environment with necessary machinery.
    Setup,
    /// Manage releases and tags.
    Release {
        #[command(subcommand)]
        action: ReleaseAction,
    },
}

#[derive(Subcommand)]
pub enum ReleaseAction {
    /// Create a patch release (v0.0.X).
    Patch {
        /// Open an editor to write a changelog.
        #[arg(short, long, default_value_t = false)]
        changelog: bool,
        /// Custom title for the release.
        #[arg(short, long)]
        title: Option<String>,
        /// Mark as a pre-release.
        #[arg(short, long, default_value_t = false)]
        prerelease: bool,
    },
    /// Create a minor release (v0.X.0).
    Minor {
        /// Open an editor to write a changelog.
        #[arg(short, long, default_value_t = false)]
        changelog: bool,
        /// Custom title for the release.
        #[arg(short, long)]
        title: Option<String>,
        /// Mark as a pre-release.
        #[arg(short, long, default_value_t = false)]
        prerelease: bool,
    },
    /// Create a major release (vX.0.0).
    Major {
        /// Open an editor to write a changelog.
        #[arg(short, long, default_value_t = false)]
        changelog: bool,
        /// Custom title for the release.
        #[arg(short, long)]
        title: Option<String>,
        /// Mark as a pre-release.
        #[arg(short, long, default_value_t = false)]
        prerelease: bool,
    },
    /// Delete a release tag locally and remotely.
    Delete {
        /// Name of the tag to delete.
        name: String,
    },
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
        Commands::Setup => {
            commands::setup::run().await.into_diagnostic()?;
        }
        Commands::Release { action } => {
            commands::release::run(action).into_diagnostic()?;
        }
    }

    Ok(())
}
