//! Logic for the 'check' command.

use crate::errors::Result;
use crate::ui;
use serde::Deserialize;

#[derive(Deserialize)]
struct CratesIoCrate {
    max_version: String,
}

#[derive(Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    krate: CratesIoCrate,
}

/// Checks the current version against the latest on crates.io.
pub async fn run() -> Result<()> {
    ui::print_banner();
    ui::info("Checking for latest version on crates.io...");

    let client = reqwest::Client::new();
    let res = client
        .get("https://crates.io/api/v1/crates/refinery-rs")
        .header("User-Agent", "refinery-rs (https://github.com/SirCesarium/refinery-rs)")
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                handle_success(response).await?;
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                ui::warn("Crate 'refinery-rs' not found on crates.io yet.");
            } else {
                ui::warn(&format!("Crates.io returned an error: {}", response.status()));
            }
        }
        Err(_) => {
            ui::warn("Could not connect to crates.io to check for updates.");
        }
    }

    Ok(())
}

async fn handle_success(response: reqwest::Response) -> Result<()> {
    if let Ok(data) = response.json::<CratesIoResponse>().await {
        let latest = data.krate.max_version;
        let current = env!("CARGO_PKG_VERSION");
        
        if latest == current {
            ui::success(&format!("You are on the latest version (v{current})"));
        } else {
            ui::warn(&format!("A new version is available on crates.io: v{latest}"));
            ui::info("To update, run: cargo install refinery-rs");
            ui::info("Check the changelog at: https://github.com/SirCesarium/refinery-rs/releases");
        }
    } else {
        ui::warn("Could not parse crates.io response.");
    }
    Ok(())
}
