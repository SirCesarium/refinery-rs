use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowJobs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepare: Option<Job>,
    pub build: Job,
    pub release: Job,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WorkflowEvents {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push: Option<PushEvent>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "pull_request")]
    pub pull_request: Option<PullRequestEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<ReleaseEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_dispatch: Option<serde_yaml::Value>,
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

#[derive(Debug, Serialize, Deserialize, Default)]
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
