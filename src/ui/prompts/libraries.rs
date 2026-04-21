use crate::core::schema::{LibType, Library, RefineryConfig};
use crate::prompt;
use crate::ui::{Result, get_render_config, prompt_confirm, warn};
use anyhow::anyhow;
use inquire::{MultiSelect, Select};
use owo_colors::OwoColorize;

/// # Errors
/// Returns error if prompt fails.
pub fn edit_libraries(config: &mut RefineryConfig) -> Result<bool> {
    let mut changed = false;
    loop {
        let mut options = vec![format!("{} Add New Library", "+".green())];
        options.extend(config.libraries.iter().map(|l| l.name.clone()));
        options.push("Back".dimmed().to_string());

        let choice = Select::new("Manage Libraries", options)
            .with_render_config(get_render_config())
            .prompt()?;

        if choice.contains("Back") {
            break;
        }

        if choice.contains("Add New Library") {
            let name = prompt!("Name:", "The name of your library artifact")?;
            if name.trim().is_empty() {
                continue;
            }
            let path_input = prompt!("Path (default: src/lib.rs):", "Source file location")?;
            let path = if path_input.trim().is_empty() {
                "src/lib.rs".to_string()
            } else {
                path_input.trim().to_string()
            };

            config.libraries.push(Library {
                name: name.trim().to_string(),
                path,
                types: vec![LibType::Static],
                ..Default::default()
            });
            changed = true;
        } else {
            let idx = config
                .libraries
                .iter()
                .position(|l| l.name == choice)
                .ok_or_else(|| anyhow!("Library not found"))?;

            if edit_library_fields(&mut config.libraries, idx)? {
                changed = true;
            }
        }
    }
    Ok(changed)
}

/// # Errors
/// Returns error if prompt fails.
pub fn edit_library_fields(libs: &mut Vec<Library>, idx: usize) -> Result<bool> {
    loop {
        let lib = libs
            .get(idx)
            .ok_or_else(|| anyhow!("Library index out of bounds"))?;
        let headers = if lib.headers { "Yes" } else { "No" };

        let remove_label = "Remove Library".red().to_string();
        let done_label = "Done".cyan().to_string();

        let field = Select::new(
            &format!("Editing Library: {}", lib.name.yellow()),
            vec![
                format!("Name: {}", lib.name),
                format!("Path: {}", lib.path),
                format!("Types: {:?}", lib.types),
                format!("Headers: {}", headers),
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
            if prompt_confirm(&format!("Delete {}?", lib.name), false)? {
                libs.remove(idx);
                return Ok(true);
            }
            continue;
        }

        let lib_mut = libs
            .get_mut(idx)
            .ok_or_else(|| anyhow!("Failed to get mutable library"))?;
        if field.starts_with("Name:") {
            let new = prompt!("New name:", &format!("Current: {}", lib_mut.name))?;
            if !new.trim().is_empty() {
                lib_mut.name = new.trim().to_string();
            }
        } else if field.starts_with("Path:") {
            let new = prompt!("New path:", &format!("Current: {}", lib_mut.path))?;
            if !new.trim().is_empty() {
                lib_mut.path = new.trim().to_string();
            }
        } else if field.starts_with("Types:") {
            let options = vec![LibType::Static, LibType::Dynamic];
            let defaults: Vec<usize> = options
                .iter()
                .enumerate()
                .filter(|(_, t)| lib_mut.types.contains(t))
                .map(|(i, _)| i)
                .collect();
            lib_mut.types = MultiSelect::new("Types:", options)
                .with_default(&defaults)
                .with_render_config(get_render_config())
                .prompt()?;
        } else if field.starts_with("Headers:") {
            lib_mut.headers = !lib_mut.headers;
        }
    }
}

/// # Errors
/// Returns error if prompt fails.
pub fn configure_libraries(config: &mut RefineryConfig, default_name: &str) -> Result<()> {
    println!();
    loop {
        let name = loop {
            let input = prompt!(
                &format!("Library name (default: {default_name}, '!' to go back)"),
                "The name of your crate"
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
            &format!("Path for {name} (default: src/lib.rs)"),
            "Relative to project root"
        )?;
        let path = if path_input.is_empty() {
            "src/lib.rs".to_string()
        } else {
            path_input
        };

        let types = MultiSelect::new("Library types:", vec![LibType::Static, LibType::Dynamic])
            .with_render_config(get_render_config())
            .prompt()?;

        let headers = prompt_confirm("Generate C headers?", false)?;

        config.libraries.push(Library {
            name,
            path,
            out_name: None,
            types,
            headers,
        });

        println!();
        if !prompt_confirm("Add another library?", false)? {
            break;
        }
    }
    Ok(())
}
