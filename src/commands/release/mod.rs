// @swt-disable max-repetition
use clap::{Args, Subcommand};

use crate::auto_dispatch;
use crate::cmd;

#[derive(Args)]
pub struct ReleaseArgs {
    #[command(subcommand)]
    pub action: ReleaseAction,
}

#[derive(Subcommand)]
pub enum ReleaseAction {
    Major,
    Minor,
    Patch,
    PreRelease,
}

pub mod major;
pub mod minor;
pub mod patch;
pub mod pre_release;

cmd!(release(args: ReleaseArgs) {
    auto_dispatch!(args.action, ReleaseAction, {
        Major,
        Minor,
        Patch,
        PreRelease,
    })
});
