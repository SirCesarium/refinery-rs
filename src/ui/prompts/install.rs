// @swt-disable max-repetition
use crate::errors::Result;
use crate::spinner;
use crate::ui::{prompt_confirm, warn};
use std::process::{Command, Stdio};

/// Checks if a command is installed and prompts for installation if missing.
///
/// # Errors
/// Returns error if installation fails.
pub fn check_and_install(crate_name: &str, bin_name: &str) -> Result<bool> {
    if !check_command(bin_name) {
        warn(&format!("{bin_name} is not installed."));
        if prompt_confirm(&format!("Would you like to install {crate_name}?"), true)? {
            let sp = spinner!(format!("Installing {crate_name}..."));
            let status = Command::new("cargo")
                .arg("install")
                .arg(crate_name)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;

            if status.success() {
                sp.finish_with_message(format!("{crate_name} installed successfully."));
                return Ok(true);
            }
            sp.finish_and_clear();
            warn(&format!("Failed to install {crate_name}."));
            return Ok(false);
        }
        return Ok(false);
    }
    Ok(true)
}

#[must_use]
pub fn check_command(cmd: &str) -> bool {
    let cmd_exe = format!("{cmd}.exe");
    let check = if cfg!(target_os = "windows") {
        &cmd_exe
    } else {
        cmd
    };

    Command::new("cargo")
        .arg(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
        || Command::new("which")
            .arg(check)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
}

#[must_use]
pub const fn get_current_os() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unknown"
    }
}

#[must_use]
pub const fn get_current_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    }
}
