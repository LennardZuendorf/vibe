//! `vibe-flow`'s entry point: parse, run, write the streams, map the code.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let (code, stdout, stderr) = vibe_flow::cli::run(&cwd, &argv, &vibe_flow::paths::Process);
    let _ = std::io::stdout().write_all(&stdout);
    let _ = std::io::stderr().write_all(&stderr);
    ExitCode::from(code)
}
