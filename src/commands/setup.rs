use anyhow::Result;
use clap::Args;
use refinery_rs::core::schema::{LibType, RefineryConfig, prepare_cargo_lib};
use refinery_rs::core::workflow::{Workflow, actions};
use refinery_rs::ui::prompts::{configure_libraries, install, installers};
use refinery_rs::ui::{icons, prompt_confirm, success, warn};
use refinery_rs::{log_step, prompt, prompt_multi};
use std::fs;
use std::path::Path;

#[derive(Args, Debug)]
pub struct SetupArgs {
    #[arg(short, long)]
    pub force: bool,
}

pub fn run(args: &SetupArgs) -> Result<()> {
    let mut config = RefineryConfig::load("refinery.toml")?;

    let options = vec![
        "Pipeline (refinery.yml)".to_string(),
        "Quality Gate (ci.yml)".to_string(),
        "Installers (WiX, deb, rpm)".to_string(),
        "Library Setup".to_string(),
    ];
    let selections: Vec<String> = prompt_multi!("What would you like to setup?", options)?;

    for selection in selections {
        match selection.as_str() {
            "Pipeline (refinery.yml)" => setup_pipeline(&config)?,
            "Quality Gate (ci.yml)" => setup_quality_gate()?,
            "Installers (WiX, deb, rpm)" => installers::setup_installers(&config, args.force)?,
            "Library Setup" => setup_lib(&mut config)?,
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn setup_pipeline(config: &RefineryConfig) -> Result<()> {
    log_step!(
        icons::TICK,
        Green,
        "Configuring unified refinery pipeline..."
    );
    let workflow = Workflow::primary_workflow(config)?;
    let yaml = workflow.to_yaml()?;

    let path = Path::new(".github/workflows/refinery.yml");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, yaml)?;
    success("Unified pipeline generated at .github/workflows/refinery.yml");
    Ok(())
}

fn setup_quality_gate() -> Result<()> {
    println!("\n{} CI Quality Gate Setup", icons::SETUP);
    println!("  - Sweet: Maintainability analysis (nesting, length, duplication)");
    println!("  - Format: Ensures code follows rustfmt standards");
    println!("  - Clippy: Lints for common mistakes and idiomatic improvements");
    println!("  - Tests: Executes your full test suite\n");

    let options = vec![
        "Sweet (Maintainability)".into(),
        "Format (rustfmt)".into(),
        "Clippy (Lints)".into(),
        "Tests (cargo test)".into(),
    ];
    let checks: Vec<String> = prompt_multi!("Select checks to include in ci.yml:", options)?;

    if checks.is_empty() {
        warn("No checks selected. Quality Gate skipped.");
        return Ok(());
    }

    let mut steps = vec![
        "      - uses: actions/checkout@v6".to_string(),
        "      - uses: dtolnay/rust-toolchain@stable\n        with:\n          components: clippy, rustfmt".to_string(),
        "      - uses: Swatinem/rust-cache@v2".to_string(),
    ];

    if checks.iter().any(|c| c.contains("Sweet")) {
        steps.push(format!(
            "      - name: Sweet Analysis\n        run: curl -L {}/releases/download/{}/{} -o swt && chmod +x swt && ./swt",
            actions::SWEET_REPO,
            actions::SWEET_DEFAULT_VERSION,
            actions::SWEET_BINARY
        ));
    }

    if checks.iter().any(|c| c.contains("Format")) {
        steps.push("      - name: Check Format\n        run: cargo fmt --check".to_string());
    }

    if checks.iter().any(|c| c.contains("Clippy")) {
        let flags: String = prompt!("Clippy flags (default: -- -D warnings):")?;
        let flags = if flags.trim().is_empty() {
            "-- -D warnings"
        } else {
            flags.trim()
        };
        steps.push(format!(
            "      - name: Clippy\n        run: cargo clippy {flags}"
        ));
    }

    if checks.iter().any(|c| c.contains("Tests")) {
        steps.push("      - name: Run Tests\n        run: cargo test".to_string());
    }

    let yaml = format!(
        "name: Quality Gate
on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]
jobs:
  check:
    name: Quality & Testing
    runs-on: ubuntu-latest
    steps:
{}
",
        steps.join("\n")
    );

    let path = Path::new(".github/workflows/ci.yml");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, yaml)?;
    success("Custom Quality Gate generated at .github/workflows/ci.yml");
    Ok(())
}

fn setup_lib(config: &mut RefineryConfig) -> Result<()> {
    if config.libraries.is_empty() {
        warn("No libraries defined in refinery.toml.");
        if prompt_confirm("Add a library configuration now?", true)? {
            let default_name = RefineryConfig::get_default_project_name();
            configure_libraries(config, &default_name)?;
            config.save("refinery.toml")?;
        } else {
            return Ok(());
        }
    }

    for lib in &config.libraries {
        log_step!(icons::LIB, Yellow, "Configuring library: {}...", lib.name);

        let cargo_content = fs::read_to_string("Cargo.toml")?;
        let crate_types: Vec<String> = lib
            .types
            .iter()
            .map(|t| match t {
                LibType::Dynamic => "cdylib".to_string(),
                LibType::Static => "staticlib".to_string(),
            })
            .collect();

        let updated_cargo = prepare_cargo_lib(&cargo_content, crate_types, lib.headers)?;
        fs::write("Cargo.toml", updated_cargo)?;
        success("Cargo.toml configured for library export.");

        let lib_path = Path::new(&lib.path);
        if !lib_path.exists() {
            if let Some(parent) = lib_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let boilerplate = r#"#[no_mangle]
pub extern "C" fn hello_refinery() {
    println!("Hello from Refinery-optimized library!");
}
"#;
            fs::write(lib_path, boilerplate)?;
            success(&format!("Boilerplate created at {}", lib.path));
        }

        if lib.headers {
            install::check_and_install("cbindgen", "cbindgen")?;
            success("cbindgen ready for header generation.");
        }
    }

    Ok(())
}
