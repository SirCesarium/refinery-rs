use crate::core::schema::{Arch, Binary, Library, NameOverride, TargetMatrix};
use crate::prompt;
use crate::ui::{Result, get_render_config};
use anyhow::anyhow;
use inquire::Select;
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::hash::BuildHasher;

/// # Errors
/// Returns error if prompt fails.
pub fn manage_overrides(
    target: &mut TargetMatrix,
    bins: &[Binary],
    libs: &[Library],
) -> Result<()> {
    let mut ovs = target.overrides.clone().unwrap_or_default();
    loop {
        let mut options = vec![format!("{} Add Override", "+".green())];
        options.extend(ovs.keys().cloned());
        options.push("Back".dimmed().to_string());

        let choice = Select::new("Naming Overrides", options)
            .with_render_config(get_render_config())
            .prompt()?;
        if choice.contains("Back") {
            break;
        }

        if choice.contains("Add Override") {
            let available: Vec<String> = bins
                .iter()
                .map(|b| b.name.clone())
                .chain(libs.iter().map(|l| l.name.clone()))
                .collect();
            if available.is_empty() {
                continue;
            }
            let art = Select::new("Select artifact:", available)
                .with_render_config(get_render_config())
                .prompt()?;
            ovs.insert(art, NameOverride::default());
        } else {
            let art_key = choice.clone();
            edit_name_override_with_remove(&mut ovs, &art_key, &target.archs, &target.pkg)?;
        }
    }
    target.overrides = if ovs.is_empty() { None } else { Some(ovs) };
    Ok(())
}

/// # Errors
/// Returns error if prompt fails.
pub fn edit_name_override_with_remove<S: BuildHasher>(
    ovs: &mut HashMap<String, NameOverride, S>,
    key: &str,
    archs: &[Arch],
    pkgs: &[String],
) -> Result<()> {
    loop {
        let ov = ovs.get(key).ok_or_else(|| anyhow!("Override not found"))?;
        let remove_label = "Remove Override".red().to_string();
        let done_label = "Done".cyan().to_string();

        let field = Select::new(
            &format!("Override: {key}"),
            vec![
                format!(
                    "Custom Base Name: {}",
                    ov.out_name.as_deref().unwrap_or("Default")
                ),
                "Per Architecture Name".to_string(),
                "Per Package Format Name".to_string(),
                remove_label.clone(),
                done_label.clone(),
            ],
        )
        .with_render_config(get_render_config())
        .prompt()?;

        if field == done_label {
            break;
        }
        if field == remove_label {
            ovs.remove(key);
            break;
        }

        let ov_mut = ovs
            .get_mut(key)
            .ok_or_else(|| anyhow!("Failed to get mutable override"))?;
        match field.as_str() {
            _ if field.starts_with("Custom Base Name:") => {
                let cur = ov_mut.out_name.clone().unwrap_or_default();
                let new = prompt!("New base name:", &format!("Current: {cur}"))?;
                ov_mut.out_name = if new.trim().is_empty() {
                    None
                } else {
                    Some(new.trim().to_string())
                };
            }
            _ if field.contains("Architecture") => {
                let mut map = ov_mut.per_arch.clone().unwrap_or_default();
                let arch = Select::new("Target Arch:", archs.to_vec())
                    .with_render_config(get_render_config())
                    .prompt()?;
                let cur = map.get(&arch).cloned().unwrap_or_default();
                let name = prompt!("Override name:", &format!("Current for arch {arch}: {cur}"))?;
                if name.trim().is_empty() {
                    map.remove(&arch);
                } else {
                    map.insert(arch, name.trim().to_string());
                }
                ov_mut.per_arch = if map.is_empty() { None } else { Some(map) };
            }
            _ if field.contains("Package") => {
                if pkgs.is_empty() {
                    continue;
                }
                let mut map = ov_mut.per_pkg.clone().unwrap_or_default();
                let pkg = Select::new("Target Package:", pkgs.to_vec())
                    .with_render_config(get_render_config())
                    .prompt()?;
                let cur = map.get(&pkg).cloned().unwrap_or_default();
                let name = prompt!("Override name:", &format!("Current for pkg {pkg}: {cur}"))?;
                if name.trim().is_empty() {
                    map.remove(&pkg);
                } else {
                    map.insert(pkg, name.trim().to_string());
                }
                ov_mut.per_pkg = if map.is_empty() { None } else { Some(map) };
            }
            _ => break,
        }
    }
    Ok(())
}
