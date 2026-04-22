//! Refinery-RS
#![deny(
    clippy::all,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    clippy::absolute_paths
)]

mod commands;

use crate::commands::{Cli, handle_command};
use clap::Parser;
use refinery_rs::ui::error;
use std::process;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(err) = handle_command(cli).await {
        error(&err);
        process::exit(1);
    }
}
