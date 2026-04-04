#![allow(dead_code)]

use clap::ValueEnum;
use miette::{Result, miette};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
pub enum Os {
    Linux,
    Windows,
    Macos,
}

#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
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
pub struct RefineryConfig {
    pub project: Project,
    pub build: Build,
    pub publish: Option<Publish>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub r#type: ProjectType,
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Bin,
    Lib,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    pub targets: Vec<Target>,

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Publish {
    pub crates_io: Option<CratesIoConfig>,
    pub ghcr: Option<GhcrConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoConfig {
    pub enabled: bool,
    #[serde(default)]
    pub allow_dirty: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GhcrConfig {
    pub image_name: String,
    #[serde(default = "default_dockerfile")]
    pub dockerfile: String,
}

fn default_dockerfile() -> String {
    "./Dockerfile".to_string()
}

impl Target {
    pub fn to_triple(&self) -> Result<String> {
        let triple = match (self.os, self.arch) {
            // --- LINUX ---
            (Os::Linux, Arch::X64) => "x86_64-unknown-linux-musl",
            (Os::Linux, Arch::X86) => "i686-unknown-linux-musl",
            (Os::Linux, Arch::Arm64) => "aarch64-unknown-linux-musl",

            // --- WINDOWS ---
            (Os::Windows, Arch::X64) => "x86_64-pc-windows-msvc",
            (Os::Windows, Arch::X86) => "i686-pc-windows-msvc",
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
