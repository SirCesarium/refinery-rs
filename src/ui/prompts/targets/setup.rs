use crate::core::schema::{Arch, Binary, LibC, Library, OS, TargetMatrix, Targets};
use crate::ui::{Result, get_render_config, prompt_confirm, prompt_opt};
use inquire::MultiSelect;
use std::iter::once;

/// # Errors
/// Returns error if prompt fails.
#[allow(clippy::too_many_lines)]
pub fn configure_targets(
    targets: &mut Targets,
    binaries: &[Binary],
    libraries: &[Library],
) -> Result<()> {
    println!();
    let available_os = [OS::Linux, OS::Windows, OS::Macos];

    loop {
        let options: Vec<String> = available_os
            .iter()
            .map(ToString::to_string)
            .chain(once("Back".to_string()))
            .collect();

        let selection = prompt_opt("Select Target OS:", options)?;
        if selection == "Back" {
            break;
        }

        let os = match selection.as_str() {
            "linux" => OS::Linux,
            "windows" => OS::Windows,
            "macos" => OS::Macos,
            _ => break,
        };

        let existing_matrix = match os {
            OS::Windows => targets.windows.as_ref(),
            OS::Macos => targets.macos.as_ref(),
            OS::Linux => targets
                .linux
                .as_ref()
                .and_then(|l| l.gnu.as_ref().or(l.musl.as_ref())),
        };

        let libc_options = if os == OS::Linux {
            let opts = vec![LibC::Gnu, LibC::Musl];
            let defaults = targets.linux.as_ref().map_or_else(Vec::new, |l| {
                let mut d = Vec::new();
                if l.gnu.is_some() {
                    d.push(0);
                }
                if l.musl.is_some() {
                    d.push(1);
                }
                d
            });
            MultiSelect::new("Select LibC variants:", opts)
                .with_default(&defaults)
                .with_render_config(get_render_config())
                .prompt()?
        } else {
            vec![LibC::Gnu]
        };

        let arch_opts = if os == OS::Macos {
            vec![Arch::X86_64, Arch::Aarch64]
        } else {
            vec![Arch::X86_64, Arch::Aarch64, Arch::I686]
        };

        let arch_defaults: Vec<usize> = existing_matrix.map_or_else(Vec::new, |m| {
            arch_opts
                .iter()
                .enumerate()
                .filter(|(_, a)| m.archs.contains(a))
                .map(|(i, _)| i)
                .collect()
        });

        let archs = MultiSelect::new("Select Architectures:", arch_opts)
            .with_default(&arch_defaults)
            .with_render_config(get_render_config())
            .prompt()?;

        if archs.is_empty() {
            println!("No architectures selected. Skipping target.");
            continue;
        }

        let artifact_opts: Vec<String> = binaries
            .iter()
            .map(|b| b.name.clone())
            .chain(libraries.iter().map(|l| l.name.clone()))
            .collect();

        let artifact_defaults: Vec<usize> = existing_matrix.map_or_else(
            || (0..artifact_opts.len()).collect(),
            |m| {
                artifact_opts
                    .iter()
                    .enumerate()
                    .filter(|(_, name)| m.artifacts.contains(name))
                    .map(|(i, _)| i)
                    .collect()
            },
        );

        let artifacts =
            MultiSelect::new("Select artifacts to include in this target:", artifact_opts)
                .with_default(&artifact_defaults)
                .with_render_config(get_render_config())
                .prompt()?;

        if artifacts.is_empty() {
            println!("No artifacts selected for this target. Skipping OS configuration.");
            continue;
        }

        let has_binaries = binaries.iter().any(|b| artifacts.contains(&b.name));

        let pkg = if has_binaries {
            let pkg_opts = match os {
                OS::Linux => vec!["deb", "rpm", "tar.gz"],
                OS::Windows => vec!["msi", "zip"],
                OS::Macos => vec!["dmg", "pkg", "tar.gz"],
            };
            let pkg_defaults: Vec<usize> = existing_matrix.map_or_else(Vec::new, |m| {
                pkg_opts
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| m.pkg.contains(&(*p).to_string()))
                    .map(|(i, _)| i)
                    .collect()
            });

            MultiSelect::new(&format!("Select packages for {os}:"), pkg_opts)
                .with_default(&pkg_defaults)
                .with_render_config(get_render_config())
                .prompt()?
                .into_iter()
                .map(String::from)
                .collect()
        } else {
            Vec::new()
        };

        let strip = if has_binaries {
            let strip_default = existing_matrix.is_some_and(|m| m.strip);
            prompt_confirm("Strip symbols from binaries?", strip_default)?
        } else {
            false
        };

        for libc in libc_options {
            let matrix = TargetMatrix {
                archs: archs.clone(),
                artifacts: artifacts.clone(),
                pkg: pkg.clone(),
                ext: if os == OS::Windows {
                    Some(".exe".into())
                } else {
                    None
                },
                strip,
                ..Default::default()
            };
            match (os, libc) {
                (OS::Linux, LibC::Gnu) => {
                    let mut l = targets.linux.take().unwrap_or_default();
                    l.gnu = Some(matrix);
                    targets.linux = Some(l);
                }
                (OS::Linux, LibC::Musl) => {
                    let mut l = targets.linux.take().unwrap_or_default();
                    l.musl = Some(matrix);
                    targets.linux = Some(l);
                }
                (OS::Windows, _) => targets.windows = Some(matrix),
                (OS::Macos, _) => targets.macos = Some(matrix),
            }
        }

        if !prompt_confirm("Add/Modify another OS target?", false)? {
            break;
        }
    }
    Ok(())
}
