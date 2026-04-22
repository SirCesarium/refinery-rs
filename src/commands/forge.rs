use clap::Args;
use refinery_rs::core::schema::RefineryConfig;
use refinery_rs::core::workflow::Workflow;
use refinery_rs::ui::{success, warn};
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;

#[derive(Args, Debug)]
pub struct ForgeArgs {
    #[arg(short, long)]
    pub force: bool,
}

pub fn run(args: &ForgeArgs) -> anyhow::Result<()> {
    let config = RefineryConfig::load("refinery.toml")?;

    validate_cargo_metadata(&config)?;

    let workflow = Workflow::release_workflow(&config)?;
    let yaml = workflow.to_yaml()?;

    let workflow_path = Path::new(".github/workflows/release.yml");
    if workflow_path.exists() && !args.force {
        warn("Release workflow already exists. Use --force to overwrite.");
        return Ok(());
    }

    if let Some(parent) = workflow_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(workflow_path, yaml)?;
    success("Release workflow generated at .github/workflows/release.yml");

    Ok(())
}

fn validate_cargo_metadata(config: &RefineryConfig) -> anyhow::Result<()> {
    let cargo_content = fs::read_to_string("Cargo.toml")?;
    let cargo_toml = cargo_content.parse::<DocumentMut>()?;

    let mut needs_deb = false;
    let mut needs_rpm = false;
    let mut needs_wix = false;

    if let Some(linux) = &config.targets.linux {
        if let Some(gnu) = &linux.gnu {
            for pkg in &gnu.pkg {
                match pkg.as_str() {
                    "deb" => needs_deb = true,
                    "rpm" => needs_rpm = true,
                    "msi" => needs_wix = true,
                    _ => {}
                }
            }
        }
        if let Some(musl) = &linux.musl {
            for pkg in &musl.pkg {
                match pkg.as_str() {
                    "deb" => needs_deb = true,
                    "rpm" => needs_rpm = true,
                    "msi" => needs_wix = true,
                    _ => {}
                }
            }
        }
    }

    if let Some(windows) = &config.targets.windows {
        for pkg in &windows.pkg {
            if pkg == "msi" {
                needs_wix = true;
            }
        }
    }

    if needs_deb
        && cargo_toml
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("deb"))
            .is_none()
    {
        warn(
            "Target 'deb' configured but [package.metadata.deb] missing in Cargo.toml. Run 'refinery setup' first.",
        );
    }

    if needs_rpm
        && cargo_toml
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("generate-rpm"))
            .is_none()
    {
        warn(
            "Target 'rpm' configured but [package.metadata.generate-rpm] missing in Cargo.toml. Run 'refinery setup' first.",
        );
    }

    if needs_wix && !Path::new("wix").exists() {
        warn(
            "Target 'msi' configured but 'wix' directory missing. Run 'refinery setup' or 'cargo wix init' first.",
        );
    }

    Ok(())
}
