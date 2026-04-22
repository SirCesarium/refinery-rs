pub mod actions;
pub mod jobs;

use crate::core::schema::RefineryConfig;
use crate::errors::Result as RefineryResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub on: WorkflowEvents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    pub jobs: HashMap<String, Job>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WorkflowEvents {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push: Option<PushEvent>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "pull_request")]
    pub pull_request: Option<PullRequestEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<ReleaseEvent>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PushEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branches: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PullRequestEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branches: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ReleaseEvent {
    pub types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub name: String,
    #[serde(rename = "runs-on")]
    pub runs_on: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "if")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<Strategy>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Strategy {
    #[serde(rename = "fail-fast")]
    pub fail_fast: bool,
    pub matrix: Matrix,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Matrix {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<HashMap<String, String>>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Step {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "if")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
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
            jobs: HashMap::new(),
        }
    }

    /// # Errors
    /// Returns an error if job collection fails.
    pub fn primary_workflow(config: &RefineryConfig) -> RefineryResult<Self> {
        let mut workflow = Self::new("Refinery Pipeline");

        let mut perms = HashMap::new();
        perms.insert("contents".into(), "write".into());
        perms.insert("pull-requests".into(), "write".into());
        workflow.permissions = Some(perms);

        let mut jobs = HashMap::new();

        // 1. Preparation Job (Headers)
        if config.libraries.iter().any(|l| l.headers) {
            let prep_steps = vec![
                Step { name: Some("Checkout".into()), uses: Some(actions::CHECKOUT.into()), ..Default::default() },
                Step { name: Some("Install Toolchain".into()), uses: Some(actions::RUST_TOOLCHAIN.into()), ..Default::default() },
                Step { name: Some("Install Refinery".into()), run: Some("cargo install --git https://github.com/SirCesarium/refinery-rs --no-default-features".into()), shell: Some("bash".into()), ..Default::default() },
                Step { name: Some("Install cbindgen".into()), run: Some("cargo install cbindgen".into()), shell: Some("bash".into()), ..Default::default() },
                Step { name: Some("Generate Headers".into()), run: Some("refinery build --headers-only".into()), shell: Some("bash".into()), ..Default::default() },
                Step {
                    name: Some("Upload Headers".into()),
                    uses: Some("actions/upload-artifact@v4".into()),
                    with: Some({
                        let mut m = HashMap::new();
                        m.insert("name".into(), "headers".into());
                        m.insert("path".into(), "*.h".into());
                        m
                    }),
                    ..Default::default()
                },
            ];
            jobs.insert(
                "prepare".into(),
                Job {
                    name: "Prepare Assets".into(),
                    runs_on: "ubuntu-latest".into(),
                    needs: None,
                    condition: None,
                    strategy: None,
                    steps: prep_steps,
                },
            );
        }

        // 2. Build Job
        let mut build_job = jobs::create_matrix_job(config)?;
        if jobs.contains_key("prepare") {
            build_job.needs = Some(vec!["prepare".into()]);
            // Add download step to build job
            let download_step = Step {
                name: Some("Download Headers".into()),
                uses: Some("actions/download-artifact@v4".into()),
                with: Some({
                    let mut m = HashMap::new();
                    m.insert("name".into(), "headers".into());
                    m
                }),
                ..Default::default()
            };
            build_job.steps.insert(1, download_step);
        }
        jobs.insert("build".into(), build_job);

        // 3. Release Job
        let release_steps = vec![
            Step {
                name: Some("Checkout".into()),
                uses: Some(actions::CHECKOUT.into()),
                ..Default::default()
            },
            Step {
                name: Some("Download Artifacts".into()),
                uses: Some("actions/download-artifact@v4".into()),
                with: Some({
                    let mut m = HashMap::new();
                    m.insert("path".into(), "artifacts".into());
                    m.insert("merge-multiple".into(), "true".into());
                    m
                }),
                ..Default::default()
            },
            Step {
                name: Some("Publish Release".into()),
                uses: Some(actions::SOFTPROPS_RELEASE.into()),
                with: Some({
                    let mut m = HashMap::new();
                    m.insert("files".into(), "artifacts/*".into());
                    m
                }),
                ..Default::default()
            },
        ];

        jobs.insert(
            "release".into(),
            Job {
                name: "Release Artifacts".into(),
                runs_on: "ubuntu-latest".into(),
                needs: Some(vec!["build".into()]),
                condition: None,
                strategy: None,
                steps: release_steps,
            },
        );

        workflow.jobs = jobs;
        Ok(workflow)
    }

    /// Serializes the workflow to YAML.
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}
