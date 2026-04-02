//! Logic for writing workflow files to the .github/workflows directory.

use std::fs;
use std::fmt::Write;
use std::path::Path;
use crate::errors::{RefineryError, Result};
use crate::models::{CiConfig, ReleaseConfig};
use crate::yaml_block;

/// A writer capable of generating and updating project configuration files.
pub struct WorkflowWriter {
    output_dir: String,
    force: bool,
}

impl WorkflowWriter {
    #[must_use]
    pub fn new(force: bool) -> Self {
        Self {
            output_dir: ".github/workflows".to_string(),
            force,
        }
    }

    pub fn ensure_dir(&self) -> Result<()> {
        if !Path::new(&self.output_dir).exists() {
            fs::create_dir_all(&self.output_dir)?;
        }
        Ok(())
    }

    pub fn check_conflict(&self, name: &str) -> Result<()> {
        if self.force { return Ok(()); }
        let path = format!("{}/{}.yml", self.output_dir, name);
        if Path::new(&path).exists() {
            return Err(RefineryError::FileExists(path));
        }
        Ok(())
    }

    pub fn write_ci(&self, config: &CiConfig) -> Result<String> {
        let path = format!("{}/ci.yml", self.output_dir);
        let content = yaml_block!(r"name: CI Validation

on:
  push:
    branches: [ main, master ]
  pull_request:
    branches: [ main, master ]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6.0.2
      - name: Refinery CI
        uses: sircesarium/refinery-rs/ci@main
        with:
          enable-sweet: <enable_sweet>
          enable-clippy: <enable_clippy>
          enable-fmt: <enable_fmt>
", 
            enable_sweet = config.enable_sweet,
            enable_clippy = config.enable_clippy,
            enable_fmt = config.enable_fmt
        );
        self.write_file(&path, &content)?;
        Ok(path)
    }

    pub fn write_release(&self, config: &ReleaseConfig) -> Result<String> {
        let path = format!("{}/release.yml", self.output_dir);
        
        let mut matrix_include = String::new();
        for bin in &config.binaries {
            for target in &bin.targets {
                let os = if target.contains("windows") {
                    "windows-latest"
                } else if target.contains("apple") {
                    "macos-latest"
                } else {
                    "ubuntu-latest"
                };
                
                let can_docker = config.features.publish_docker && target.contains("linux") && target.contains("x86_64") && bin.name == "swt";
                let use_cross = target.contains("musl") || target.contains("i686") || target.contains("aarch64-unknown-linux-gnu");
                
                let _ = writeln!(
                    matrix_include, 
                    "          - target: {target}\n            os: {os}\n            bin: {bin_name}\n            features: \"{features}\"\n            export_libs: {export_libs}\n            package: {package}\n            docker: {can_docker}\n            use_cross: {use_cross}",
                    bin_name = bin.name,
                    features = bin.features,
                    export_libs = bin.export_libs,
                    package = bin.enable_packaging
                );
            }
        }

        let mut content = yaml_block!(r"name: Release & Export

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    name: Build (${{ matrix.bin }} - ${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
<matrix_include>
    steps:
      - uses: actions/checkout@v6.0.2
      - name: Refinery Build
        uses: sircesarium/refinery-rs@main
        with:
          target: ${{ matrix.target }}
          binary-name: ${{ matrix.bin }}
          features: ${{ matrix.features }}
          export-libs: ${{ matrix.export_libs }}
          package: ${{ matrix.package }}
          publish-docker: ${{ matrix.docker }}
          use-cross: ${{ matrix.use_cross }}
          github-token: ${{ secrets.GITHUB_TOKEN }}
",
            matrix_include = matrix_include.trim_end(),
        );

        if config.features.publish_crates {
            content.push_str(r"
  publish-crates:
    name: Publish to Crates.io
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/checkout@v6.0.2
      - name: Publish
        run: cargo publish --token ${{ secrets.CRATES_IO_TOKEN }}
");
        }

        self.write_file(&path, &content)?;
        Ok(path)
    }

    fn write_file(&self, path: &str, content: &str) -> Result<()> {
        if Path::new(path).exists() && !self.force {
            return Err(RefineryError::FileExists(path.to_string()));
        }
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BinaryConfig, ReleaseFeatures};

    #[test]
    fn test_ci_config_generation() -> Result<()> {
        let writer = WorkflowWriter::new(true);
        let config = CiConfig {
            enable_sweet: true,
            enable_clippy: false,
            enable_fmt: true,
        };
        
        let path = writer.write_ci(&config)?;
        assert!(Path::new(&path).exists());
        
        let content = fs::read_to_string(path)?;
        assert!(content.contains("enable-sweet: true"));
        assert!(content.contains("enable-clippy: false"));
        Ok(())
    }

    #[test]
    fn test_release_config_matrix() -> Result<()> {
        let writer = WorkflowWriter::new(true);
        let config = ReleaseConfig {
            binaries: vec![BinaryConfig {
                name: "test-app".to_string(),
                targets: vec!["x86_64-unknown-linux-gnu".to_string()],
                ..Default::default()
            }],
            features: ReleaseFeatures::default(),
        };
        
        let path = writer.write_release(&config)?;
        let content = fs::read_to_string(path)?;
        assert!(content.contains("bin: test-app"));
        assert!(content.contains("target: x86_64-unknown-linux-gnu"));
        assert!(content.contains("${{ matrix.bin }}"));
        Ok(())
    }
}
