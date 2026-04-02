//! Logic for the 'release' command to automate GitHub tagging and releases.

use std::fs;
use std::io::Read;
use std::process::Command;
use crate::errors::Result;
use crate::ui;
use crate::ReleaseAction;
use semver::Version;
use tempfile::NamedTempFile;
use toml_edit::{DocumentMut, value};

/// Available editors for changelog editing.
const EDITORS: &[(&str, &str)] = &[
    ("vscode", "code"),
    ("neovim", "nvim"),
    ("vim", "vim"),
    ("nano", "nano"),
];

/// Orchestrates the release or deletion process.
pub fn run(action: ReleaseAction) -> Result<()> {
    match action {
        ReleaseAction::Patch { changelog, title, prerelease } => {
            create_release("patch", changelog, title, prerelease)
        }
        ReleaseAction::Minor { changelog, title, prerelease } => {
            create_release("minor", changelog, title, prerelease)
        }
        ReleaseAction::Major { changelog, title, prerelease } => {
            create_release("major", changelog, title, prerelease)
        }
        ReleaseAction::Delete { name } => delete_release(&name),
    }
}

fn create_release(bump: &str, use_changelog: bool, title: Option<String>, prerelease: bool) -> Result<()> {
    ui::print_banner();
    ui::info("Starting the release process...");

    check_repo_status()?;

    let current_version = get_current_version()?;
    let mut version = Version::parse(&current_version).map_err(std::io::Error::other)?;
    
    match bump {
        "patch" => version.patch += 1,
        "minor" => {
            version.minor += 1;
            version.patch = 0;
        }
        "major" => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
        }
        _ => unreachable!(),
    }

    let new_version = version.to_string();
    let new_tag = format!("v{new_version}");
    ui::info(&format!("Current version: v{current_version}"));
    ui::success(&format!("Planned version: {new_tag}"));

    let mut changelog_content = String::new();
    if use_changelog {
        changelog_content = capture_changelog()?;
    }

    println!("\n{}\n", console::Style::new().cyan().apply_to("--- Release Preview ---"));
    ui::info(&format!("File: Cargo.toml [version: {current_version} -> {new_version}]"));
    ui::info(&format!("Git: Create commit 'chore: bump version to {new_tag}'"));
    ui::info(&format!("Git: Create tag '{new_tag}'"));

    if ui::prompt_confirm("Do you want to proceed with the release?", true)? {
        bump_cargo_toml(&new_version)?;
        update_cargo_lock()?;
        commit_version_bump(&new_tag)?;
        create_and_push_tag(&new_tag, title.as_deref(), &changelog_content, prerelease)?;
    }

    Ok(())
}

fn check_repo_status() -> Result<()> {
    ui::info("Checking repository status...");
    let output = Command::new("git").args(["status", "--short"]).output()?;
    let status_text = String::from_utf8_lossy(&output.stdout);

    if !status_text.trim().is_empty() {
        ui::warn("The repository has uncommitted changes:");
        println!("{status_text}");
        ui::info("These changes will be included in the auto-commit if they are staged.");
        if !ui::prompt_confirm("Are you sure you want to continue with a dirty repository?", false)? {
            ui::info("Release aborted by user.");
            std::process::exit(0);
        }
    }
    Ok(())
}

fn bump_cargo_toml(new_version: &str) -> Result<()> {
    ui::info(&format!("Updating Cargo.toml version to {new_version}..."));
    let content = fs::read_to_string("Cargo.toml")?;
    let mut doc = content.parse::<DocumentMut>().map_err(std::io::Error::other)?;
    doc["package"]["version"] = value(new_version);
    fs::write("Cargo.toml", doc.to_string())?;
    Ok(())
}

fn update_cargo_lock() -> Result<()> {
    ui::info("Synchronizing Cargo.lock...");
    let status = Command::new("cargo")
        .args(["fetch"])
        .status()?;

    if status.success() {
        ui::success("Cargo.lock synchronized.");
        Ok(())
    } else {
        Err(std::io::Error::other("Failed to update Cargo.lock").into())
    }
}

fn commit_version_bump(tag: &str) -> Result<()> {
    ui::info("Committing version bump...");
    Command::new("git").args(["add", "Cargo.toml", "Cargo.lock"]).status()?;
    let status = Command::new("git")
        .args(["commit", "-m", &format!("chore: bump version to {tag}")])
        .status()?;

    if status.success() {
        ui::success("Version bump committed.");
        Ok(())
    } else {
        Err(std::io::Error::other("Failed to commit Cargo.toml changes").into())
    }
}

fn delete_release(name: &str) -> Result<()> {
    ui::print_banner();
    ui::info(&format!("Attempting to delete release tag: {name}..."));

    let local_status = Command::new("git").args(["tag", "-d", name]).status();
    if let Ok(s) = local_status {
        if s.success() { ui::success(&format!("Deleted local tag: {name}")); }
        else { ui::warn(&format!("Local tag '{name}' not found.")); }
    }

    if ui::prompt_confirm(&format!("Delete remote tag '{name}' from origin?"), true)? {
        ui::info(&format!("Pushing deletion to origin for tag: {name}..."));
        let remote_status = Command::new("git").args(["push", "origin", "--delete", name]).status();
        if let Ok(s) = remote_status {
            if s.success() { ui::success(&format!("Deleted remote tag: {name}")); }
            else { ui::warn(&format!("Remote tag '{name}' not found on origin.")); }
        }
    }
    Ok(())
}

fn get_current_version() -> Result<String> {
    let content = fs::read_to_string("Cargo.toml")?;
    let doc = content.parse::<DocumentMut>().map_err(std::io::Error::other)?;
    let version = doc["package"]["version"].as_str()
        .ok_or_else(|| std::io::Error::other("Missing package version"))?;
    Ok(version.to_string())
}

fn capture_changelog() -> Result<String> {
    let (_, editor_cmd) = discover_editor()?;
    ui::info(&format!("Opening editor: {editor_cmd}..."));

    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();
    fs::write(path, "# Changelog\n\n- ")?;

    let status = Command::new(editor_cmd).arg(path).status()?;
    if !status.success() {
        return Err(std::io::Error::other("Editor exited with non-zero status").into());
    }

    let mut content = String::new();
    fs::File::open(path)?.read_to_string(&mut content)?;
    Ok(content)
}

fn discover_editor() -> Result<(&'static str, &'static str)> {
    for (name, cmd) in EDITORS {
        if Command::new(cmd).arg("--version").output().is_ok() {
            return Ok((*name, *cmd));
        }
    }
    Err(std::io::Error::other("No suitable editor found").into())
}

fn create_and_push_tag(tag: &str, title: Option<&str>, body: &str, prerelease: bool) -> Result<()> {
    ui::info(&format!("Creating local tag: {tag}..."));
    let status = Command::new("git")
        .args(["tag", "-a", tag, "-m", title.unwrap_or(tag)])
        .status()?;

    if status.success() {
        ui::info("Pushing changes and tag to origin...");
        let push_status = Command::new("git").args(["push", "origin", "HEAD", "--tags"]).status()?;
        if push_status.success() {
            ui::success("Release pushed successfully!");
            if is_gh_installed() {
                create_gh_release(tag, title, body, prerelease)?;
            }
        }
    }
    Ok(())
}

fn create_gh_release(tag: &str, title: Option<&str>, body: &str, prerelease: bool) -> Result<()> {
    ui::info("GitHub CLI detected. Creating a formal release...");
    let mut args = vec!["release", "create", tag, "--title", title.unwrap_or(tag), "--notes", body];
    if prerelease { args.push("--prerelease"); }
    let _ = Command::new("gh").args(args).status();
    Ok(())
}

fn is_gh_installed() -> bool {
    Command::new("gh").arg("--version").output().is_ok()
}
