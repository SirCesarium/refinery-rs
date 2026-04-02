//! Logic for the 'setup' command to prepare the local development environment.

use crate::errors::Result;
use crate::ui;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::process::Command;
use toml_edit::{DocumentMut, value};

#[derive(Deserialize)]
struct DockerTag {
    name: String,
}

#[derive(Deserialize)]
struct DockerHubResponse {
    results: Vec<DockerTag>,
}

/// Runs the local setup process.
/// 
/// # Errors
/// Returns an error if cargo installation or network requests fail.
pub async fn run() -> Result<()> {
    ui::print_banner();
    ui::info("Starting intelligent factory setup...");

    ensure_cargo_toml()?;

    if !Path::new(".github/workflows/release.yml").exists() {
        ui::warn("Release workflow not found ('.github/workflows/release.yml').");
        ui::info("It's recommended to run 'refinery init' first.");
        println!();
    }

    setup_packaging_machinery()?;
    setup_docker_environment().await?;

    ui::success("Local environment is now fully equipped!");
    Ok(())
}

fn ensure_cargo_toml() -> Result<()> {
    if !Path::new("Cargo.toml").exists() {
        ui::warn("No Cargo.toml found in the current directory.");
        if ui::prompt_confirm("Do you want to initialize a new Rust project with 'cargo init'?", true)? {
            ui::info("Executing: cargo init...");
            let status = Command::new("cargo").arg("init").status()?;
            if !status.success() {
                return Err(std::io::Error::other("Failed to initialize cargo project").into());
            }
            ui::success("Cargo project initialized successfully.");
        } else {
            ui::info("Exiting setup. Refinery requires a Cargo.toml to continue.");
            std::process::exit(0);
        }
    }
    Ok(())
}
fn setup_packaging_machinery() -> Result<()> {
    println!(
        "\n{}\n",
        console::Style::new()
            .cyan()
            .apply_to("--- Packaging Setup ---")
    );

    let all_tools = if cfg!(target_os = "windows") {
        vec!["cargo-wix"]
    } else if cfg!(target_os = "linux") {
        vec!["cargo-deb", "cargo-generate-rpm"]
    } else {
        vec![]
    };

    let missing_tools: Vec<&str> = all_tools
        .iter()
        .filter(|&&t| !is_installed(t))
        .copied()
        .collect();

    if !missing_tools.is_empty() {
        ui::info(&format!(
            "Planned command: cargo install {}",
            missing_tools.join(" ")
        ));
        if ui::prompt_confirm("Install missing packaging tools globaly?", false)? {
            let status = Command::new("cargo")
                .arg("install")
                .args(&missing_tools)
                .status()?;
            if status.success() {
                ui::success("Machinery installed.");
            }
        }
    }

    if cfg!(target_os = "linux") {
        configure_debian()?;
        configure_rpm()?;
    } else if cfg!(target_os = "windows") {
        configure_wix()?;
    }

    Ok(())
}

fn configure_debian() -> Result<()> {
    if !is_installed("cargo-deb") {
        return Ok(());
    }

    ui::info("Checking Debian (.deb) configuration...");
    let toml_content = fs::read_to_string("Cargo.toml")?;
    if toml_content.contains("[package.metadata.deb]") {
        ui::success("Debian metadata already exists in Cargo.toml.");
        return Ok(());
    }

    if ui::prompt_confirm(
        "Would you like to initialize Debian packaging metadata?",
        true,
    )? {
        let maintainer =
            ui::prompt_text("Maintainer name & email:", "Developer <dev@example.com>")?;
        let description = ui::prompt_text("Package description:", "A surgical Rust tool.")?;

        let mut doc = toml_content
            .parse::<DocumentMut>()
            .map_err(std::io::Error::other)?;
        doc["package"]["metadata"]["deb"]["maintainer"] = value(maintainer);
        doc["package"]["metadata"]["deb"]["description"] = value(description);

        fs::write("Cargo.toml", doc.to_string())?;
        ui::success("Debian metadata added to Cargo.toml.");
    }
    Ok(())
}

fn configure_rpm() -> Result<()> {
    if !is_installed("cargo-generate-rpm") {
        return Ok(());
    }

    ui::info("Checking RPM (.rpm) configuration...");
    let toml_content = fs::read_to_string("Cargo.toml")?;
    if toml_content.contains("[package.metadata.rpm]") {
        ui::success("RPM metadata already exists in Cargo.toml.");
        return Ok(());
    }

    if ui::prompt_confirm("Would you like to initialize RPM packaging metadata?", true)? {
        let license = ui::prompt_text("License (e.g. MIT):", "MIT")?;

        let mut doc = toml_content
            .parse::<DocumentMut>()
            .map_err(std::io::Error::other)?;
        doc["package"]["metadata"]["rpm"]["package"]["license"] = value(license);

        fs::write("Cargo.toml", doc.to_string())?;
        ui::success("RPM metadata added to Cargo.toml.");
    }
    Ok(())
}

fn configure_wix() -> Result<()> {
    if !is_installed("cargo-wix") {
        return Ok(());
    }

    ui::info("Checking WiX (.msi) configuration...");
    if Path::new("wix").exists() {
        ui::success("WiX project already initialized.");
        return Ok(());
    }

    if ui::prompt_confirm(
        "Would you like to initialize the WiX project for Windows installers?",
        true,
    )? {
        ui::info("Executing: cargo wix init...");
        let status = Command::new("cargo").arg("wix").arg("init").status()?;
        if status.success() {
            ui::success("WiX project initialized successfully.");
        }
    }
    Ok(())
async fn setup_docker_environment() -> Result<()> {
    println!(
        "\n{}\n",
        console::Style::new()
            .cyan()
            .apply_to("--- Docker Infrastructure ---")
    );

    if !is_installed("docker") {
        ui::warn("Docker is not installed. Skipping Dockerfile generation.");
        return Ok(());
    }

    if Path::new("Dockerfile").exists() {
        ui::success("Dockerfile already exists.");
        if !ui::prompt_confirm(
            "Do you want to recreate it with Refinery optimization?",
            false,
        )? {
            return Ok(());
        }
    }

    ui::info("Analyzing project for Docker optimization...");

    let bin_name = get_binary_name_from_cargo();
    let debian_tag = fetch_latest_debian_tag()
        .await
        .unwrap_or_else(|_| "bookworm-slim".to_string());

    let selected_bin = ui::prompt_text("Binary to containerize:", &bin_name)?;

    let dockerfile_content = format!(
        r#"# Generated by Refinery-RS
FROM debian:{debian_tag}
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary from the artifacts directory (standard Refinery Release path)
COPY artifacts/{selected_bin}_linux-x86_64 /usr/local/bin/app

RUN chmod +x /usr/local/bin/app
ENTRYPOINT ["/usr/local/bin/app"]
"#
    );

    fs::write("Dockerfile", dockerfile_content)?;
    ui::success(&format!(
        "Intelligent Dockerfile generated using debian:{debian_tag}"
    ));
    Ok(())
}

fn get_binary_name_from_cargo() -> String {
    if let Ok(content) = fs::read_to_string("Cargo.toml")
        && let Ok(doc) = content.parse::<DocumentMut>()
        && let Some(name) = doc["package"]["name"].as_str()
    {
        return name.to_string();
    }
    "app".to_string()
}

async fn fetch_latest_debian_tag() -> Result<String> {
    let client = reqwest::Client::new();
    let res = client
        .get("https://hub.docker.com/v2/repositories/library/debian/tags/?page_size=10")
        .header("User-Agent", "refinery-rs")
        .send()
        .await?;

    if let Ok(data) = res.json::<DockerHubResponse>().await {
        return Ok(data
            .results
            .iter()
            .map(|t| &t.name)
            .find(|n| n.ends_with("-slim") && !n.contains("rc") && !n.contains("experimental"))
            .cloned()
            .unwrap_or_else(|| "bookworm-slim".to_string()));
    }
    Ok("bookworm-slim".to_string())
}

fn is_installed(name: &str) -> bool {
    Command::new(name).arg("--version").output().is_ok()
}
