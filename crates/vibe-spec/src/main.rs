//! `vibe-spec`: the spec tool. Commands arrive with F16–F20.
use std::process::ExitCode;

use clap::Parser;

/// The vibe spec tool.
#[derive(Parser)]
#[command(name = "vibe-spec", version, about, arg_required_else_help = true)]
struct Cli {}

fn main() -> ExitCode {
    let _cli = Cli::parse();
    ExitCode::from(vibe_core::exit::OK)
}
