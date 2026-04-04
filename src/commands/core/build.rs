use crate::{
    commands::core::CoreAction,
    core::schema::refinery::{Arch, Os, Target},
    subcmd,
};
use clap::Args;

#[derive(Args)]
pub struct BuildArgs {
    #[arg(short, long, value_enum)]
    pub os: Os,

    #[arg(short, long, value_enum)]
    pub arch: Arch,
}

subcmd!(CoreAction, build(args: BuildArgs) {
    let target = Target {
        os: args.os,
        arch: args.arch,
    }.to_triple()?;

    print!("{target}");

    Ok(())
});
