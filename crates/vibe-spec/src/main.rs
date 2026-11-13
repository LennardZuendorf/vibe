//! Standalone argv/stdio adapter for the embedded spec library.
use std::process::ExitCode;
fn main() -> ExitCode {
    ExitCode::from(vibe_spec::run_process(
        &std::env::args_os().collect::<Vec<_>>(),
    ))
}
