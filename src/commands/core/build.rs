use crate::{commands::core::CoreAction, subcmd};
use clap::{Args, ValueEnum};
use miette::miette;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Os {
    Linux,
    Windows,
    Macos,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Arch {
    #[value(name = "x64")]
    X64,
    #[value(name = "x86")]
    X86,
    #[value(name = "arm64")]
    Arm64,
}

#[derive(Args)]
pub struct BuildArgs {
    #[arg(short, long, value_enum)]
    pub os: Os,

    #[arg(short, long, value_enum)]
    pub arch: Arch,
}

subcmd!(CoreAction, build(args: BuildArgs) {
    let target_triple = match (args.os, args.arch) {
        // --- LINUX ---
        (Os::Linux, Arch::X64) => "x86_64-unknown-linux-musl",
        (Os::Linux, Arch::X86) => "i686-unknown-linux-musl",
        (Os::Linux, Arch::Arm64) => "aarch64-unknown-linux-musl",

        // --- WINDOWS ---
        (Os::Windows, Arch::X64) => "x86_64-pc-windows-msvc",
        (Os::Windows, Arch::X86) => "i686-pc-windows-msvc",
        (Os::Windows, Arch::Arm64) => "aarch64-pc-windows-msvc",

        // --- MACOS ---
        (Os::Macos, Arch::Arm64) => "aarch64-apple-darwin", // Apple Silicon
        (Os::Macos, Arch::X64) => "x86_64-apple-darwin",    // Intel Mac

        // --- Invariants / Unsupported ---
        (Os::Macos, Arch::X86) => {
            return Err(miette!("macOS doesn't support x86 architecture."));
        }
    };

    print!("{target_triple}");

    Ok(())
});
