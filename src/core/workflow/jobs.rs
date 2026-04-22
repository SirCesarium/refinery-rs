use crate::core::schema::{LibC, OS, RefineryConfig, TargetMatrix};
use crate::core::workflow::{Job, Matrix, Step, Strategy, actions};
use crate::errors::Result;
use std::collections::HashMap;

/// Creates a matrix build job based on configuration.
///
/// # Errors
/// Returns an error if target triples cannot be resolved.
pub fn create_matrix_job(config: &RefineryConfig) -> Result<Job> {
    let mut include = Vec::new();

    if let Some(linux) = &config.targets.linux {
        if let Some(gnu) = &linux.gnu {
            add_targets(&mut include, gnu, OS::Linux, Some(LibC::Gnu))?;
        }
        if let Some(musl) = &linux.musl {
            add_targets(&mut include, musl, OS::Linux, Some(LibC::Musl))?;
        }
    }

    if let Some(windows) = &config.targets.windows {
        add_targets(&mut include, windows, OS::Windows, None)?;
    }

    if let Some(macos) = &config.targets.macos {
        add_targets(&mut include, macos, OS::Macos, None)?;
    }

    Ok(Job {
        name: "Build ${{ matrix.target }}".into(),
        runs_on: "${{ matrix.os }}".into(),
        needs: None,
        strategy: Some(Strategy {
            fail_fast: true,
            matrix: Matrix {
                include: Some(include),
            },
        }),
        steps: create_build_steps(config),
    })
}

fn add_targets(
    inc: &mut Vec<HashMap<String, String>>,
    matrix: &TargetMatrix,
    os: OS,
    libc: Option<LibC>,
) -> Result<()> {
    for triple in matrix.get_triples(os, libc)? {
        let mut m = HashMap::new();
        m.insert("target".into(), triple.clone());
        m.insert(
            "os".into(),
            match os {
                OS::Linux => "ubuntu-latest",
                OS::Windows => "windows-latest",
                OS::Macos => "macos-latest",
            }
            .into(),
        );
        let cross =
            os == OS::Linux && (libc == Some(LibC::Musl) || triple != "x86_64-unknown-linux-gnu");
        m.insert("use_cross".into(), cross.to_string());
        inc.push(m);
    }
    Ok(())
}

fn create_build_steps(config: &RefineryConfig) -> Vec<Step> {
    let mut steps = create_base_steps();

    for bin in &config.binaries {
        steps.push(create_prepare_step(&bin.name));
    }

    steps.push(create_upload_step());
    steps
}

fn create_prepare_step(name: &str) -> Step {
    let run = format!(
        "mkdir -p dist\n\
         if [ \"${{{{ runner.os }}}}\" = \"Windows\" ]; then\n  \
           find target -name \"{name}.exe\" -path \"*/release/*\" -exec cp {{}} dist/{name}-${{{{ matrix.target }}}}.exe \\; 2>/dev/null || true\n  \
           find target -name \"*.msi\" -exec cp {{}} dist/ \\; 2>/dev/null || true\n\
         else\n  \
           find target -name \"{name}\" -path \"*/release/*\" -not -name \"*.d\" -exec cp {{}} dist/{name}-${{{{ matrix.target }}}} \\; 2>/dev/null || true\n  \
           find target -name \"*.deb\" -exec cp {{}} dist/ \\; 2>/dev/null || true\n  \
           find target -name \"*.rpm\" -exec cp {{}} dist/ \\; 2>/dev/null || true\n\
         fi\n\
         ls -R dist/"
    );

    Step {
        name: Some(format!("Prepare Artifacts: {name}")),
        run: Some(run),
        shell: Some("bash".into()),
        ..Default::default()
    }
}

fn create_upload_step() -> Step {
    Step {
        name: Some("Upload Artifacts".into()),
        uses: Some("actions/upload-artifact@v4".into()),
        with: Some({
            let mut m = HashMap::new();
            m.insert("name".into(), "artifact-${{ matrix.target }}".into());
            m.insert("path".into(), "dist/*".into());
            m.insert("if-no-files-found".into(), "error".into());
            m
        }),
        ..Default::default()
    }
}

fn create_base_steps() -> Vec<Step> {
    vec![
        create_simple_step("Checkout", Some(actions::CHECKOUT.into()), None),
        create_toolchain_step(),
        create_simple_step("Rust Cache", Some(actions::RUST_CACHE.into()), None),
        create_linux_dep_step("i686-unknown-linux-gnu", "gcc-multilib libc6-dev-i386"),
        create_linux_dep_step("aarch64-unknown-linux-gnu", "gcc-aarch64-linux-gnu libc6-dev-arm64-cross"),
        create_linker_config_step(),
        create_packagers_step(),
        create_simple_step("Install Refinery", None, Some("cargo install --git https://github.com/SirCesarium/refinery-rs --no-default-features".into())),
        create_cross_step(),
        create_simple_step("Check Format", None, Some("cargo fmt --check".into())),
        create_simple_step("Clippy", None, Some("cargo clippy -- -D warnings".into())),
        create_simple_step("Build", None, Some("refinery build --target ${{ matrix.target }} --release".into())),
    ]
}

fn create_simple_step(name: &str, uses: Option<String>, run: Option<String>) -> Step {
    Step {
        name: Some(name.into()),
        uses,
        shell: if run.is_some() {
            Some("bash".into())
        } else {
            None
        },
        run,
        ..Default::default()
    }
}

fn create_linux_dep_step(target: &str, pkgs: &str) -> Step {
    Step {
        name: Some(format!("Deps: {target}")),
        condition: Some(format!("matrix.target == '{target}'")),
        run: Some(format!(
            "sudo apt-get update && sudo apt-get install -y {pkgs}"
        )),
        shell: Some("bash".into()),
        ..Default::default()
    }
}

fn create_toolchain_step() -> Step {
    Step {
        name: Some("Rust Toolchain".into()),
        uses: Some(actions::RUST_TOOLCHAIN.into()),
        with: Some({
            let mut m = HashMap::new();
            m.insert("targets".into(), "${{ matrix.target }}".into());
            m
        }),
        ..Default::default()
    }
}

fn create_packagers_step() -> Step {
    Step {
        name: Some("Install Packagers".into()),
        shell: Some("bash".into()),
        run: Some(
            r#"if [ "${{ runner.os }}" = "Windows" ]; then
  cargo install cargo-wix
else
  cargo install cargo-deb cargo-generate-rpm
fi"#
            .into(),
        ),
        ..Default::default()
    }
}

fn create_cross_step() -> Step {
    Step {
        name: Some("Install Cross".into()),
        condition: Some("${{ matrix.use_cross == 'true' }}".into()),
        run: Some("curl -L https://github.com/cross-rs/cross/releases/latest/download/cross-x86_64-unknown-linux-musl.tar.gz | tar xz -C /usr/local/bin".into()),
        shell: Some("bash".into()),
        ..Default::default()
    }
}

fn create_linker_config_step() -> Step {
    Step {
        name: Some("Linker Config".into()),
        shell: Some("bash".into()),
        run: Some("if [ \"${{ matrix.target }}\" = \"aarch64-unknown-linux-gnu\" ]; then echo \"CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc\" >> $GITHUB_ENV; fi\nif [ \"${{ matrix.target }}\" = \"i686-unknown-linux-gnu\" ]; then echo \"LIBRARY_PATH=/usr/lib32\" >> $GITHUB_ENV; echo \"LD_LIBRARY_PATH=/usr/lib32\" >> $GITHUB_ENV; fi".into()),
        ..Default::default()
    }
}
