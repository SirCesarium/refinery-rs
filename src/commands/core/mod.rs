// @swt-disable max-repetition
use clap::{Args, Subcommand};

use crate::auto_dispatch;
use crate::cmd;
use crate::commands::core::build::BuildArgs;

#[derive(Args)]
pub struct CoreArgs {
    #[command(subcommand)]
    pub action: CoreAction,
}

#[derive(Subcommand)]
pub enum CoreAction {
    Build(BuildArgs),
}

pub mod build;

cmd!(core(args: CoreArgs) {
    auto_dispatch!(args.action, CoreAction, {
        Build(args)
    })
});
