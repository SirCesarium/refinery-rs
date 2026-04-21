use crate::core::schema::{Binary, RefineryConfig};
use crate::prompt;
use crate::ui::{Result, get_render_config, prompt_confirm, warn};
use anyhow::anyhow;
use inquire::Select;
use owo_colors::OwoColorize;

/// # Errors
/// Returns error if prompt fails.
pub fn edit_binaries(config: &mut RefineryConfig) -> Result<bool> {
    let mut changed = false;
    loop {
        let mut options = vec![format!("{} Add New Binary", "+".green())];
        options.extend(config.binaries.iter().map(|b| b.name.clone()));
        options.push("Back".dimmed().to_string());

        let choice = Select::new("Manage Binaries", options)
            .with_render_config(get_render_config())
            .prompt()?;

        if choice.contains("Back") {
            break;
        }

        if choice.contains("Add New Binary") {
            let name = prompt!("Name:", "The name of your binary artifact")?;
            if name.trim().is_empty() {
                continue;
            }
            let path_input = prompt!("Path (default: src/main.rs):", "Source file location")?;
            let path = if path_input.trim().is_empty() {
                "src/main.rs".to_string()
            } else {
                path_input.trim().to_string()
            };

            config.binaries.push(Binary {
                name: name.trim().to_string(),
                path,
                ..Default::default()
            });
            changed = true;
        } else {
            let idx = config
                .binaries
                .iter()
                .position(|b| b.name == choice)
                .ok_or_else(|| anyhow!("Binary not found"))?;

            if edit_binary_fields(&mut config.binaries, idx)? {
                changed = true;
            }
        }
    }
    Ok(changed)
}

/// # Errors
/// Returns error if prompt fails.
pub fn edit_binary_fields(bins: &mut Vec<Binary>, idx: usize) -> Result<bool> {
    loop {
        let bin = bins
            .get(idx)
            .ok_or_else(|| anyhow!("Binary index out of bounds"))?;
        let def_feat = if bin.no_default_features { "No" } else { "Yes" };

        let remove_label = "Remove Binary".red().to_string();
        let done_label = "Done".cyan().to_string();

        let field = Select::new(
            &format!("Editing Binary: {}", bin.name.yellow()),
            vec![
                format!("Name: {}", bin.name),
                format!("Path: {}", bin.path),
                format!(
                    "Output Name: {}",
                    bin.out_name.as_deref().unwrap_or("Default")
                ),
                format!("Features: {:?}", bin.features),
                format!("Default Features: {}", def_feat),
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
            if prompt_confirm(&format!("Delete {}?", bin.name), false)? {
                bins.remove(idx);
                return Ok(true);
            }
            continue;
        }

        let bin_mut = bins
            .get_mut(idx)
            .ok_or_else(|| anyhow!("Failed to get mutable binary"))?;
        if field.starts_with("Name:") {
            let new = prompt!("New name:", &format!("Current: {}", bin_mut.name))?;
            if !new.trim().is_empty() {
                bin_mut.name = new.trim().to_string();
            }
        } else if field.starts_with("Path:") {
            let new = prompt!("New path:", &format!("Current: {}", bin_mut.path))?;
            if !new.trim().is_empty() {
                bin_mut.path = new.trim().to_string();
            }
        } else if field.starts_with("Output Name:") {
            let cur = bin_mut.out_name.clone().unwrap_or_default();
            let new = prompt!(
                "New output name (empty for default):",
                &format!("Current: {cur}")
            )?;
            bin_mut.out_name = if new.trim().is_empty() {
                None
            } else {
                Some(new.trim().to_string())
            };
        } else if field.starts_with("Features:") {
            let cur = bin_mut.features.join(",");
            let f = prompt!("Features (csv):", &format!("Current: {cur}"))?;
            bin_mut.features = f
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if field.starts_with("Default Features:") {
            bin_mut.no_default_features = !bin_mut.no_default_features;
        }
    }
}

/// # Errors
/// Returns error if prompt fails.
pub fn configure_binaries(config: &mut RefineryConfig, default_name: &str) -> Result<()> {
    println!();
    loop {
        let name = loop {
            let input = prompt!(
                &format!("Binary name (default: {default_name}, '!' to go back)"),
                "The name of your executable"
            )?;
            if input == "!" {
                return Ok(());
            }
            let n = if input.is_empty() {
                default_name.to_string()
            } else {
                input
            };
            if config.is_name_taken(&n) {
                warn(&format!("Name '{n}' is already taken by another artifact."));
                continue;
            }
            break n;
        };

        let path_input = prompt!(
            &format!("Path for {name} (default: src/main.rs)"),
            "Relative to project root"
        )?;
        let path = if path_input.is_empty() {
            "src/main.rs".to_string()
        } else {
            path_input
        };

        let no_default_features =
            prompt_confirm(&format!("Disable default features for {name}?"), false)?;

        let features = if prompt_confirm(&format!("Add specific features for {name}?"), false)? {
            prompt!(&format!("Features for {name}:"), "comma separated list")?
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        } else {
            Vec::new()
        };

        config.binaries.push(Binary {
            name,
            path,
            out_name: None,
            features,
            no_default_features,
        });

        println!();
        if !prompt_confirm("Add another binary?", false)? {
            break;
        }
    }
    Ok(())
}
