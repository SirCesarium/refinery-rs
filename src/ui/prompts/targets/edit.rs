use super::overrides::manage_overrides;
use super::setup::configure_targets;
use crate::core::schema::{Arch, LibC, OS, RefineryConfig};
use crate::prompt;
use crate::ui::{Result, get_render_config, prompt_confirm};
use anyhow::anyhow;
use inquire::list_option::ListOption;
use inquire::validator::Validation;
use inquire::{MultiSelect, Select};
use owo_colors::OwoColorize;

/// # Errors
/// Returns error if prompt fails.
pub fn edit_targets(config: &mut RefineryConfig) -> Result<bool> {
    let mut changed = false;
    loop {
        let mut options = vec![format!("{} Add/Edit Target OS", "+".green())];
        if let Some(ref l) = config.targets.linux {
            if let Some(ref g) = l.gnu {
                options.push(format!("linux-{} {:?}", "gnu".yellow(), g.archs));
            }
            if let Some(ref m) = l.musl {
                options.push(format!("linux-{} {:?}", "musl".yellow(), m.archs));
            }
        }
        if let Some(ref w) = config.targets.windows {
            options.push(format!("windows {:?}", w.archs));
        }
        if let Some(ref m) = config.targets.macos {
            options.push(format!("macos {:?}", m.archs));
        }

        options.push("Back".dimmed().to_string());

        let choice = Select::new("Manage Targets", options)
            .with_render_config(get_render_config())
            .prompt()?;

        if choice.contains("Back") {
            break;
        }

        if choice.contains("Add/Edit Target OS") {
            configure_targets(&mut config.targets, &config.binaries, &config.libraries)?;
            changed = true;
        } else {
            let (os, libc) = if choice.starts_with("linux-gnu") {
                (OS::Linux, Some(LibC::Gnu))
            } else if choice.starts_with("linux-musl") {
                (OS::Linux, Some(LibC::Musl))
            } else if choice.starts_with("windows") {
                (OS::Windows, None)
            } else if choice.starts_with("macos") {
                (OS::Macos, None)
            } else {
                continue;
            };

            if edit_target_matrix_ui(config, os, libc)? {
                changed = true;
            }
        }
    }
    Ok(changed)
}

#[allow(clippy::too_many_lines)]
fn edit_target_matrix_ui(config: &mut RefineryConfig, os: OS, libc: Option<LibC>) -> Result<bool> {
    loop {
        let target = match (os, libc) {
            (OS::Linux, Some(LibC::Gnu)) => {
                config.targets.linux.as_ref().and_then(|l| l.gnu.as_ref())
            }
            (OS::Linux, Some(LibC::Musl)) => {
                config.targets.linux.as_ref().and_then(|l| l.musl.as_ref())
            }
            (OS::Windows, _) => config.targets.windows.as_ref(),
            (OS::Macos, _) => config.targets.macos.as_ref(),
            _ => None,
        }
        .ok_or_else(|| anyhow!("Target configuration not found"))?;

        let remove_label = "Remove Target Matrix".red().to_string();
        let done_label = "Done".cyan().to_string();

        let field = Select::new(
            &format!("Target Matrix ({os})"),
            vec![
                format!("Archs: {:?}", target.archs),
                format!("Included Artifacts: {:?}", target.artifacts),
                format!("Packages: {:?}", target.pkg),
                format!("Global Suffix: {}", target.ext.as_deref().unwrap_or("None")),
                format!("Strip: {}", if target.strip { "Yes" } else { "No" }),
                "Overrides (Fine-grained)".to_string(),
                remove_label.clone(),
                done_label.clone(),
            ],
        )
        .with_render_config(get_render_config())
        .prompt()?;

        if field == done_label {
            return Ok(true);
        }
        if field == remove_label {
            if prompt_confirm("Delete this target matrix?", false)? {
                match (os, libc) {
                    (OS::Linux, Some(LibC::Gnu)) => {
                        if let Some(mut l) = config.targets.linux.take() {
                            l.gnu = None;
                            config.targets.linux = Some(l);
                        }
                    }
                    (OS::Linux, Some(LibC::Musl)) => {
                        if let Some(mut l) = config.targets.linux.take() {
                            l.musl = None;
                            config.targets.linux = Some(l);
                        }
                    }
                    (OS::Windows, _) => {
                        config.targets.windows = None;
                    }
                    (OS::Macos, _) => {
                        config.targets.macos = None;
                    }
                    _ => {}
                }
                return Ok(true);
            }
            continue;
        }

        let target_mut = match (os, libc) {
            (OS::Linux, Some(LibC::Gnu)) => {
                config.targets.linux.as_mut().and_then(|l| l.gnu.as_mut())
            }
            (OS::Linux, Some(LibC::Musl)) => {
                config.targets.linux.as_mut().and_then(|l| l.musl.as_mut())
            }
            (OS::Windows, _) => config.targets.windows.as_mut(),
            (OS::Macos, _) => config.targets.macos.as_mut(),
            _ => None,
        }
        .ok_or_else(|| anyhow!("Failed to get mutable target"))?;

        match field.as_str() {
            _ if field.starts_with("Archs:") => {
                let options = if os == OS::Macos {
                    vec![Arch::X86_64, Arch::Aarch64]
                } else {
                    vec![Arch::X86_64, Arch::I686, Arch::Aarch64]
                };
                let defaults: Vec<usize> = options
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| target_mut.archs.contains(a))
                    .map(|(i, _)| i)
                    .collect();
                target_mut.archs = MultiSelect::new("Archs:", options)
                    .with_default(&defaults)
                    .with_render_config(get_render_config())
                    .with_validator(|ans: &[ListOption<&Arch>]| {
                        if ans.is_empty() {
                            Ok(Validation::Invalid(
                                "Select at least one architecture".into(),
                            ))
                        } else {
                            Ok(Validation::Valid)
                        }
                    })
                    .prompt()?;
            }
            _ if field.starts_with("Included Artifacts:") => {
                let available: Vec<String> = config
                    .binaries
                    .iter()
                    .map(|b| b.name.clone())
                    .chain(config.libraries.iter().map(|l| l.name.clone()))
                    .collect();
                let defaults: Vec<usize> = available
                    .iter()
                    .enumerate()
                    .filter(|(_, name)| target_mut.artifacts.contains(name))
                    .map(|(i, _)| i)
                    .collect();
                target_mut.artifacts = MultiSelect::new("Artifacts:", available)
                    .with_default(&defaults)
                    .with_render_config(get_render_config())
                    .with_validator(|ans: &[ListOption<&String>]| {
                        if ans.is_empty() {
                            Ok(Validation::Invalid("Select at least one artifact".into()))
                        } else {
                            Ok(Validation::Valid)
                        }
                    })
                    .prompt()?;
            }
            _ if field.starts_with("Packages:") => {
                let cur = target_mut.pkg.join(",");
                let p = prompt!("Packages (csv):", &format!("Current: {cur}"))?;
                target_mut.pkg = p
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            _ if field.starts_with("Global Suffix") => {
                let cur = target_mut.ext.clone().unwrap_or_default();
                let new = prompt!("Extension:", &format!("Current: {cur}"))?;
                target_mut.ext = if new.trim().is_empty() {
                    None
                } else {
                    Some(new.trim().to_string())
                };
            }
            _ if field.starts_with("Strip:") => {
                target_mut.strip = !target_mut.strip;
            }
            _ if field.contains("Overrides") => {
                manage_overrides(target_mut, &config.binaries, &config.libraries)?;
            }
            _ => {}
        }
    }
}
