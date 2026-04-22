use crate::core::schema::{
    RefineryConfig, get_cargo_metadata, inject_cargo_fields, update_cargo_toml_with_metadata,
};
use crate::errors::Result;
use crate::ui::prompts::install;
use crate::ui::{prompt_confirm, success, warn};
use crate::{log_step, prompt, spinner};
use std::fs;
use std::path::Path;
use std::process::Command;
use toml_edit::DocumentMut;

/// Sets up installers (`WiX`, `deb`, `rpm`) by updating `Cargo.toml` and initializing tools.
///
/// # Errors
/// Returns error if Cargo.toml updates or tool initialization fails.
pub fn setup_installers(config: &RefineryConfig, force: bool) -> Result<()> {
    validate_cargo_for_installers()?;

    if config.targets.windows.is_some()
        && prompt_confirm("Configure Windows WiX installer?", true)?
        && install::check_and_install("cargo-wix", "wix")?
    {
        let mut args = vec!["wix", "init"];
        if force
            || (Path::new("wix").exists()
                && prompt_confirm("WiX files already exist. Overwrite?", false)?)
        {
            args.push("--force");
        }
        run_installer_init("cargo", &args, "WiX");
    }

    if config.targets.linux.is_some()
        && prompt_confirm("Configure Linux installers (deb/rpm)?", true)?
    {
        let mut updated = prompt_confirm("Configure .deb installer?", true)?
            && install::check_and_install("cargo-deb", "deb")?;

        updated |= prompt_confirm("Configure .rpm installer?", true)?
            && install::check_and_install("cargo-generate-rpm", "generate-rpm")?;

        if updated {
            log_step!("󰄬", "*", Green, "Updating Cargo.toml with metadata...");
            let cargo_content = fs::read_to_string("Cargo.toml")?;
            let updated_toml = update_cargo_toml_with_metadata(&cargo_content)?;
            fs::write("Cargo.toml", updated_toml)?;
            success("Linux metadata added to Cargo.toml");
        }
    }

    Ok(())
}

fn run_installer_init(cmd: &str, args: &[&str], label: &str) {
    let sp = spinner!(format!("Initializing {label}..."));
    let status = Command::new(cmd).args(args).status();

    match status {
        Ok(s) if s.success() => sp.finish_with_message(format!("{label} initialized.")),
        _ => {
            sp.finish_and_clear();
            warn(&format!("Failed to run '{cmd} {}'", args.join(" ")));
        }
    }
}

fn validate_cargo_for_installers() -> Result<()> {
    let cargo_content = fs::read_to_string("Cargo.toml")?;
    let doc = cargo_content.parse::<DocumentMut>()?;
    let metadata = get_cargo_metadata(&doc);

    let mut new_authors = None;
    let mut new_license = None;
    let mut new_description = None;
    let mut new_repo = None;

    if metadata.authors.is_empty() {
        warn("Missing 'authors' in Cargo.toml. It is required for installers.");
        let author: String = prompt!("Enter author name (e.g. John Doe <john@example.com>)")?;
        new_authors = Some(vec![author]);
    }

    if metadata.license.is_empty() {
        warn("Missing 'license' in Cargo.toml. It is required for installers.");
        let license: String = prompt!("Enter license (e.g. MIT OR Apache-2.0)")?;
        new_license = Some(license);
    }

    if metadata.description.is_empty() {
        warn("Missing 'description' in Cargo.toml. It is highly recommended.");
        let desc: String = prompt!("Enter project description")?;
        new_description = Some(desc);
    }

    if metadata.repository.is_empty() {
        warn("Missing 'repository' URL in Cargo.toml. It is highly recommended.");
        let repo: String = prompt!("Enter repository URL (e.g. https://github.com/user/repo)")?;
        new_repo = Some(repo);
    }

    if new_authors.is_some()
        || new_license.is_some()
        || new_description.is_some()
        || new_repo.is_some()
    {
        let updated = inject_cargo_fields(
            &cargo_content,
            new_authors,
            new_license,
            new_description,
            new_repo,
        )?;
        fs::write("Cargo.toml", updated)?;
        success("Cargo.toml updated with missing fields.");
    }

    Ok(())
}
