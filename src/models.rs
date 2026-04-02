//! Data models and configurations for Refinery-RS.

/// Configuration for the CI workflow.
#[derive(Debug, Default, Clone)]
pub struct CiConfig {
    pub enable_sweet: bool,
    pub enable_clippy: bool,
    pub enable_fmt: bool,
}

/// Configuration for an individual binary.
#[derive(Debug, Default, Clone)]
pub struct BinaryConfig {
    pub name: String,
    pub features: String,
    pub targets: Vec<String>,
    pub export_libs: bool,
    pub enable_packaging: bool,
}

/// Features to enable in the release workflow.
#[derive(Debug, Default, Clone)]
pub struct ReleaseFeatures {
    pub publish_docker: bool,
    pub publish_crates: bool,
}

/// Configuration for the Release workflow.
#[derive(Debug, Default, Clone)]
pub struct ReleaseConfig {
    pub binaries: Vec<BinaryConfig>,
    pub features: ReleaseFeatures,
}
