//! Refinery-RS

#![deny(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    clippy::absolute_paths,
    missing_docs
)]

mod commands;
mod errors;
mod ui;

#[tokio::main]
async fn main() -> miette::Result<()> {
    ui::print_banner();

    commands::execute().await?;

    Ok(())
}
