//! `vibe-spec`: the spec tool. F16 ports the read-only spec scripts and adds
//! root discovery and the Spec JSON v1 queries.
// F16 in progress: callers arrive in Tasks 3–7; Task 7 removes this.
#![allow(dead_code, unused_macros)]
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use vibe_core::exit;

/// Concatenates byte-slice-like values (`&str`, `&[u8]`, `Vec<u8>`, `String`) into one `Vec<u8>`.
macro_rules! cat {
    ($($part:expr),* $(,)?) => {{
        let mut v: Vec<u8> = Vec::new();
        $(v.extend_from_slice(AsRef::<[u8]>::as_ref(&$part));)*
        v
    }};
}

mod lessons;
mod list_specs;
mod root;
mod text;

/// The vibe spec tool.
#[derive(Parser)]
#[command(name = "vibe-spec", version, about, arg_required_else_help = true)]
struct Cli {
    /// Spec root directory; skips discovery. Goes before the command.
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the spec root and how it was found.
    Root {
        /// Print Spec JSON v1.
        #[arg(long)]
        json: bool,
    },
    /// List spec files (port of list-specs.sh).
    #[command(disable_help_flag = true)]
    ListSpecs {
        /// Ignored, as the oracle ignores them.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Lessons matching tags (port of lessons-for.sh; `--json` for Spec JSON v1).
    #[command(disable_help_flag = true)]
    LessonsFor {
        /// Tags, `--format <markdown|inject|json>`, `--tag <t>`, `--json`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("vibe-spec: cannot read the current directory: {e}");
            return ExitCode::from(exit::USAGE);
        }
    };
    if std::env::var_os("SPEC_DIR").is_some() {
        eprintln!("vibe-spec: SPEC_DIR is ignored; set VIBE_SPEC_ROOT instead");
    }
    let env_root = std::env::var_os("VIBE_SPEC_ROOT");
    let found = root::discover(&cwd, cli.root.as_deref(), env_root.as_deref());
    let (code, out) = match cli.command {
        Command::Root { json } => root::command(&found, json),
        Command::ListSpecs { args: _ } => list_specs::run(&found),
        Command::LessonsFor { args } => lessons::run(&found, &args),
    };
    emit(&out);
    ExitCode::from(code)
}

/// Writes `bytes` to stdout. A closed pipe is not an error.
fn emit(bytes: &[u8]) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(bytes);
    let _ = out.flush();
}
