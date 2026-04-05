#![allow(dead_code)]
// @swt-disable max-depth

use clap::ValueEnum;
use miette::{Result, miette};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use toml_edit::{de, ser::to_string};

#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Linux,
    Windows,
    Macos,
}

#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    #[value(name = "x64")]
    X64,
    #[value(name = "x86")]
    X86,
    #[value(name = "arm64")]
    Arm64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub os: Os,
    pub arch: Arch,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub targets: Vec<Target>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default = "default_true")]
    pub default_features: bool,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefineryConfig {
    pub build: Build,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    pub artifacts: Vec<Artifact>,

    #[serde(default)]
    pub library: LibraryFormats,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LibraryFormats {
    #[serde(default)]
    pub dynamic: bool, // .so, .dll, .dylib
    #[serde(default)]
    pub static_lib: bool, // .a, .lib
    #[serde(default)]
    pub headers: bool,
}

impl Target {
    pub fn to_triple(&self) -> Result<String> {
        let triple = match (self.os, self.arch) {
            // --- LINUX ---
            (Os::Linux, Arch::X64) => "x86_64-unknown-linux-musl",
            (Os::Linux, Arch::X86) => "i686-unknown-linux-musl",
            (Os::Linux, Arch::Arm64) => "aarch64-unknown-linux-musl",

            // --- WINDOWS ---
            (Os::Windows, Arch::X64) => "x86_64-pc-windows-gnu",
            (Os::Windows, Arch::X86) => "i686-pc-windows-gnu",
            (Os::Windows, Arch::Arm64) => "aarch64-pc-windows-msvc",

            // --- MACOS ---
            (Os::Macos, Arch::Arm64) => "aarch64-apple-darwin", // Apple Silicon
            (Os::Macos, Arch::X64) => "x86_64-apple-darwin",    // Intel Mac

            // --- Invariants / Unsupported ---
            (Os::Macos, Arch::X86) => {
                return Err(miette!("macOS doesn't support x86 architecture."));
            }
        };

        Ok(triple.to_string())
    }
}

impl RefineryConfig {
    pub async fn init() -> Result<()> {
        let path = Path::new("refinery.toml");

        if path.exists() {
            return Err(miette!("refinery.toml already exists."));
        }

        let config = Self::default();

        let toml_string =
            to_string(&config).map_err(|e| miette!("Failed to serialize default config: {}", e))?;

        fs::write(path, toml_string)
            .await
            .map_err(|e| miette!("Failed to create refinery.toml: {}", e))?;

        Ok(())
    }

    pub async fn load() -> Result<Self> {
        let path = Path::new("refinery.toml");
        if !path.exists() {
            return Err(miette!(
                "refinery.toml not found. Run 'refinery init' first."
            ));
        }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| miette!("Failed to read refinery.toml: {}", e))?;

        let config: Self =
            de::from_str(&content).map_err(|e| miette!("Invalid refinery.toml format: {}", e))?;

        Ok(config)
    }
}

impl Default for RefineryConfig {
    fn default() -> Self {
        Self {
            build: Build {
                artifacts: vec![Artifact {
                    name: "my-project".to_string(),
                    targets: vec![
                        Target {
                            os: Os::Linux,
                            arch: Arch::X64,
                        },
                        Target {
                            os: Os::Windows,
                            arch: Arch::X64,
                        },
                    ],
                    features: vec![],
                    default_features: true,
                }],
                library: LibraryFormats::default(),
            },
        }
    }
}
