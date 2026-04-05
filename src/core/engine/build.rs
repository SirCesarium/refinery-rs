use crate::core::schema::refinery::{Artifact, RefineryConfig, Target};
use crate::errors::{RefineryError, Result};
use futures::future::try_join_all;
use std::env::consts::OS;
use std::process::Stdio;
use tokio::process::Command;

pub struct BuildEngine {
    config: RefineryConfig,
}

impl BuildEngine {
    pub const fn new(config: RefineryConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> Result<()> {
        let mut tasks = Vec::new();

        for artifact in &self.config.build.artifacts {
            for target in &artifact.targets {
                tasks.push(self.compile_target(artifact, target.clone()));
            }
        }

        try_join_all(tasks).await?;
        Ok(())
    }

    async fn compile_target(&self, artifact: &Artifact, target: Target) -> Result<()> {
        let triple = target
            .to_triple()
            .map_err(|e| RefineryError::Config(e.to_string()))?;

        println!("::group::Building {} for {}", artifact.name, triple);

        let is_macos_target = triple.contains("apple-darwin");
        let is_windows_gnu = triple.contains("windows-gnu");
        let is_linux_x64 = triple.contains("x86_64-unknown-linux");
        let is_macos_host = OS == "macos";

        let bin = if (is_macos_target && is_macos_host)
            || (is_windows_gnu && !is_macos_host)
            || is_linux_x64
        {
            "cargo"
        } else {
            "cross"
        };

        let mut cmd = Command::new(bin);
        cmd.arg("build").args(["--release", "--target", &triple]);

        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        if is_windows_gnu {
            cmd.env(
                "RUSTFLAGS",
                "-C link-arg=-lws2_32 -C link-arg=-luser32 -C link-arg=-lntdll",
            );
        }

        if !artifact.features.is_empty() {
            cmd.arg("--features").arg(artifact.features.join(","));
        }

        if !artifact.default_features {
            cmd.arg("--no-default-features");
        }

        let status = cmd.status().await.map_err(RefineryError::Io)?;

        println!("::endgroup::");

        if status.success() {
            println!("✅ SUCCESS: {} -> {}", artifact.name, triple);
            Ok(())
        } else {
            println!("❌ FAILED: {} -> {}", artifact.name, triple);
            Err(RefineryError::Generic(format!(
                "Build failed for {} on {}. Check the logs above.",
                artifact.name, triple
            )))
        }
    }
}
