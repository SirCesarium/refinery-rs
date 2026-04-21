use crate::core::schema::{Arch, Binary, LibC, Library, OS, TargetMatrix, Targets};
use crate::ui::{Result, get_render_config, prompt_confirm, prompt_opt};
use inquire::MultiSelect;
use inquire::list_option::ListOption;
use inquire::validator::Validation;
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

        let existing_matrix = if os == OS::Windows {
            targets.windows.as_ref()
        } else if os == OS::Macos {
            targets.macos.as_ref()
        } else {
            None
        };

        let libc_options = if os == OS::Linux {
            let opts = vec![LibC::Gnu, LibC::Musl];
            let mut defaults = Vec::new();
            if let Some(ref l) = targets.linux {
                if l.gnu.is_some() {
                    defaults.push(0);
                }
                if l.musl.is_some() {
                    defaults.push(1);
                }
            }
            MultiSelect::new("Select LibC variants:", opts)
                .with_default(&defaults)
                .with_render_config(get_render_config())
                .with_validator(|ans: &[ListOption<&LibC>]| {
                    if ans.is_empty() {
                        Ok(Validation::Invalid(
                            "At least one LibC variant must be selected for Linux".into(),
                        ))
                    } else {
                        Ok(Validation::Valid)
                    }
                })
                .prompt()?
        } else {
            vec![LibC::Gnu]
        };

        let arch_opts = if os == OS::Macos {
            vec![Arch::X86_64, Arch::Aarch64]
        } else {
            vec![Arch::X86_64, Arch::Aarch64, Arch::I686]
        };
        let mut arch_defaults = Vec::new();
        if let Some(m) = existing_matrix {
            arch_defaults = arch_opts
                .iter()
                .enumerate()
                .filter(|(_, a)| m.archs.contains(a))
                .map(|(i, _)| i)
                .collect();
        } else if os == OS::Linux
            && let Some(ref l) = targets.linux
        {
            let m = l.gnu.as_ref().or(l.musl.as_ref());
            if let Some(matrix) = m {
                arch_defaults = arch_opts
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| matrix.archs.contains(a))
                    .map(|(i, _)| i)
                    .collect();
            }
        }

        let archs = MultiSelect::new("Select Architectures:", arch_opts)
            .with_default(&arch_defaults)
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

        if !archs.is_empty() {
            let pkg_opts = match os {
                OS::Linux => vec!["deb", "rpm", "tar.gz"],
                OS::Windows => vec!["msi", "zip"],
                OS::Macos => vec!["dmg", "pkg", "tar.gz"],
            };
            let mut pkg_defaults = Vec::new();
            if let Some(m) = existing_matrix {
                pkg_defaults = pkg_opts
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| m.pkg.contains(&(*p).to_string()))
                    .map(|(i, _)| i)
                    .collect();
            } else if os == OS::Linux
                && let Some(ref l) = targets.linux
            {
                let m = l.gnu.as_ref().or(l.musl.as_ref());
                if let Some(matrix) = m {
                    pkg_defaults = pkg_opts
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| matrix.pkg.contains(&(*p).to_string()))
                        .map(|(i, _)| i)
                        .collect();
                }
            }

            let pkg: Vec<String> =
                MultiSelect::new(&format!("Select packages for {os}:"), pkg_opts)
                    .with_default(&pkg_defaults)
                    .with_render_config(get_render_config())
                    .with_validator(|ans: &[ListOption<&&str>]| {
                        if ans.is_empty() {
                            Ok(Validation::Invalid(
                                "Select at least one package format".into(),
                            ))
                        } else {
                            Ok(Validation::Valid)
                        }
                    })
                    .prompt()?
                    .into_iter()
                    .map(String::from)
                    .collect();

            let strip_default = existing_matrix.map_or_else(
                || {
                    if os == OS::Linux {
                        targets
                            .linux
                            .as_ref()
                            .and_then(|l| l.gnu.as_ref().or(l.musl.as_ref()))
                            .is_some_and(|m| m.strip)
                    } else {
                        false
                    }
                },
                |m| m.strip,
            );

            let strip = prompt_confirm("Strip symbols from binaries?", strip_default)?;
            let artifacts: Vec<String> = binaries
                .iter()
                .map(|b| b.name.clone())
                .chain(libraries.iter().map(|l| l.name.clone()))
                .collect();

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
        }
        if !prompt_confirm("Add/Modify another OS target?", false)? {
            break;
        }
    }
    Ok(())
}
