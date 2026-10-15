#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_vibe-spec"))
        .args(args)
        .output()
        .expect("spawn vibe-spec");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn version_exits_zero() {
    let (code, out, _) = run(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out, "vibe-spec 0.4.0\n");
}

#[test]
fn no_arguments_is_a_usage_error() {
    let (code, out, err) = run(&[]);
    assert_eq!(code, 2, "stdout={out} stderr={err}");
    assert!(err.contains("Usage: vibe-spec"), "{err}");
}

#[test]
fn unknown_command_is_a_usage_error() {
    let (code, _, err) = run(&["frobnicate"]);
    assert_eq!(code, 2);
    assert!(err.contains("frobnicate"), "{err}");
}

#[cfg(unix)]
#[test]
fn non_utf8_arguments_never_panic() {
    use std::os::unix::ffi::OsStrExt;
    let bad = std::ffi::OsStr::from_bytes(&[0xff, b'x']);
    for args in [
        vec![
            std::ffi::OsStr::new("--root"),
            bad,
            std::ffi::OsStr::new("scan-merges"),
        ],
        vec![
            std::ffi::OsStr::new("--root"),
            bad,
            std::ffi::OsStr::new("validate"),
        ],
        vec![
            std::ffi::OsStr::new("--root"),
            bad,
            std::ffi::OsStr::new("lessons-for"),
        ],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_vibe-spec"))
            .args(&args)
            .output()
            .expect("spawn vibe-spec");
        assert_ne!(
            out.status.code(),
            Some(101),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.status.code().is_some());
    }
}
