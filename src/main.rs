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
mod core;
mod errors;
mod macros;
mod ui;

#[tokio::main]
async fn main() -> miette::Result<()> {
    ui::print_banner();

    commands::execute().await?;

    Ok(())
}
