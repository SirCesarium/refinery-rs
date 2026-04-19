//! Refinery-RS Visual Test Suite
#![deny(
    clippy::all,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    clippy::absolute_paths
)]

mod errors;
mod ui;

use crate::ui::{
    error, info, inquire_text, print_banner, prompt, prompt_confirm, prompt_opt, success, warn,
};
use anyhow::{Context, Result};
use std::{process, time::Duration};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    if let Err(err) = run_test_suite().await {
        error(&err);
        process::exit(1);
    }
}

async fn run_test_suite() -> Result<()> {
    print_banner();
    println!();

    info("Initializing component test suite...");
    success("UI module loaded.");
    warn("System check warning.");
    println!();

    let sample = inquire_text("Stylized prompt text");
    info(&format!("Preview: {sample}"));
    println!();

    log_step!("󰄀", "=>", cyan, "Step with {} arguments: {}", 2, "OK");
    log_step!("󱓞", ">>", magenta, "Testing icon consistency...");
    println!();

    {
        let pb = spinner!("Processing metadata...");
        sleep(Duration::from_secs(1)).await;
        pb.set_message("Finalizing...");
        sleep(Duration::from_millis(500)).await;
        pb.finish_and_clear();
        success("Spinner task completed.");
    }
    println!();

    {
        let total = 100;
        let pb = progress!(total, "Refining artifacts");
        pb.inc(40);
        sleep(Duration::from_millis(300)).await;
        pb.inc(60);
        pb.finish_with_message("Artifacts refined.");
    }
    println!();

    if cfg!(feature = "pretty-cli") {
        let name = prompt("Enter project alias")?;
        println!();

        let proceed = prompt_confirm(&format!("Continue with '{name}'?"), true)?;
        println!();

        if proceed {
            let options = vec!["Debug", "Release", "Test"];
            let profile = prompt_opt("Select profile:", options)?;
            success(&format!("Profile set to: {profile}"));
        }
    } else {
        warn("Skipping interactive prompts (feature disabled).");
        let mock = ui::ProgressBarMock;
        mock.inc(10);
        mock.finish_and_clear();
    }
    println!();

    let root = anyhow::anyhow!("Low-level socket error");
    let err = Context::context(
        Err::<(), anyhow::Error>(root),
        "Failed to connect to backend",
    );
    let final_err = Context::context(err, "Could not initialize execution environment");

    if let Err(e) = final_err {
        error(&e);
    }
    println!();

    success("UI test suite finished.");
    Ok(())
}
