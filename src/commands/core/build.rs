use crate::{
    commands::core::CoreAction,
    core::{
        engine::build::BuildEngine,
        schema::refinery::{Arch, Artifact, Build, LibraryFormats, Os, RefineryConfig, Target},
    },
    errors::RefineryError,
    subcmd, ui,
};
use clap::Args;

#[derive(Args)]
pub struct BuildArgs {
    #[arg(short, long, value_enum)]
    pub os: Option<Os>,

    #[arg(short, long, value_enum)]
    pub arch: Option<Arch>,
}

subcmd!(CoreAction, build(args: BuildArgs) {
    let mut config = RefineryConfig::load().await.unwrap_or_else(|_| RefineryConfig {
        build: Build {
            artifacts: vec![],
            library: LibraryFormats::default(),
        },
    });

    if let (Some(os), Some(arch)) = (args.os, args.arch) {
        config.build.artifacts = vec![Artifact {
            name: "artifact".to_string(),
            targets: vec![Target { os, arch }],
            features: Vec::new(),
            default_features: true,
        }];
    }

    if config.build.artifacts.is_empty() {
        return Err(RefineryError::Generic(
            "No build targets specified via CLI flags or refinery.toml".into()
        ).into());
    }

    ui::info("Refining artifacts...");
    let engine = BuildEngine::new(config);
    engine.run().await?;
    ui::success("Build complete.");

    Ok(())
});
