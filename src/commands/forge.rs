use clap::Args;
use refinery_rs::core::project;
use refinery_rs::core::schema::{CargoMetadata, RefineryConfig, get_cargo_metadata};
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

    project::sync_metadata(&config)?;
    validate_cargo_metadata(&config)?;

    let workflow = Workflow::primary_workflow(&config)?;
    let yaml = workflow.to_yaml()?;

    let workflow_path = Path::new(".github/workflows/refinery.yml");
    if workflow_path.exists() && !args.force {
        warn("Workflow already exists. Use --force to overwrite.");
        return Ok(());
    }

    if let Some(parent) = workflow_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(workflow_path, yaml)?;
    success("Unified pipeline generated at .github/workflows/refinery.yml");

    Ok(())
}

fn validate_cargo_metadata(config: &RefineryConfig) -> anyhow::Result<()> {
    let cargo_content = fs::read_to_string("Cargo.toml")?;
    let cargo_toml = cargo_content.parse::<DocumentMut>()?;
    let metadata = get_cargo_metadata(&cargo_toml);

    let mut needs_deb = false;
    let mut needs_rpm = false;
    let mut needs_wix = false;

    if let Some(linux) = &config.targets.linux {
        let matrices = [linux.gnu.as_ref(), linux.musl.as_ref()];
        for m in matrices.into_iter().flatten() {
            for pkg in &m.pkg {
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

    check_missing_fields(needs_deb, needs_rpm, &metadata);

    if needs_wix && !Path::new("wix").exists() {
        warn("Target 'msi' configured but 'wix' directory missing. Run 'refinery setup' first.");
    }

    Ok(())
}

fn check_missing_fields(deb: bool, rpm: bool, meta: &CargoMetadata) {
    if (deb || rpm)
        && (meta.authors.is_empty()
            || meta.description.is_empty()
            || meta.license.is_empty()
            || meta.repository.is_empty())
    {
        warn(
            "Required metadata (authors, description, license, repository) missing in Cargo.toml. Run 'refinery setup' first.",
        );
    }
}
