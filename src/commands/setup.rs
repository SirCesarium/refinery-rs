use anyhow::Result;
use clap::Args;
use refinery_rs::core::schema::{LibType, RefineryConfig, prepare_cargo_lib};
use refinery_rs::core::workflow::Workflow;
use refinery_rs::core::workflow::actions;
use refinery_rs::ui::prompts::{self, installers};
use refinery_rs::ui::{prompt_confirm, success, warn};
use refinery_rs::{log_step, prompt_multi, spinner};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

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
            "Installers (WiX, deb, rpm)" => installers::setup_installers(&config, args.force)?,
            "Library Setup" => setup_lib(&mut config)?,
            _ => unreachable!(),
        }
    }

    Ok(())
}

async fn setup_ci(config: &RefineryConfig) -> Result<()> {
    log_step!("󰄬", "Check", Green, "Configuring unified CI/CD workflow...");
    let workflow = Workflow::primary_workflow(config)?;
    let yaml = workflow.to_yaml()?;

    let path = Path::new(".github/workflows/refinery.yml");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, yaml)?;
    success("Unified CI/CD workflow generated at .github/workflows/refinery.yml");

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

fn setup_lib(config: &mut RefineryConfig) -> Result<()> {
    if config.libraries.is_empty() {
        warn("No libraries defined in refinery.toml.");
        if prompt_confirm("Add a library configuration now?", true)? {
            let default_name = RefineryConfig::get_default_project_name();
            prompts::configure_libraries(config, &default_name)?;
            config.save("refinery.toml")?;
        } else {
            return Ok(());
        }
    }

    for lib in &config.libraries {
        log_step!("󰒓", "Lib", Yellow, "Configuring library: {}...", lib.name);

        let cargo_content = fs::read_to_string("Cargo.toml")?;
        let crate_types: Vec<String> = lib
            .types
            .iter()
            .map(|t| match t {
                LibType::Dynamic => "cdylib".to_string(),
                LibType::Static => "staticlib".to_string(),
            })
            .collect();

        let updated_cargo = prepare_cargo_lib(&cargo_content, crate_types, lib.headers)?;
        fs::write("Cargo.toml", updated_cargo)?;
        success("Cargo.toml configured for library export.");

        let lib_path = Path::new(&lib.path);
        if !lib_path.exists() {
            if let Some(parent) = lib_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let boilerplate = r#"#[no_mangle]
pub extern "C" fn hello_refinery() {
    println!("Hello from Refinery-optimized library!");
}
"#;
            fs::write(lib_path, boilerplate)?;
            success(&format!("Boilerplate created at {}", lib.path));
        }

        if lib.headers {
            prompts::check_and_install("cbindgen", "cbindgen")?;
            success("cbindgen ready for header generation.");
        }
    }

    Ok(())
}
