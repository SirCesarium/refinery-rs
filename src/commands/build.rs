use clap::Args;
use refinery_rs::core::schema::{LibC, OS, RefineryConfig, TargetMatrix};
use refinery_rs::errors::{RefineryError, Result};
use std::fs;
use std::process::{Command, Stdio};
use toml_edit::DocumentMut;

#[derive(Args, Debug)]
pub struct BuildArgs {
    #[arg(short, long)]
    pub target: Option<String>,
    #[arg(long)]
    pub release: bool,
}

pub struct TargetInfo {
    pub triple: String,
    pub _os: OS,
    pub _libc: Option<LibC>,
    pub matrix: TargetMatrix,
}

pub fn run(args: &BuildArgs) -> anyhow::Result<()> {
    let config = RefineryConfig::load("refinery.toml")?;

    let targets = collect_targets_info(&config)?;

    if let Some(target_triple) = &args.target {
        let info = targets
            .into_iter()
            .find(|t| t.triple == *target_triple)
            .ok_or_else(|| anyhow::anyhow!("Target {target_triple} not found in configuration"))?;
        build_target(&info, args.release)?;
    } else {
        for info in targets {
            build_target(&info, args.release)?;
        }
    }

    Ok(())
}

fn collect_targets_info(config: &RefineryConfig) -> Result<Vec<TargetInfo>> {
    let mut infos = Vec::new();

    if let Some(linux) = &config.targets.linux {
        if let Some(gnu) = &linux.gnu {
            for triple in gnu.get_triples(OS::Linux, Some(LibC::Gnu))? {
                infos.push(TargetInfo {
                    triple,
                    _os: OS::Linux,
                    _libc: Some(LibC::Gnu),
                    matrix: gnu.clone(),
                });
            }
        }
        if let Some(musl) = &linux.musl {
            for triple in musl.get_triples(OS::Linux, Some(LibC::Musl))? {
                infos.push(TargetInfo {
                    triple,
                    _os: OS::Linux,
                    _libc: Some(LibC::Musl),
                    matrix: musl.clone(),
                });
            }
        }
    }

    if let Some(windows) = &config.targets.windows {
        for triple in windows.get_triples(OS::Windows, None)? {
            infos.push(TargetInfo {
                triple,
                _os: OS::Windows,
                _libc: None,
                matrix: windows.clone(),
            });
        }
    }

    if let Some(macos) = &config.targets.macos {
        for triple in macos.get_triples(OS::Macos, None)? {
            infos.push(TargetInfo {
                triple,
                _os: OS::Macos,
                _libc: None,
                matrix: macos.clone(),
            });
        }
    }

    Ok(infos)
}

fn build_target(info: &TargetInfo, release: bool) -> Result<()> {
    setup_toolchain(&info.triple)?;

    let mut cmd = if info.triple.contains("musl") {
        let mut c = Command::new("cross");
        let _ = c.arg("build");
        c
    } else {
        let mut c = Command::new("cargo");
        let _ = c.arg("build");
        c
    };

    let _ = cmd.arg("--target").arg(&info.triple);

    if release {
        let _ = cmd.arg("--release");
    }

    let status = cmd.status().map_err(RefineryError::Io)?;

    if !status.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to build for target: {}",
            info.triple
        )));
    }

    // Process packaging with validation
    for pkg_format in &info.matrix.pkg {
        validate_packaging_metadata(pkg_format)?;
        package_target(info, pkg_format, release)?;
    }

    Ok(())
}

fn validate_packaging_metadata(format: &str) -> Result<()> {
    let cargo_content = fs::read_to_string("Cargo.toml").map_err(RefineryError::Io)?;
    let cargo_toml = cargo_content
        .parse::<DocumentMut>()
        .map_err(RefineryError::Toml)?;

    match format {
        "deb" => {
            if cargo_toml
                .get("package")
                .and_then(|p| p.get("metadata"))
                .and_then(|m| m.get("deb"))
                .is_none()
            {
                return Err(RefineryError::Generic(anyhow::anyhow!(
                    "Missing [package.metadata.deb] in Cargo.toml. Run 'refinery setup' first."
                )));
            }
        }
        "rpm" => {
            if cargo_toml
                .get("package")
                .and_then(|p| p.get("metadata"))
                .and_then(|m| m.get("generate-rpm"))
                .is_none()
            {
                return Err(RefineryError::Generic(anyhow::anyhow!(
                    "Missing [package.metadata.generate-rpm] in Cargo.toml. Run 'refinery setup' first."
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

fn package_target(info: &TargetInfo, format: &str, release: bool) -> Result<()> {
    match format {
        "deb" => run_cargo_deb(info, release),
        "msi" => run_cargo_wix(info, release),
        "rpm" => run_cargo_generate_rpm(info, release),
        _ => Ok(()),
    }
}

fn run_cargo_deb(info: &TargetInfo, release: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    let _ = cmd.arg("deb");
    let _ = cmd.arg("--target").arg(&info.triple);

    let _ = cmd.arg("--no-build");
    let _ = cmd.arg("--no-strip");

    let status = cmd.status().map_err(RefineryError::Io)?;
    if !status.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to generate .deb for target: {}",
            info.triple
        )));
    }
    let _ = release;
    Ok(())
}

fn run_cargo_wix(info: &TargetInfo, release: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    let _ = cmd.arg("wix").arg("--target").arg(&info.triple);

    let status = cmd.status().map_err(RefineryError::Io)?;
    if !status.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to generate .msi for target: {}",
            info.triple
        )));
    }
    let _ = release;
    Ok(())
}

fn run_cargo_generate_rpm(info: &TargetInfo, release: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    let _ = cmd.arg("generate-rpm").arg("--target").arg(&info.triple);
    let status = cmd.status().map_err(RefineryError::Io)?;
    if !status.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to generate .rpm for target: {}",
            info.triple
        )));
    }
    let _ = release;
    Ok(())
}

fn setup_toolchain(target: &str) -> Result<()> {
    let status = Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg(target)
        .status()
        .map_err(RefineryError::Io)?;

    if !status.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to add rustup target: {target}"
        )));
    }

    if target.contains("musl") && !check_command("cross") {
        let install_status = Command::new("cargo")
            .arg("install")
            .arg("cross")
            .status()
            .map_err(RefineryError::Io)?;

        if !install_status.success() {
            return Err(RefineryError::Generic(anyhow::anyhow!(
                "Failed to install cross for musl build"
            )));
        }
    }

    Ok(())
}

fn check_command(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
