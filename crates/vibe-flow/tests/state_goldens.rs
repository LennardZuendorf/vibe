#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;
use vibe_flow::paths::Env;

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct NoEnv;

impl Env for NoEnv {
    fn get(&self, _key: &str) -> Option<String> {
        None
    }
}

struct Golden {
    code: u64,
    stdout: String,
    stderr: String,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus_dir() -> PathBuf {
    repo_root().join("tests/parity/corpus/flow/cursors")
}

fn goldens_dir() -> PathBuf {
    repo_root().join("tests/parity/goldens/flow/state-get")
}

fn scratch(case: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "vibe-flow-state-get-{case}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn copy_tree(from: &Path, to: &Path) {
    if !from.is_dir() {
        return;
    }
    fs::create_dir_all(to).expect("create destination directory");
    for entry in fs::read_dir(from).expect("read source directory") {
        let entry = entry.expect("read source entry");
        let src = entry.path();
        let dst = to.join(entry.file_name());
        let kind = entry.file_type().expect("source entry type");
        if kind.is_dir() {
            copy_tree(&src, &dst);
        } else {
            fs::copy(&src, &dst).expect("copy source file");
        }
    }
}

fn case_names() -> Vec<String> {
    let dir = corpus_dir();
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let entry = entry.unwrap_or_else(|e| panic!("read entry in {}: {e}", dir.display()));
        if entry
            .file_type()
            .unwrap_or_else(|e| panic!("type of {}: {e}", entry.path().display()))
            .is_dir()
        {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    names
}

fn golden_names() -> Vec<String> {
    let dir = goldens_dir();
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let entry = entry.unwrap_or_else(|e| panic!("read entry in {}: {e}", dir.display()));
        let path = entry.path();
        if !entry
            .file_type()
            .unwrap_or_else(|e| panic!("type of {}: {e}", path.display()))
            .is_file()
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let stem = name
            .strip_suffix(".json")
            .unwrap_or_else(|| panic!("golden {} is not a .json file", path.display()));
        names.push(stem.to_string());
    }
    names.sort();
    names
}

fn parse_golden(path: &Path, bytes: &[u8]) -> Golden {
    assert!(
        bytes.ends_with(b"\n"),
        "golden {} must end with a newline",
        path.display()
    );
    let value: Value = serde_json::from_slice(bytes)
        .unwrap_or_else(|e| panic!("parse golden {}: {e}", path.display()));
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("golden {} is not a JSON object", path.display()));
    let code = object
        .get("code")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("golden {} has no integer \"code\"", path.display()));
    let stdout = object
        .get("stdout")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("golden {} has no string \"stdout\"", path.display()));
    let stderr = object
        .get("stderr")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("golden {} has no string \"stderr\"", path.display()));
    Golden {
        code,
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
    }
}

fn check_case(case: &str) {
    let root = scratch(case);
    copy_tree(&corpus_dir().join(case).join("tree"), &root);
    let argv = vec![
        OsString::from("vibe-flow"),
        OsString::from("--root"),
        root.as_os_str().to_os_string(),
        OsString::from("state"),
        OsString::from("get"),
    ];
    let (code, stdout, stderr) = vibe_flow::cli::run(&root, &argv, &NoEnv);
    let root_text = root.to_string_lossy().into_owned();
    let stdout = String::from_utf8(stdout).expect("stdout utf8");
    let stderr = String::from_utf8(stderr).expect("stderr utf8");
    let actual = Golden {
        code: u64::from(code),
        stdout: stdout.replace(&root_text, "<ROOT>"),
        stderr: stderr.replace(&root_text, "<ROOT>"),
    };

    let path = goldens_dir().join(format!("{case}.json"));
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read golden {}: {e}", path.display()));
    let golden = parse_golden(&path, &bytes);

    assert_eq!(
        actual.code,
        golden.code,
        "case {case}: exit code differs from golden {}",
        path.display()
    );
    assert_eq!(
        actual.stdout,
        golden.stdout,
        "case {case}: stdout differs from golden {}",
        path.display()
    );
    assert_eq!(
        actual.stderr,
        golden.stderr,
        "case {case}: stderr differs from golden {}",
        path.display()
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn every_cursor_case_matches_its_checked_in_golden() {
    let cases = case_names();
    assert!(
        !cases.is_empty(),
        "no cases under {}",
        corpus_dir().display()
    );
    let goldens = golden_names();
    let case_set: BTreeSet<&str> = cases.iter().map(String::as_str).collect();
    let golden_set: BTreeSet<&str> = goldens.iter().map(String::as_str).collect();

    for case in &cases {
        assert!(
            golden_set.contains(case.as_str()),
            "case {case} has no golden; expected {}",
            goldens_dir().join(format!("{case}.json")).display()
        );
    }
    for golden in &goldens {
        assert!(
            case_set.contains(golden.as_str()),
            "golden {} has no case under {}",
            goldens_dir().join(format!("{golden}.json")).display(),
            corpus_dir().display()
        );
    }

    for case in &cases {
        check_case(case);
    }
}
