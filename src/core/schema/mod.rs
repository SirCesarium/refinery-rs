pub mod artifacts;
pub mod metadata;
pub mod targets;
pub mod types;

pub use artifacts::*;
pub use metadata::*;
pub use targets::*;
pub use types::*;

use crate::errors::{RefineryError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{env, fs, path::Path};
use toml_edit::DocumentMut;
use toml_edit::de;
use toml_edit::ser::to_string_pretty;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefineryConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binaries: Vec<Binary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub libraries: Vec<Library>,
    pub targets: Targets,
}

impl RefineryConfig {
    /// Validates the configuration for consistency and correctness.
    ///
    /// # Errors
    /// Returns `RefineryError` if:
    /// - No binaries or libraries are defined.
    /// - Duplicate names are found.
    /// - Target triples are invalid.
    pub fn validate(&self) -> Result<()> {
        if self.binaries.is_empty() && self.libraries.is_empty() {
            return Err(RefineryError::Config(
                "At least one binary or library must be defined".into(),
            ));
        }

        let mut names = HashSet::new();
        for bin in &self.binaries {
            if !names.insert(&bin.name) {
                return Err(RefineryError::Config(format!(
                    "Duplicate artifact name found: {}",
                    bin.name
                )));
            }
        }
        for lib in &self.libraries {
            if !names.insert(&lib.name) {
                return Err(RefineryError::Config(format!(
                    "Duplicate artifact name found: {}",
                    lib.name
                )));
            }
        }

        if let Some(ref l) = self.targets.linux {
            if let Some(ref g) = l.gnu {
                g.get_triples(OS::Linux, Some(LibC::Gnu))?;
            }
            if let Some(ref m) = l.musl {
                m.get_triples(OS::Linux, Some(LibC::Musl))?;
            }
        }
        if let Some(ref w) = self.targets.windows {
            w.get_triples(OS::Windows, None)?;
        }
        if let Some(ref m) = self.targets.macos {
            m.get_triples(OS::Macos, None)?;
        }

        Ok(())
    }

    #[must_use]
    pub fn is_name_taken(&self, name: &str) -> bool {
        self.binaries.iter().any(|b| b.name == name)
            || self.libraries.iter().any(|l| l.name == name)
    }

    #[must_use]
    pub fn is_library(&self, name: &str) -> bool {
        self.libraries.iter().any(|l| l.name == name)
    }

    #[must_use]
    pub fn get_default_project_name() -> String {
        let cargo_path = "Cargo.toml";
        if let Ok(content) = fs::read_to_string(cargo_path)
            && let Ok(value) = content.parse::<DocumentMut>()
            && let Some(name) = value
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
        {
            return name.to_string();
        }
        env::current_dir()
            .ok()
            .and_then(|p| p.file_name().and_then(|n| n.to_str().map(String::from)))
            .unwrap_or_else(|| "my-project".to_string())
    }

    /// # Errors
    /// Returns `RefineryError` if file not found or invalid format.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = de::from_str(&content)
            .map_err(|e| RefineryError::Config(format!("Failed to parse config: {e}")))?;
        Ok(config)
    }

    /// # Errors
    /// Returns `RefineryError` if file cannot be written or serialization fails.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml = self.to_toml()?;
        fs::write(path, toml)?;
        Ok(())
    }

    /// # Errors
    /// Returns `RefineryError` if serialization fails.
    pub fn to_toml(&self) -> Result<String> {
        to_string_pretty(self)
            .map_err(|e| RefineryError::Config(format!("Failed to serialize config: {e}")))
    }
}

impl Default for RefineryConfig {
    fn default() -> Self {
        Self {
            binaries: vec![Binary::default()],
            libraries: vec![],
            targets: Targets::default_standard(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;
    use std::result::Result as StdResult;
    use toml_edit::de;

    #[test]
    fn test_config_optional_binaries() -> StdResult<(), Box<dyn StdError>> {
        let toml = r#"
            [targets]
            linux.gnu.archs = ["x86_64"]
            linux.gnu.artifacts = ["lib-only"]
            
            [[libraries]]
            name = "lib-only"
            path = "src/lib.rs"
            types = ["static"]
        "#;
        let config: RefineryConfig = de::from_str(toml)?;
        assert!(config.binaries.is_empty());
        assert_eq!(config.libraries.len(), 1);
        assert!(config.validate().is_ok());
        Ok(())
    }

    #[test]
    fn test_config_validation_empty() {
        let config = RefineryConfig {
            binaries: vec![],
            libraries: vec![],
            targets: Targets::default(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_binary_default_path() -> StdResult<(), Box<dyn StdError>> {
        let toml = r#"
            [[binaries]]
            name = "test"
            [targets]
        "#;
        let config: RefineryConfig = de::from_str(toml)?;
        assert_eq!(config.binaries[0].path, "src/main.rs");
        Ok(())
    }
}
