use clap::Args;
use refinery_rs::core::schema::{
    LibC, LibType, OS, RefineryConfig, TargetMatrix, prepare_cargo_lib,
};
use refinery_rs::errors::{RefineryError, Result};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use toml_edit::DocumentMut;

#[derive(Args, Debug)]
pub struct BuildArgs {
    #[arg(short, long)]
    pub target: Option<String>,
    #[arg(long)]
    pub release: bool,
    #[arg(long)]
    pub headers_only: bool,
}

pub struct TargetInfo {
    pub triple: String,
    pub _os: OS,
    pub _libc: Option<LibC>,
    pub matrix: TargetMatrix,
}

pub fn run(args: &BuildArgs) -> anyhow::Result<()> {
    let config = RefineryConfig::load("refinery.toml")?;

    // Auto-sync Cargo.toml metadata to avoid collisions and ensure environment is ready
    sync_project_metadata(&config)?;

    if args.headers_only {
        return generate_headers(&config);
    }

    let targets = collect_targets_info(&config)?;

    if let Some(target_triple) = &args.target {
        let info = targets
            .into_iter()
            .find(|t| t.triple == *target_triple)
            .ok_or_else(|| anyhow::anyhow!("Target {target_triple} not found in configuration"))?;
        build_target(&config, &info, args.release)?;
    } else {
        for info in targets {
            build_target(&config, &info, args.release)?;
        }
    }

    Ok(())
}

fn sync_project_metadata(config: &RefineryConfig) -> anyhow::Result<()> {
    let cargo_content = fs::read_to_string("Cargo.toml")?;
    let mut current_toml = cargo_content;

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

    if current_toml != fs::read_to_string("Cargo.toml")? {
        fs::write("Cargo.toml", current_toml)?;
    }

    Ok(())
}

fn generate_headers(config: &RefineryConfig) -> anyhow::Result<()> {
    for lib in &config.libraries {
        if lib.headers {
            let mut cmd = Command::new("cbindgen");
            let _ = cmd.arg("--output").arg(format!("{}.h", lib.name));
            let _ = cmd.arg(&lib.path);

            // Only add config if it exists, otherwise use defaults to avoid cbindgen panic
            if Path::new("cbindgen.toml").exists() {
                let _ = cmd.arg("--config").arg("cbindgen.toml");
            } else {
                let _ = cmd.arg("--lang").arg("c");
            }

            let status = cmd.status()?;
            if !status.success() {
                anyhow::bail!("Failed to generate headers for {}", lib.name);
            }
            println!("Headers generated: {}.h", lib.name);
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

fn build_target(config: &RefineryConfig, info: &TargetInfo, release: bool) -> Result<()> {
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
        package_target(config, info, pkg_format, release)?;
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

fn package_target(
    config: &RefineryConfig,
    info: &TargetInfo,
    format: &str,
    release: bool,
) -> Result<()> {
    match format {
        "deb" => run_cargo_deb(info),
        "msi" => run_cargo_wix(info),
        "rpm" => run_cargo_generate_rpm(info),
        "tar.gz" => run_archive_tar(config, info, release),
        "zip" => run_archive_zip(config, info, release),
        _ => Ok(()),
    }
}

fn run_cargo_deb(info: &TargetInfo) -> Result<()> {
    let mut cargo_toml_modified = false;
    let original_toml = fs::read_to_string("Cargo.toml").map_err(RefineryError::Io)?;

    // Bypass for musl targets missing cdylib
    if info.triple.contains("musl") {
        let profile = "release";
        let mut lib_found = false;
        if let Ok(entries) = fs::read_dir(format!("target/{}/{}", info.triple, profile)) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str()
                    && Path::new(name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("so"))
                {
                    lib_found = true;
                    break;
                }
            }
        }

        if !lib_found {
            // Temporarily rename [lib] to avoid cargo-deb error
            let mut doc = original_toml
                .parse::<DocumentMut>()
                .map_err(RefineryError::Toml)?;
            if let Some(lib) = doc.remove("lib") {
                doc.insert("lib-disabled", lib);
                fs::write("Cargo.toml", doc.to_string()).map_err(RefineryError::Io)?;
                cargo_toml_modified = true;
            }
        }
    }

    let mut cmd = Command::new("cargo");
    let _ = cmd.arg("deb");
    let _ = cmd.arg("--target").arg(&info.triple);
    let _ = cmd.arg("--no-build");
    let _ = cmd.arg("--no-strip");

    let status = cmd.status().map_err(RefineryError::Io);

    // Restore Cargo.toml immediately
    if cargo_toml_modified {
        let _ = fs::write("Cargo.toml", original_toml);
    }

    let s = status?;
    if !s.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to generate .deb for target: {}",
            info.triple
        )));
    }
    Ok(())
}

fn run_cargo_wix(info: &TargetInfo) -> Result<()> {
    let mut cmd = Command::new("cargo");
    let _ = cmd.arg("wix").arg("--target").arg(&info.triple);

    let status = cmd.status().map_err(RefineryError::Io)?;
    if !status.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to generate .msi for target: {}",
            info.triple
        )));
    }
    Ok(())
}

fn run_cargo_generate_rpm(info: &TargetInfo) -> Result<()> {
    let mut cmd = Command::new("cargo");
    let _ = cmd.arg("generate-rpm").arg("--target").arg(&info.triple);
    let status = cmd.status().map_err(RefineryError::Io)?;
    if !status.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to generate .rpm for target: {}",
            info.triple
        )));
    }
    Ok(())
}

fn run_archive_tar(config: &RefineryConfig, info: &TargetInfo, release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    let base_path = format!("target/{}/{}", info.triple, profile);
    let archive_name = format!("target/{}/{}.tar.gz", info.triple, info.triple);

    let mut args = vec!["-czf", &archive_name, "-C", &base_path];

    for bin in &config.binaries {
        if info.matrix.artifacts.contains(&bin.name) {
            args.push(&bin.name);
        }
    }

    let status = Command::new("tar")
        .args(&args)
        .status()
        .map_err(RefineryError::Io)?;

    if !status.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to create .tar.gz for target: {}",
            info.triple
        )));
    }
    Ok(())
}

fn run_archive_zip(config: &RefineryConfig, info: &TargetInfo, release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    let base_path = format!("target/{}/{}", info.triple, profile);
    let archive_name = format!("{}.zip", info.triple);

    let mut cmd = Command::new("zip");
    let _ = cmd.arg("-j").arg(format!("../../{archive_name}"));

    for bin in &config.binaries {
        if info.matrix.artifacts.contains(&bin.name) {
            let name = if info.triple.contains("windows") {
                format!("{}.exe", bin.name)
            } else {
                bin.name.clone()
            };
            let _ = cmd.arg(name);
        }
    }

    let status = cmd
        .current_dir(Path::new(&base_path))
        .status()
        .map_err(RefineryError::Io)?;

    if !status.success() {
        return Err(RefineryError::Generic(anyhow::anyhow!(
            "Failed to create .zip for target: {}",
            info.triple
        )));
    }

    let _ = fs::rename(
        format!("target/{archive_name}"),
        format!("target/{}/{archive_name}", info.triple),
    );

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
