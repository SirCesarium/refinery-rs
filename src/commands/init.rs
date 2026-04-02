//! Logic for the 'init' command.

use crate::errors::Result;
use crate::models::{BinaryConfig, CiConfig, ReleaseConfig, ReleaseFeatures};
use crate::ui;
use crate::writer::WorkflowWriter;
use console::Style;
use inquire::{MultiSelect, Select};

/// Orchestrates the workflow initialization process.
pub fn run(force: bool) -> Result<()> {
    ui::print_banner();

    let writer = WorkflowWriter::new(force);
    writer.ensure_dir()?;

    let workflow_type = Select::new(
        "What type of workflow would you like to generate?",
        vec!["CI (Validation)", "Release (Build & Export)"],
    )
    .prompt()?;

    match workflow_type {
        "CI (Validation)" => init_ci_workflow(&writer),
        "Release (Build & Export)" => init_release_workflow(&writer),
        _ => unreachable!(),
    }
}

fn init_ci_workflow(writer: &WorkflowWriter) -> Result<()> {
    println!("{}\n", Style::new().cyan().apply_to("--- CI Configuration ---"));
    
    println!("{}\n{}\n{}\n{}\n", 
        Style::new().yellow().apply_to("Quality Gate Tools:"),
        Style::new().dim().apply_to("- Sweet (swt): Maintainability & Architecture analyzer (Bloat, Nesting, DRY)."),
        Style::new().dim().apply_to("- Clippy: Standard Rust lints (Common errors & idiomatic code)."),
        Style::new().dim().apply_to("- Fmt: Ensures consistent project-wide formatting.")
    );

    let enable_sweet = ui::prompt_confirm("Enable Sweet (swt) analysis?", true)?;
    let enable_clippy = ui::prompt_confirm("Enable Clippy lints?", true)?;
    let enable_fmt = ui::prompt_confirm("Enable Formatting check?", true)?;

    let config = CiConfig {
        enable_sweet,
        enable_clippy,
        enable_fmt,
    };

    let path = writer.write_ci(&config)?;
    ui::success(&format!("Generated CI workflow at {}", Style::new().bold().apply_to(path)));
    Ok(())
}

fn init_release_workflow(writer: &WorkflowWriter) -> Result<()> {
    println!("{}\n", Style::new().cyan().apply_to("--- Binary Configuration ---"));
    let mut binaries = Vec::new();
    
    loop {
        let bin_name = ui::prompt_text("Binary name:", "app")?;
        let features = ui::prompt_text(&format!("Features for {bin_name} (comma-separated, leave empty for default, or 'all-features'):"), "")?;

        println!("\n{}\n{}\n", 
            Style::new().yellow().apply_to("Quick Guide:"),
            Style::new().dim().apply_to("- Linux (GNU vs MUSL): GNU is standard, MUSL is static/portable (Docker/Alpine).\n- Legacy (32-bit): Support for very old PCs.\n- ARM: Modern efficiency (M1/M2/M3 Mac, Surface Pro X, Linux ARM).")
        );

        let targets = MultiSelect::new(
            &format!("Select targets for {bin_name}:"),
            vec![
                "x86_64-unknown-linux-gnu (Linux 64-bit, standard)",
                "x86_64-unknown-linux-musl (Linux 64-bit, static/portable)",
                "i686-unknown-linux-gnu (Linux 32-bit)",
                "aarch64-unknown-linux-gnu (Linux ARM64)",
                "x86_64-pc-windows-msvc (Windows 64-bit, modern)",
                "i686-pc-windows-msvc (Windows 32-bit, legacy/older PCs)",
                "aarch64-pc-windows-msvc (Windows ARM64, Surface/modern tablets)",
                "x86_64-apple-darwin (macOS Intel 64-bit)",
                "aarch64-apple-darwin (macOS Apple Silicon/ARM)",
            ],
        )
        .prompt()?;

        let clean_targets = targets.into_iter()
            .map(|t| t.split(' ').next().unwrap_or(t).to_string())
            .collect();

        let export_libs = ui::prompt_confirm(&format!("Export library files (.so, .dll, .dylib, .a) for {bin_name}?"), true)?;
        let enable_packaging = ui::prompt_confirm(&format!("Generate installers (.deb, .rpm, .msi) for {bin_name}?"), false)?;

        binaries.push(BinaryConfig {
            name: bin_name,
            features,
            targets: clean_targets,
            export_libs,
            enable_packaging,
        });

        if !ui::prompt_confirm("Add another binary?", false)? {
            break;
        }
    }

    println!("\n{}\n", Style::new().cyan().apply_to("--- Global Pipeline Configuration ---"));

    let publish_docker = ui::prompt_confirm("Would you like to publish a Docker image to GHCR?", false)?;
    let publish_crates = ui::prompt_confirm("Do you want to publish to crates.io?", false)?;

    let config = ReleaseConfig {
        binaries,
        features: ReleaseFeatures {
            publish_docker,
            publish_crates,
        },
    };

    let path = writer.write_release(&config)?;
    ui::success(&format!("Generated Release workflow at {}", Style::new().bold().apply_to(path)));
    Ok(())
}
