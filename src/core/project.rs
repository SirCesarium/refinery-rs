use crate::core::schema::{
    LibType, Library, RefineryConfig, prepare_cargo_bins, prepare_cargo_lib,
};
use crate::core::workflow::Workflow;
use crate::errors::Result;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

/// Synchronizes project metadata between `refinery.toml` and `Cargo.toml`.
///
/// # Errors
/// Returns an error if reading or writing `Cargo.toml` fails.
pub fn sync_metadata(config: &RefineryConfig) -> Result<()> {
    let cargo_path = "Cargo.toml";
    let cargo_content = fs::read_to_string(cargo_path)?;
    let mut current_toml = cargo_content.clone();

    // Synchronize binaries
    current_toml = prepare_cargo_bins(&current_toml, &config.binaries)?;

    // Synchronize libraries
    for lib in &config.libraries {
        let crate_types: Vec<String> = lib
            .types
            .iter()
            .map(|t| match t {
                LibType::Dynamic => "cdylib".to_string(),
                LibType::Static => "staticlib".to_string(),
            })
            .collect();
        current_toml = prepare_cargo_lib(&current_toml, &lib.name, crate_types, lib.headers)?;
    }

    if current_toml != cargo_content {
        fs::write(cargo_path, current_toml)?;
    }

    Ok(())
}

/// Generates the GitHub Actions workflow for the project.
///
/// # Errors
/// Returns an error if serialization or file writing fails.
pub fn generate_workflow(config: &RefineryConfig) -> Result<()> {
    let workflow = Workflow::primary_workflow(config)?;
    let yaml = workflow.to_yaml()?;
    let path = Path::new(".github/workflows/refinery.yml");

    ensure_dir(".github/workflows")?;
    fs::write(path, yaml)?;
    Ok(())
}

/// Generates a Quality Gate workflow file.
///
/// # Errors
/// Returns an error if file writing fails.
pub fn generate_quality_gate(checks: &[String], clippy_flags: &str) -> Result<()> {
    let yaml = Workflow::quality_gate(checks, clippy_flags);
    let path = Path::new(".github/workflows/ci.yml");

    ensure_dir(".github/workflows")?;
    fs::write(path, yaml)?;
    Ok(())
}

/// Generates boilerplate for a library.
///
/// # Errors
/// Returns an error if file writing fails.
pub fn generate_lib_boilerplate(lib: &Library) -> Result<()> {
    let lib_path = Path::new(&lib.path);
    if !lib_path.exists() {
        if let Some(parent) = lib_path.parent() {
            ensure_dir(parent)?;
        }
        let boilerplate = r#"#[unsafe(no_mangle)]
pub extern "C" fn hello_refinery() {
    println!("Hello from Refinery-optimized library!");
}
"#;
        fs::write(lib_path, boilerplate)?;
    }
    Ok(())
}

/// Ensures the required toolchain and tools are installed.
pub fn setup_toolchain(target: &str) {
    let _ = Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg(target)
        .status();

    if target.contains("musl") && !check_command("cross") {
        let _ = Command::new("cargo").arg("install").arg("cross").status();
    }
}

/// Checks if a command is available in the system PATH.
#[must_use]
pub fn check_command(cmd: &str) -> bool {
    let search_cmd = if cfg!(windows) { "where" } else { "which" };
    Command::new(search_cmd)
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Ensures a directory exists.
///
/// # Errors
/// Returns an error if directory creation fails.
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}
