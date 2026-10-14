//! Test candidate for the parity harness. Reproduces
//! `tests/fixture/oracle/echo.sh`; `--mutate=<exit|stdout|file>` makes it
//! diverge on purpose so tests can prove the harness turns red.
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut mutate = String::new();
    let mut args = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.strip_prefix("--mutate=") {
            Some(m) => mutate = m.to_string(),
            None => args.push(arg),
        }
    }
    let mark = if mutate == "stdout" { "!" } else { "" };
    println!("args:{}{mark}", args.join(" "));
    let body = if mutate == "file" { "y\n" } else { "x\n" };
    if std::fs::write("out.txt", body).is_err() {
        return ExitCode::from(1);
    }
    ExitCode::from(if mutate == "exit" { 3 } else { 0 })
}
