use clap::{Parser, Subcommand};
use miette::Result;

pub mod init;
pub mod release;

pub struct Actions;

#[derive(Parser)]
#[command(name = "refinery", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init,
    Release(release::ReleaseArgs),
}

/// Orchestrates CLI execution.
pub async fn execute() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => Actions::init().await?,
        Commands::Release(args) => Actions::release(args).await?,
    }

    Ok(())
}
