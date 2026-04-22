use anyhow::Result;
use clap::Args;
use refinery_rs::core::schema::{RefineryConfig, Targets};
use refinery_rs::core::workflow::Workflow;
use refinery_rs::ui::prompts::{
    configure_binaries, configure_libraries, configure_targets, select_init_action,
};
use refinery_rs::ui::{print_banner, print_highlighted_toml, prompt_confirm, success, warn};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct InitArgs {
    #[arg(short, long)]
    pub force: bool,
}

pub fn run(args: &InitArgs) -> Result<()> {
    print_banner();
    let config_path = PathBuf::from("refinery.toml");
    if config_path.exists() && !args.force {
        anyhow::bail!(
            "Configuration file 'refinery.toml' already exists. Use --force to overwrite."
        );
    }

    let default_name = RefineryConfig::get_default_project_name();
    let mut config = RefineryConfig {
        binaries: vec![],
        libraries: vec![],
        targets: Targets::default(),
    };

    loop {
        let selection = select_init_action()?;

        match selection.as_str() {
            "Add Binaries" => configure_binaries(&mut config, &default_name)?,
            "Add Libraries" => configure_libraries(&mut config, &default_name)?,
            "Configure Targets" => {
                if config.binaries.is_empty() && config.libraries.is_empty() {
                    println!();
                    warn("Add an artifact (binary or library) before configuring targets.");
                } else {
                    configure_targets(&mut config.targets, &config.binaries, &config.libraries)?;
                }
            }
            "Review & Save" => {
                if handle_save(&config, &config_path)? {
                    break;
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn handle_save(config: &RefineryConfig, path: &Path) -> Result<bool> {
    config.validate()?;
    let toml = config.to_toml()?;
    println!("\n--- Preview ---");
    print_highlighted_toml(&toml);
    println!();
    if prompt_confirm("Save to refinery.toml?", true)? {
        config.save(path)?;
        println!();
        success("Project initialized successfully.");

        if !config.targets.is_empty() && prompt_confirm("Generate GitHub Actions workflow?", true)?
        {
            generate_workflow(config)?;
        }
        return Ok(true);
    }
    Ok(false)
}

fn generate_workflow(config: &RefineryConfig) -> Result<()> {
    let workflow = Workflow::primary_workflow(config)?;
    let yaml = workflow.to_yaml()?;
    let workflow_path = Path::new(".github/workflows/refinery.yml");
    if let Some(parent) = workflow_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(workflow_path, yaml)?;
    success("GitHub Actions workflow generated at .github/workflows/refinery.yml");
    Ok(())
}
