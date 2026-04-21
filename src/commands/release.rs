#[cfg(feature = "semver")]
mod internal {
    use anyhow::{Context, Result, anyhow};
    use clap::{Args, Subcommand};
    use refinery_rs::core::config::Config;
    use refinery_rs::core::release::{BumpType, ReleaseManager};
    use refinery_rs::ui::{info, success};
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    #[derive(Args, Debug)]
    pub struct ReleaseArgs {
        #[command(subcommand)]
        pub action: Option<ReleaseAction>,

        /// Delete local and remote tag
        #[arg(short = 'd', long, value_name = "TAG")]
        pub delete: Option<String>,

        /// Create a changelog entry using your preferred editor
        #[arg(short = 'e', long)]
        pub changelog: bool,
    }

    #[derive(Subcommand, Debug)]
    pub enum ReleaseAction {
        /// Bump major version
        Major(BumpArgs),
        /// Bump minor version
        Minor(BumpArgs),
        /// Bump patch version
        Patch(BumpArgs),
    }

    #[derive(Args, Debug)]
    pub struct BumpArgs {
        /// Pre-release candidate (auto-increments if already on rc, else starts at 1)
        #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = "0")]
        pub candidate: Option<u64>,
    }

    pub fn run(args: &ReleaseArgs) -> Result<()> {
        let config = Config::load()?;
        let manager = ReleaseManager::new(&config.main_branch, config.auto_check_cargo);

        if let Some(tag) = &args.delete {
            info(&format!("Deleting tag {tag}..."));
            ReleaseManager::delete_tag(tag)?;
            success(&format!("Deleted tag {tag} (local and remote)"));
            return Ok(());
        }

        let action = args.action.as_ref().ok_or_else(|| {
            anyhow!("Specify a release action (major, minor, patch) or use --delete")
        })?;

        let bump = match action {
            ReleaseAction::Major(_) => BumpType::Major,
            ReleaseAction::Minor(_) => BumpType::Minor,
            ReleaseAction::Patch(_) => BumpType::Patch,
        };

        let candidate = match action {
            ReleaseAction::Major(a) | ReleaseAction::Minor(a) | ReleaseAction::Patch(a) => {
                a.candidate
            }
        };

        let new_version = manager.bump_version(&bump, candidate)?;
        success(&format!("Updated Cargo.toml to {new_version}"));

        let changelog_path = if args.changelog {
            capture_changelog(&config)?
        } else {
            None
        };

        manager.finalize_git_release(&new_version, changelog_path.as_deref())?;
        success(&format!("Created and pushed tag v{new_version}"));

        Ok(())
    }

    fn capture_changelog(config: &Config) -> Result<Option<PathBuf>> {
        let editor = config.get_editor();
        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path().to_owned();

        info(&format!("Opening editor ({editor}) for changelog..."));
        let parts: Vec<&str> = editor.split_whitespace().collect();
        let status = Command::new(parts[0])
            .args(&parts[1..])
            .arg(&temp_path)
            .status()
            .context("Failed to open editor")?;

        if status.success() {
            let content = fs::read_to_string(&temp_path)?;
            if !content.trim().is_empty() {
                success("Changelog captured.");
                return Ok(Some(temp_path));
            }
        }
        Ok(None)
    }
}

#[cfg(not(feature = "semver"))]
mod internal {
    #[derive(clap::Args, Debug)]
    pub struct ReleaseArgs {}

    pub fn run(_args: &ReleaseArgs) -> anyhow::Result<()> {
        anyhow::bail!("The 'semver' feature is required for the release command")
    }
}

pub use internal::*;
