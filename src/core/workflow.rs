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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowEvents {
    Push {
        #[serde(skip_serializing_if = "Option::is_none")]
        branches: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tags: Option<Vec<String>>,
    },
    PullRequest {
        #[serde(skip_serializing_if = "Option::is_none")]
        branches: Option<Vec<String>>,
    },
    Release {
        types: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub name: String,
    #[serde(rename = "runs-on")]
    pub runs_on: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<Strategy>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Strategy {
    #[serde(rename = "fail-fast")]
    pub fail_fast: bool,
    pub matrix: HashMap<String, Vec<String>>,
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
}

impl Workflow {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            on: WorkflowEvents::Push {
                branches: Some(vec!["main".into()]),
                tags: None,
            },
            permissions: None,
            env: None,
            jobs: HashMap::new(),
        }
    }

    /// Serializes the workflow structure into a YAML-formatted string.
    ///
    /// This method converts the internal representation of the GitHub Actions workflow
    /// into a valid YAML string that can be written directly to a `.yml` file.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_yaml::Error`] if the structure contains data that cannot be
    /// represented in YAML format.
    ///
    /// # Examples
    ///
    /// ```
    /// # use refinery_rs::core::workflow::Workflow;
    /// let workflow = Workflow::new("CI");
    /// let yaml = workflow.to_yaml().expect("Failed to generate YAML");
    /// assert!(yaml.contains("name: CI"));
    /// ```
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}
