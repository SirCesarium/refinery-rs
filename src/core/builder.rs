// @swt-disable max-lines
use crate::core::project;
use crate::core::schema::{LibC, OS, RefineryConfig, TargetMatrix};
use crate::errors::{RefineryError, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use toml_edit::DocumentMut;

/// Information about a build target.
pub struct TargetInfo {
    /// The target triple (e.g., x86_64-unknown-linux-gnu).
    pub triple: String,
    /// The target operating system.
    pub os: OS,
    /// The target C library (if applicable).
    pub libc: Option<LibC>,
    /// The packaging matrix for this target.
    pub matrix: TargetMatrix,
}

/// Manages the build process for different targets.
pub struct BuildManager<'a> {
    /// The project configuration.
    pub config: &'a RefineryConfig,
    /// Whether to build in release mode.
    pub release: bool,
}

impl<'a> BuildManager<'a> {
    /// Creates a new `BuildManager`.
    #[must_use]
    pub const fn new(config: &'a RefineryConfig, release: bool) -> Self {
        Self { config, release }
    }

    /// Builds all configured targets.
    ///
    /// # Errors
    /// Returns an error if any target fails to build or package.
    pub fn build_all(&self) -> Result<()> {
        let targets = self.collect_targets_info()?;
        for info in targets {
            self.build_target(&info)?;
        }
        Ok(())
    }

    /// Builds a single target specified by its triple.
    ///
    /// # Errors
    /// Returns an error if the target is not found or fails to build.
    pub fn build_single(&self, target_triple: &str) -> Result<()> {
        let targets = self.collect_targets_info()?;
        let info = targets
            .into_iter()
            .find(|t| t.triple == target_triple)
            .ok_or_else(|| anyhow::anyhow!("Target {target_triple} not found in configuration"))?;
        self.build_target(&info)
    }

    /// Generates C headers for libraries that have them enabled.
    ///
    /// # Errors
    /// Returns an error if `cbindgen` fails to generate headers.
    pub fn generate_headers(&self) -> Result<()> {
        for lib in &self.config.libraries {
            if lib.headers {
                let mut cmd = Command::new("cbindgen");
                let _ = cmd.arg("--output").arg(format!("{}.h", lib.name));
                let _ = cmd.arg(&lib.path);

                if Path::new("cbindgen.toml").exists() {
                    let _ = cmd.arg("--config").arg("cbindgen.toml");
                } else {
                    let _ = cmd.arg("--lang").arg("c");
                }

                let status = cmd.status()?;
                if !status.success() {
                    return Err(RefineryError::Generic(anyhow::anyhow!(
                        "Failed to generate headers for {}",
                        lib.name
                    )));
                }
            }
        }
        Ok(())
    }

    fn collect_targets_info(&self) -> Result<Vec<TargetInfo>> {
        let mut infos = Vec::new();

        if let Some(linux) = &self.config.targets.linux {
            if let Some(gnu) = &linux.gnu {
                for triple in gnu.get_triples(OS::Linux, Some(LibC::Gnu))? {
                    infos.push(TargetInfo {
                        triple,
                        os: OS::Linux,
                        libc: Some(LibC::Gnu),
                        matrix: gnu.clone(),
                    });
                }
            }
            if let Some(musl) = &linux.musl {
                for triple in musl.get_triples(OS::Linux, Some(LibC::Musl))? {
                    infos.push(TargetInfo {
                        triple,
                        os: OS::Linux,
                        libc: Some(LibC::Musl),
                        matrix: musl.clone(),
                    });
                }
            }
        }

        if let Some(windows) = &self.config.targets.windows {
            for triple in windows.get_triples(OS::Windows, None)? {
                infos.push(TargetInfo {
                    triple,
                    os: OS::Windows,
                    libc: None,
                    matrix: windows.clone(),
                });
            }
        }

        if let Some(macos) = &self.config.targets.macos {
            for triple in macos.get_triples(OS::Macos, None)? {
                infos.push(TargetInfo {
                    triple,
                    os: OS::Macos,
                    libc: None,
                    matrix: macos.clone(),
                });
            }
        }

        Ok(infos)
    }

    fn build_target(&self, info: &TargetInfo) -> Result<()> {
        project::setup_toolchain(&info.triple);

        // Use cross for any Linux cross-compilation (not native x86_64-gnu)
        let use_cross = info.triple.contains("linux")
            && (info.triple.contains("musl") || !info.triple.starts_with("x86_64"));

        let mut cmd = if use_cross {
            let mut c = Command::new("cross");
            let _ = c.arg("build");
            c
        } else {
            let mut c = Command::new("cargo");
            let _ = c.arg("build");
            c
        };

        let _ = cmd.arg("--target").arg(&info.triple);
        if self.release {
            let _ = cmd.arg("--release");
        }

        let status = cmd.status().map_err(RefineryError::Io)?;
        if !status.success() {
            return Err(RefineryError::Generic(anyhow::anyhow!(
                "Failed to build target: {}",
                info.triple
            )));
        }

        for pkg_format in &info.matrix.pkg {
            Self::validate_packaging_metadata(pkg_format)?;
            self.package_target(info, pkg_format)?;
        }

        Ok(())
    }

    fn validate_packaging_metadata(format: &str) -> Result<()> {
        let cargo_content = fs::read_to_string("Cargo.toml").map_err(RefineryError::Io)?;
        let cargo_toml = cargo_content
            .parse::<DocumentMut>()
            .map_err(RefineryError::Toml)?;

        let metadata_key = match format {
            "deb" => "deb",
            "rpm" => "generate-rpm",
            _ => return Ok(()),
        };

        if cargo_toml
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get(metadata_key))
            .is_none()
        {
            return Err(RefineryError::Generic(anyhow::anyhow!(
                "Missing [package.metadata.{metadata_key}] in Cargo.toml. Run 'refinery setup' first."
            )));
        }
        Ok(())
    }

    fn package_target(&self, info: &TargetInfo, format: &str) -> Result<()> {
        match format {
            "deb" => Self::run_cargo_deb(info),
            "msi" => Self::run_cargo_wix(info),
            "rpm" => Self::run_cargo_generate_rpm(info),
            "tar.gz" => self.run_archive_tar(info),
            "zip" => self.run_archive_zip(info),
            _ => Ok(()),
        }
    }

    fn run_cargo_deb(info: &TargetInfo) -> Result<()> {
        let mut cargo_toml_modified = false;
        let original_toml = fs::read_to_string("Cargo.toml").map_err(RefineryError::Io)?;

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

        let status = Command::new("cargo")
            .arg("deb")
            .arg("--target")
            .arg(&info.triple)
            .arg("--no-build")
            .arg("--no-strip")
            .status()
            .map_err(RefineryError::Io);

        if cargo_toml_modified {
            let _ = fs::write("Cargo.toml", original_toml);
        }
        let s = status?;
        if !s.success() {
            return Err(RefineryError::Generic(anyhow::anyhow!(
                "Failed to generate .deb for {}",
                info.triple
            )));
        }
        Ok(())
    }

    fn run_cargo_wix(info: &TargetInfo) -> Result<()> {
        let cargo_content = fs::read_to_string("Cargo.toml").map_err(RefineryError::Io)?;
        let cargo_toml = cargo_content
            .parse::<DocumentMut>()
            .map_err(RefineryError::Toml)?;

        let version_str = cargo_toml["package"]["version"].as_str().unwrap_or("0.1.0");

        let mut cmd = Command::new("cargo");
        cmd.arg("wix").arg("--target").arg(&info.triple);

        // Handle pre-release for WiX (e.g., 1.0.0-rc.1 -> 1.0.0-1) as WiX has strict versioning rules
        if version_str.contains("-rc.") {
            let wix_version = version_str.replace("-rc.", "-");
            cmd.arg("--package-version").arg(wix_version);
        }

        let status = cmd.status().map_err(RefineryError::Io)?;
        if !status.success() {
            return Err(RefineryError::Generic(anyhow::anyhow!(
                "Failed to generate .msi for {}",
                info.triple
            )));
        }
        Ok(())
    }

    fn run_cargo_generate_rpm(info: &TargetInfo) -> Result<()> {
        let status = Command::new("cargo")
            .arg("generate-rpm")
            .arg("--target")
            .arg(&info.triple)
            .status()
            .map_err(RefineryError::Io)?;
        if !status.success() {
            return Err(RefineryError::Generic(anyhow::anyhow!(
                "Failed to generate .rpm for {}",
                info.triple
            )));
        }
        Ok(())
    }

    fn run_archive_tar(&self, info: &TargetInfo) -> Result<()> {
        let profile = if self.release { "release" } else { "debug" };
        let base_path = format!("target/{}/{}", info.triple, profile);
        let archive_name = format!("target/{}/{}.tar.gz", info.triple, info.triple);
        let mut args = vec!["-czf", &archive_name, "-C", &base_path];
        for bin in &self.config.binaries {
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
                "Failed to create .tar.gz for {}",
                info.triple
            )));
        }
        Ok(())
    }

    fn run_archive_zip(&self, info: &TargetInfo) -> Result<()> {
        let profile = if self.release { "release" } else { "debug" };
        let base_path = format!("target/{}/{}", info.triple, profile);
        let archive_name = format!("{}.zip", info.triple);

        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("tar");
            c.arg("-a")
                .arg("-c")
                .arg("-f")
                .arg(format!("../../{archive_name}"));
            c
        } else {
            let mut c = Command::new("zip");
            c.arg("-j").arg(format!("../../{archive_name}"));
            c
        };

        for bin in &self.config.binaries {
            if info.matrix.artifacts.contains(&bin.name) {
                cmd.arg(if info.triple.contains("windows") {
                    format!("{}.exe", bin.name)
                } else {
                    bin.name.clone()
                });
            }
        }

        let status = cmd
            .current_dir(Path::new(&base_path))
            .status()
            .map_err(RefineryError::Io)?;
        if !status.success() {
            return Err(RefineryError::Generic(anyhow::anyhow!(
                "Failed to create .zip for {}",
                info.triple
            )));
        }
        let _ = fs::rename(
            format!("target/{archive_name}"),
            format!("target/{}/{archive_name}", info.triple),
        );
        Ok(())
    }
}
