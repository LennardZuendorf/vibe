//! Test candidate for the parity harness. Reproduces
//! `tests/fixture/oracle/echo.sh`; `--mutate=<exit|stdout|file|dir|hang|path|updated>`
//! makes it diverge on purpose so tests can prove the harness turns red.
use std::process::ExitCode;

const BODY: &str = "{\"updated\": \"2026-01-01T00:00:00Z\"}\n";

fn main() -> ExitCode {
    let mut mutate = String::new();
    let mut args = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.strip_prefix("--mutate=") {
            Some(m) => mutate = m.to_string(),
            None => args.push(arg),
        }
    }
    if mutate == "hang" {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
    if mutate == "dir" && std::fs::create_dir("made").is_err() {
        return ExitCode::from(1);
    }
    let mark = if mutate == "stdout" { "!" } else { "" };
    println!("args:{}{mark}", args.join(" "));
    let body = match mutate.as_str() {
        "file" => "y\n".to_string(),
        "updated" => "{\"updated\": \"2030-01-01T00:00:00Z\"}\n".to_string(),
        _ => BODY.to_string(),
    };
    let name = if mutate == "path" {
        "renamed.txt"
    } else {
        "out.txt"
    };
    if std::fs::write(name, body).is_err() {
        return ExitCode::from(1);
    }
    ExitCode::from(if mutate == "exit" { 3 } else { 0 })
}
