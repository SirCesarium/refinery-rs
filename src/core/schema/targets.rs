use crate::core::schema::types::{Arch, LibC, OS};
use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Targets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linux: Option<LinuxTargets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows: Option<TargetMatrix>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macos: Option<TargetMatrix>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LinuxTargets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gnu: Option<TargetMatrix>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub musl: Option<TargetMatrix>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetMatrix {
    #[serde(default = "default_archs")]
    pub archs: Vec<Arch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pkg: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strip: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<String, NameOverride>>,
}

impl Default for TargetMatrix {
    fn default() -> Self {
        Self {
            archs: default_archs(),
            artifacts: vec![],
            pkg: vec![],
            ext: None,
            strip: false,
            overrides: None,
        }
    }
}

fn default_archs() -> Vec<Arch> {
    vec![Arch::X86_64]
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct NameOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_arch: Option<HashMap<Arch, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_pkg: Option<HashMap<String, String>>,
}

impl Targets {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.linux.is_none() && self.windows.is_none() && self.macos.is_none()
    }

    pub(crate) fn default_standard() -> Self {
        Self {
            linux: Some(LinuxTargets {
                gnu: Some(TargetMatrix::default_linux_gnu()),
                musl: None,
            }),
            windows: Some(TargetMatrix::default_windows()),
            macos: None,
        }
    }
}

impl TargetMatrix {
    /// Generates a list of official Rust target triples based on the matrix configuration.
    ///
    /// # Errors
    /// Returns `RefineryError` if configuration is invalid.
    pub fn get_triples(&self, os: OS, libc: Option<LibC>) -> Result<Vec<String>> {
        use crate::errors::RefineryError;
        let mut triples = Vec::new();
        for arch in &self.archs {
            let triple = match (os, libc, arch) {
                (OS::Linux, Some(LibC::Gnu), a) => {
                    format!("{}-unknown-linux-gnu", a.to_triple_part())
                }
                (OS::Linux, Some(LibC::Musl), a) => {
                    format!("{}-unknown-linux-musl", a.to_triple_part())
                }
                (OS::Linux, None, _) => {
                    return Err(RefineryError::Config(
                        "Linux target requires libc (gnu/musl)".into(),
                    ));
                }
                (OS::Windows, _, Arch::X86_64) => "x86_64-pc-windows-msvc".into(),
                (OS::Windows, _, Arch::I686) => "i686-pc-windows-msvc".into(),
                (OS::Windows, _, Arch::Aarch64) => "aarch64-pc-windows-msvc".into(),
                (OS::Macos, _, Arch::X86_64) => "x86_64-apple-darwin".into(),
                (OS::Macos, _, Arch::Aarch64) => "aarch64-apple-darwin".into(),
                (OS::Macos, _, Arch::I686) => {
                    return Err(RefineryError::Config(
                        "macOS does not support x32 (i686) architecture".into(),
                    ));
                }
            };
            triples.push(triple);
        }
        Ok(triples)
    }

    #[must_use]
    pub fn resolve_name(
        &self,
        artifact_name: &str,
        arch: Arch,
        os: OS,
        pkg_format: Option<&str>,
        is_library: bool,
    ) -> String {
        let base_name = self
            .overrides
            .as_ref()
            .and_then(|o| o.get(artifact_name))
            .map_or_else(
                || artifact_name.to_string(),
                |rule| {
                    if let Some(pkg) = pkg_format
                        && let Some(name) = rule.per_pkg.as_ref().and_then(|m| m.get(pkg))
                    {
                        name.clone()
                    } else if let Some(name) = rule.per_arch.as_ref().and_then(|m| m.get(&arch)) {
                        name.clone()
                    } else {
                        rule.out_name
                            .clone()
                            .unwrap_or_else(|| artifact_name.to_string())
                    }
                },
            );

        if !is_library {
            let extension = self
                .ext
                .as_deref()
                .unwrap_or_else(|| if os == OS::Windows { ".exe" } else { "" });

            if !extension.is_empty() && !base_name.ends_with(extension) {
                return format!("{base_name}{extension}");
            }
        }
        base_name
    }

    pub(crate) fn default_linux_gnu() -> Self {
        Self {
            artifacts: vec!["my-project".into()],
            pkg: vec!["deb".into()],
            ..Self::default()
        }
    }

    pub(crate) fn default_windows() -> Self {
        Self {
            artifacts: vec!["my-project".into()],
            pkg: vec!["msi".into()],
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::schema::types::{Arch, OS};

    #[test]
    fn test_target_matrix_defaults() {
        let matrix = TargetMatrix::default();
        assert_eq!(matrix.archs, vec![Arch::X86_64]);
        assert!(matrix.artifacts.is_empty());
        assert!(matrix.pkg.is_empty());
        assert!(matrix.ext.is_none());
        assert!(!matrix.strip);
    }

    #[test]
    fn test_resolve_name_windows_default_ext() {
        let matrix = TargetMatrix::default();
        let name = matrix.resolve_name("test", Arch::X86_64, OS::Windows, None, false);
        assert_eq!(name, "test.exe");
    }

    #[test]
    fn test_resolve_name_linux_no_ext() {
        let matrix = TargetMatrix::default();
        let name = matrix.resolve_name("test", Arch::X86_64, OS::Linux, None, false);
        assert_eq!(name, "test");
    }

    #[test]
    fn test_resolve_name_with_explicit_ext() {
        let matrix = TargetMatrix {
            ext: Some(".bin".to_string()),
            ..TargetMatrix::default()
        };
        let name = matrix.resolve_name("test", Arch::X86_64, OS::Windows, None, false);
        assert_eq!(name, "test.bin");
    }

    #[test]
    fn test_resolve_name_library_no_ext() {
        let matrix = TargetMatrix::default();
        let name = matrix.resolve_name("test", Arch::X86_64, OS::Windows, None, true);
        assert_eq!(name, "test");
    }
}
