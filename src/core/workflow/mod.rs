pub mod actions;
pub mod jobs;
pub mod types;

use crate::core::schema::RefineryConfig;
use crate::errors::Result as RefineryResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use types::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub on: WorkflowEvents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    pub jobs: WorkflowJobs,
}

impl Workflow {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            on: WorkflowEvents {
                push: Some(PushEvent {
                    branches: None,
                    tags: Some(vec!["v*".into()]),
                }),
                ..Default::default()
            },
            permissions: None,
            env: None,
            jobs: WorkflowJobs {
                prepare: None,
                build: Job::default(),
                release: Job::default(),
            },
        }
    }

    /// # Errors
    /// Returns an error if job collection fails.
    pub fn primary_workflow(config: &RefineryConfig) -> RefineryResult<Self> {
        let mut workflow = Self::new("Refinery Pipeline");
        workflow.permissions = Some(actions::default_permissions());

        let prepare = if config.libraries.iter().any(|l| l.headers) {
            Some(create_prepare_job())
        } else {
            None
        };

        let mut build = jobs::create_matrix_job(config)?;
        if prepare.is_some() {
            build.needs = Some(vec!["prepare".into()]);
            build.steps.insert(1, create_download_step("headers", "."));
        }

        workflow.jobs = WorkflowJobs {
            prepare,
            build,
            release: create_release_job(),
        };
        Ok(workflow)
    }

    /// Generates a Quality Gate workflow.
    #[must_use]
    pub fn quality_gate(checks: &[String], clippy_flags: &str) -> String {
        let mut steps = vec![
            "      - uses: actions/checkout@v6".to_string(),
            "      - uses: dtolnay/rust-toolchain@stable\n        with:\n          components: clippy, rustfmt".to_string(),
            "      - uses: Swatinem/rust-cache@v2".to_string(),
        ];

        if checks.iter().any(|c| c.contains("Sweet")) {
            steps.push(format!(
                "      - name: Sweet Analysis\n        run: curl -L {}/releases/download/{}/{} -o swt && chmod +x swt && ./swt",
                actions::SWEET_REPO,
                actions::SWEET_DEFAULT_VERSION,
                actions::SWEET_BINARY
            ));
        }

        if checks.iter().any(|c| c.contains("Format")) {
            steps.push("      - name: Check Format\n        run: cargo fmt --check".to_string());
        }

        if checks.iter().any(|c| c.contains("Clippy")) {
            steps.push(format!(
                "      - name: Clippy\n        run: cargo clippy {clippy_flags}"
            ));
        }

        if checks.iter().any(|c| c.contains("Tests")) {
            steps.push("      - name: Run Tests\n        run: cargo test".to_string());
        }

        format!(
            "name: Quality Gate
on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]
jobs:
  check:
    name: Quality & Testing
    runs-on: ubuntu-latest
    steps:
{}
",
            steps.join("\n")
        )
    }

    /// Serializes the workflow to YAML.
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

fn create_prepare_job() -> Job {
    Job {
        name: "Prepare Assets".into(),
        runs_on: "ubuntu-latest".into(),
        steps: vec![
            create_step("Checkout", Some(actions::CHECKOUT.into()), None, None),
            create_step(
                "Install Toolchain",
                Some(actions::RUST_TOOLCHAIN.into()),
                None,
                None,
            ),
            create_run_step(
                "Install Refinery",
                "cargo install --git https://github.com/SirCesarium/refinery-rs --no-default-features",
            ),
            create_run_step("Install cbindgen", "cargo install cbindgen"),
            create_run_step("Generate Headers", "refinery build --headers-only"),
            create_upload_step("headers", "*.h"),
        ],
        ..Default::default()
    }
}

fn create_release_job() -> Job {
    let mut m = HashMap::new();
    m.insert("files".into(), "artifacts/*".into());

    Job {
        name: "Release Artifacts".into(),
        runs_on: "ubuntu-latest".into(),
        needs: Some(vec!["build".into()]),
        steps: vec![
            create_step("Checkout", Some(actions::CHECKOUT.into()), None, None),
            create_download_step("artifacts", "artifacts"),
            create_step(
                "Publish Release",
                Some(actions::SOFTPROPS_RELEASE.into()),
                Some(m),
                None,
            ),
        ],
        ..Default::default()
    }
}

fn create_step(
    name: &str,
    uses: Option<String>,
    with: Option<HashMap<String, String>>,
    run: Option<String>,
) -> Step {
    Step {
        name: Some(name.into()),
        uses,
        shell: if run.is_some() {
            Some("bash".into())
        } else {
            None
        },
        run,
        with,
        ..Default::default()
    }
}

fn create_run_step(name: &str, cmd: &str) -> Step {
    create_step(name, None, None, Some(cmd.into()))
}

fn create_upload_step(name: &str, path: &str) -> Step {
    let mut m = HashMap::new();
    m.insert("name".into(), name.into());
    m.insert("path".into(), path.into());
    create_step(
        &format!("Upload {name}"),
        Some(actions::UPLOAD_ARTIFACT.into()),
        Some(m),
        None,
    )
}

fn create_download_step(name: &str, path: &str) -> Step {
    let mut m = HashMap::new();
    m.insert("name".into(), name.into());
    m.insert("path".into(), path.into());
    if name == "artifacts" {
        m.insert("merge-multiple".into(), "true".into());
    }
    create_step(
        &format!("Download {name}"),
        Some(actions::DOWNLOAD_ARTIFACT.into()),
        Some(m),
        None,
    )
}
