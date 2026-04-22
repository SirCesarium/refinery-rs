#![cfg(feature = "semver")]

use anyhow::{Context, Result, anyhow};
use semver::{Prerelease, Version};
use std::fs;
use std::path::Path;
use std::process::Command;
use toml_edit::DocumentMut;

pub enum BumpType {
    Major,
    Minor,
    Patch,
}

pub struct ReleaseManager<'a> {
    pub main_branch: &'a str,
    pub auto_check_cargo: bool,
}

impl<'a> ReleaseManager<'a> {
    #[must_use]
    pub const fn new(main_branch: &'a str, auto_check_cargo: bool) -> Self {
        Self {
            main_branch,
            auto_check_cargo,
        }
    }

    /// Performs the version bump in Cargo.toml and updates Cargo.lock if needed.
    ///
    /// # Errors
    /// Returns error if Cargo.toml is missing or invalid.
    pub fn bump_version(&self, bump: &BumpType, candidate: Option<u64>) -> Result<String> {
        let content = fs::read_to_string("Cargo.toml").context("Failed to read Cargo.toml")?;
        let mut doc = content
            .parse::<DocumentMut>()
            .context("Failed to parse Cargo.toml")?;

        let current_version_str = doc["package"]["version"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing version in Cargo.toml"))?;
        let current_version =
            Version::parse(current_version_str).context("Failed to parse current version")?;

        let next_version = Self::calculate_next_version(&current_version, bump, candidate)?;

        if next_version <= current_version && candidate.is_none() {
            anyhow::bail!(
                "Next version {next_version} must be greater than current version {current_version}"
            );
        }

        let new_version = next_version.to_string();
        doc["package"]["version"] = toml_edit::value(&new_version);
        fs::write("Cargo.toml", doc.to_string()).context("Failed to write Cargo.toml")?;

        if self.auto_check_cargo {
            let _ = Command::new("cargo").arg("check").status();
        }

        Ok(new_version)
    }

    fn calculate_next_version(
        current: &Version,
        bump: &BumpType,
        candidate: Option<u64>,
    ) -> Result<Version> {
        let mut next = current.clone();
        let is_pre = !current.pre.is_empty();

        match bump {
            BumpType::Major => {
                if !is_pre || current.minor != 0 || current.patch != 0 {
                    next.major += 1;
                    next.minor = 0;
                    next.patch = 0;
                }
            }
            BumpType::Minor => {
                if !is_pre || current.patch != 0 {
                    next.minor += 1;
                    next.patch = 0;
                }
            }
            BumpType::Patch => {
                if !is_pre {
                    next.patch += 1;
                }
            }
        }

        if let Some(c) = candidate {
            let rc_num = if c == 0 {
                Self::parse_rc_number(&current.pre)
            } else {
                c
            };
            next.pre = Prerelease::new(&format!("rc{rc_num}"))?;
        } else {
            next.pre = Prerelease::EMPTY;
        }

        Ok(next)
    }

    fn parse_rc_number(pre: &Prerelease) -> u64 {
        let s = pre.as_str();
        if let Some(stripped) = s.strip_prefix("rc")
            && let Ok(n) = stripped.parse::<u64>()
        {
            return n + 1;
        }
        1
    }

    /// Commits changes and creates a git tag.
    ///
    /// # Errors
    /// Returns error if git commands fail.
    pub fn finalize_git_release(&self, version: &str, changelog: Option<&Path>) -> Result<()> {
        let tag = format!("v{version}");
        Self::git(&["add", "Cargo.toml"])?;
        if Path::new("Cargo.lock").exists() {
            Self::git(&["add", "Cargo.lock"])?;
        }
        Self::git(&["commit", "-m", &format!("chore: release {version}")])?;
        Self::git(&["tag", "-a", &tag, "-m", &format!("Release {version}")])?;
        Self::git(&["push", "origin", self.main_branch])?;
        Self::git(&["push", "origin", "--tags"])?;

        if let Some(path) = changelog {
            let _ = Command::new("gh")
                .arg("release")
                .arg("create")
                .arg(&tag)
                .arg("--title")
                .arg(&tag)
                .arg("--notes-file")
                .arg(path.to_str().unwrap_or_default())
                .status();
        }

        Ok(())
    }

    /// Deletes a tag locally and remotely.
    ///
    /// # Errors
    /// Returns error if git commands fail.
    pub fn delete_tag(tag: &str) -> Result<()> {
        let _ = Self::git(&["tag", "-d", tag]);
        Self::git(&["push", "origin", "--delete", tag])
    }

    fn git(args: &[&str]) -> Result<()> {
        let status = Command::new("git")
            .args(args)
            .status()
            .context("Failed to execute git")?;
        if !status.success() {
            anyhow::bail!("Git command failed: git {}", args.join(" "));
        }
        Ok(())
    }
}
