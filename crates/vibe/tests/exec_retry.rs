#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The `ETXTBSY` retry and the guarded exec helpers in `common` (t_dc14910f). A real "text file
//! busy" cannot be produced on demand, so the retry is exercised with a fake that reports it.
mod common;
#[cfg(unix)]
use common::Home;
use common::{is_text_file_busy, retry_bounded, retry_text_file_busy};
use std::io;
use std::time::Duration;

fn busy() -> io::Error {
    io::Error::new(io::ErrorKind::ExecutableFileBusy, "text file busy")
}

#[test]
fn a_text_file_busy_start_is_retried_until_it_succeeds() {
    let mut attempts = 0;
    let out = retry_text_file_busy(|| {
        attempts += 1;
        if attempts < 3 {
            Err(busy())
        } else {
            Ok(attempts)
        }
    })
    .unwrap();
    assert_eq!(out, 3);
    assert_eq!(attempts, 3);
}

#[test]
fn a_real_error_is_returned_at_once_and_never_retried() {
    let mut attempts = 0;
    let err = retry_text_file_busy::<(), _>(|| {
        attempts += 1;
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
    })
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(attempts, 1, "a non-Busy error must not be retried");
}

#[test]
fn the_retry_is_bounded() {
    let mut attempts = 0;
    let err = retry_bounded(
        || {
            attempts += 1;
            Err::<(), _>(busy())
        },
        4,
        Duration::from_millis(1),
    )
    .unwrap_err();
    assert!(is_text_file_busy(&err));
    assert_eq!(attempts, 4, "the budget is a hard limit");
}

#[test]
fn ordinary_errors_are_not_mistaken_for_a_busy_file() {
    assert!(!is_text_file_busy(&io::Error::from_raw_os_error(2)));
    assert!(!is_text_file_busy(&io::Error::new(
        io::ErrorKind::NotFound,
        "missing"
    )));
    assert!(is_text_file_busy(&busy()));
}

#[cfg(unix)]
#[test]
fn a_script_written_by_the_fixture_runs() {
    let home = Home::new("exec-retry-runs");
    let bin = home.write_exec("bin/echoer", "#!/bin/sh\nexit 5\n");
    let mut cmd = std::process::Command::new(bin);
    let status = common::status(&mut cmd).unwrap();
    assert_eq!(status.code(), Some(5));
}
