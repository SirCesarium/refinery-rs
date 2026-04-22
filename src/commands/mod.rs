pub mod build;
pub mod config;
#[cfg(feature = "pretty-cli")]
pub mod edit;
pub mod forge;
#[cfg(feature = "pretty-cli")]
pub mod init;
#[cfg(all(feature = "semver", feature = "pretty-cli"))]
pub mod release;
#[cfg(all(feature = "ci", feature = "pretty-cli"))]
pub mod setup;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "refinery")]
#[command(about = "🦀 Refining Rust into universal artifacts", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new refinery project
    #[cfg(feature = "pretty-cli")]
    Init(init::InitArgs),
    /// Edit an existing refinery project
    #[cfg(feature = "pretty-cli")]
    Edit(edit::EditArgs),
    /// Build project artifacts for the specified targets
    Build(build::BuildArgs),
    /// Generate release workflow (release.yml)
    Forge(forge::ForgeArgs),
    /// Setup CI/CD and installers
    #[cfg(all(feature = "ci", feature = "pretty-cli"))]
    Setup(setup::SetupArgs),
    /// Manage project releases
    #[cfg(all(feature = "semver", feature = "pretty-cli"))]
    Release(release::ReleaseArgs),
    /// Manage global configuration
    Config(config::ConfigArgs),
}

pub async fn handle_command(cli: Cli) -> Result<()> {
    match cli.command {
        #[cfg(feature = "pretty-cli")]
        Commands::Init(args) => init::run(&args),
        #[cfg(feature = "pretty-cli")]
        Commands::Edit(args) => edit::run(&args),
        Commands::Build(args) => build::run(&args),
        Commands::Forge(args) => forge::run(&args),
        #[cfg(all(feature = "ci", feature = "pretty-cli"))]
        Commands::Setup(args) => setup::run(&args),
        #[cfg(all(feature = "semver", feature = "pretty-cli"))]
        Commands::Release(args) => release::run(&args),
        Commands::Config(args) => config::run(&args),
    }
}
