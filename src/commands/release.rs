#[cfg(feature = "semver")]
use anyhow::{Context, Result, anyhow};
#[cfg(feature = "semver")]
use clap::{Args, Subcommand};
#[cfg(feature = "semver")]
use refinery_rs::ui::{info, success};
#[cfg(feature = "semver")]
use semver::{Prerelease, Version};
#[cfg(feature = "semver")]
use std::fs;
#[cfg(feature = "semver")]
use std::process::Command;
#[cfg(feature = "semver")]
use toml_edit::DocumentMut;

#[cfg(feature = "semver")]
#[derive(Args, Debug)]
pub struct ReleaseArgs {
    #[command(subcommand)]
    pub action: Option<ReleaseAction>,

    /// Delete local and remote tag
    #[arg(short = 'd', long, value_name = "TAG")]
    pub delete: Option<String>,
}

#[cfg(feature = "semver")]
#[derive(Subcommand, Debug)]
pub enum ReleaseAction {
    /// Bump major version
    Major(BumpArgs),
    /// Bump minor version
    Minor(BumpArgs),
    /// Bump patch version
    Patch(BumpArgs),
}

#[cfg(feature = "semver")]
#[derive(Args, Debug)]
pub struct BumpArgs {
    /// Pre-release candidate number (e.g. 1 for -rc1)
    #[arg(long, short = 'c')]
    pub candidate: Option<u64>,
}

#[cfg(feature = "semver")]
pub fn run(args: &ReleaseArgs) -> Result<()> {
    if let Some(tag) = &args.delete {
        return delete_release(tag);
    }

    let action = args
        .action
        .as_ref()
        .ok_or_else(|| anyhow!("Specify a release action (major, minor, patch) or use --delete"))?;

    bump_release(action)
}

#[cfg(feature = "semver")]
fn bump_release(action: &ReleaseAction) -> Result<()> {
    let content = fs::read_to_string("Cargo.toml").context("Failed to read Cargo.toml")?;
    let mut doc = content
        .parse::<DocumentMut>()
        .context("Failed to parse Cargo.toml")?;

    let current_version_str = doc["package"]["version"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing version in Cargo.toml"))?;
    let current_version =
        Version::parse(current_version_str).context("Failed to parse current version")?;

    info(&format!("Current version: {current_version}"));

    let mut next_version = current_version.clone();
    let (ReleaseAction::Major(args) | ReleaseAction::Minor(args) | ReleaseAction::Patch(args)) =
        action;
    let candidate = args.candidate;

    let is_pre = !current_version.pre.is_empty();

    match action {
        ReleaseAction::Major(_) => {
            if !is_pre || current_version.minor != 0 || current_version.patch != 0 {
                next_version.major += 1;
                next_version.minor = 0;
                next_version.patch = 0;
            }
        }
        ReleaseAction::Minor(_) => {
            if !is_pre || current_version.patch != 0 {
                next_version.minor += 1;
                next_version.patch = 0;
            }
        }
        ReleaseAction::Patch(_) => {
            if !is_pre {
                next_version.patch += 1;
            }
        }
    }

    if let Some(c) = candidate {
        next_version.pre = Prerelease::new(&format!("rc{c}"))?;
    } else {
        next_version.pre = Prerelease::EMPTY;
    }

    if next_version <= current_version && candidate.is_none() {
        return Err(anyhow!(
            "Next version {next_version} must be greater than current version {current_version}"
        ));
    }

    let new_version = next_version.to_string();
    doc["package"]["version"] = toml_edit::value(&new_version);
    fs::write("Cargo.toml", doc.to_string()).context("Failed to write Cargo.toml")?;

    success(&format!("Updated Cargo.toml to {new_version}"));

    let tag = format!("v{new_version}");

    git(&["add", "Cargo.toml"])?;
    git(&["commit", "-m", &format!("chore: release {new_version}")])?;
    git(&["tag", "-a", &tag, "-m", &format!("Release {new_version}")])?;
    git(&["push", "origin", "main"])?;
    git(&["push", "origin", "--tags"])?;

    success(&format!("Created and pushed tag {tag}"));

    Ok(())
}

#[cfg(feature = "semver")]
fn delete_release(tag: &str) -> Result<()> {
    info(&format!("Deleting tag {tag}..."));

    let _ = git(&["tag", "-d", tag]);
    git(&["push", "origin", "--delete", tag])?;

    success(&format!("Deleted tag {tag} (local and remote)"));
    Ok(())
}

#[cfg(feature = "semver")]
fn git(args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .status()
        .context("Failed to execute git command")?;

    if !status.success() {
        return Err(anyhow!("Git command failed: git {}", args.join(" ")));
    }
    Ok(())
}

#[cfg(not(feature = "semver"))]
#[derive(clap::Args, Debug)]
pub struct ReleaseArgs {}

#[cfg(not(feature = "semver"))]
pub fn run(_args: &ReleaseArgs) -> anyhow::Result<()> {
    anyhow::bail!("The 'semver' feature is required for the release command")
}
