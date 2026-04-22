use anyhow::Result;
use clap::Args;
use refinery_rs::core::schema::{
    RefineryConfig, get_cargo_metadata, inject_cargo_fields, update_cargo_toml_with_metadata,
};
use refinery_rs::core::workflow::Workflow;
use refinery_rs::core::workflow::actions;
use refinery_rs::ui::prompts;
use refinery_rs::ui::{prompt_confirm, success, warn};
use refinery_rs::{log_step, prompt, prompt_multi, spinner};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use toml_edit::DocumentMut;

#[derive(Args, Debug)]
pub struct SetupArgs {
    #[arg(short, long)]
    pub force: bool,
}

pub async fn run(args: &SetupArgs) -> Result<()> {
    let mut config = RefineryConfig::load("refinery.toml")?;

    let options = vec![
        "CI/CD Workflow".to_string(),
        "Installers (WiX, deb, rpm)".to_string(),
        "Library Setup".to_string(),
    ];
    let selections: Vec<String> = prompt_multi!("What would you like to setup?", options)?;

    for selection in selections {
        match selection.as_str() {
            "CI/CD Workflow" => setup_ci(&config).await?,
            "Installers (WiX, deb, rpm)" => setup_installers(&config, args.force)?,
            "Library Setup" => setup_lib(&mut config)?,
            _ => unreachable!(),
        }
    }

    Ok(())
}

async fn setup_ci(config: &RefineryConfig) -> Result<()> {
    log_step!("󰄬", "Check", Green, "Configuring CI workflow...");
    let workflow = Workflow::build_workflow(config)?;
    let yaml = workflow.to_yaml()?;

    let path = Path::new(".github/workflows/refinery.yml");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, yaml)?;
    success("CI workflow generated at .github/workflows/refinery.yml");

    if prompt_confirm("Setup sweet testing framework?", true)? {
        setup_sweet().await?;
    }

    Ok(())
}

async fn setup_sweet() -> Result<()> {
    let version = actions::SWEET_DEFAULT_VERSION;
    let sp = spinner!(format!("Downloading sweet {version}..."));

    let os = prompts::get_current_os();
    let arch = prompts::get_current_arch();

    let ext = if os.len() == 7 { ".exe" } else { "" };
    let url = format!(
        "{}/releases/download/{version}/sweet-{os}-{arch}{ext}",
        actions::SWEET_REPO
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        sp.finish_and_clear();
        anyhow::bail!("Failed to download sweet: {}", response.status());
    }

    let bytes = response.bytes().await?;
    let bin_path = if os.len() == 7 { "sweet.exe" } else { "sweet" };

    let mut file = fs::File::create(bin_path)?;
    file.write_all(&bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(bin_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(bin_path, perms)?;
    }

    sp.finish_with_message(format!("Sweet {version} test runner configured."));
    let msg = format!("Sweet {version} setup complete for {os}-{arch}.");
    success(&msg);
    Ok(())
}

fn setup_installers(config: &RefineryConfig, force: bool) -> Result<()> {
    validate_cargo_for_installers()?;

    if config.targets.windows.is_some()
        && prompt_confirm("Configure Windows WiX installer?", true)?
        && prompts::check_and_install("cargo-wix", "wix")?
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
            && prompts::check_and_install("cargo-deb", "deb")?;

        updated |= prompt_confirm("Configure .rpm installer?", true)?
            && prompts::check_and_install("cargo-generate-rpm", "generate-rpm")?;

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

fn setup_lib(config: &mut RefineryConfig) -> Result<()> {
    if config.libraries.is_empty() {
        warn("No libraries defined in refinery.toml.");
        if prompt_confirm("Add a library now?", true)? {
            let default_name = RefineryConfig::get_default_project_name();
            prompts::configure_libraries(config, &default_name)?;
            config.save("refinery.toml")?;
        }
    } else {
        success("Library configuration found and verified.");
    }
    Ok(())
}
