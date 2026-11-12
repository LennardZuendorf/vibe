#![allow(clippy::unwrap_used, clippy::expect_used)]
//! spec-projects-mcp/9: connected real-Git CLI/MCP journeys.
mod common;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

type Files = BTreeMap<String, Vec<u8>>;
const DATE: &str = "2026-01-15T00:00:00Z";
const DESIGN: &str = "superpowers/specs/2026-01-15-checkout-design.md";
const PLAN: &str = "superpowers/plans/2026-01-15-checkout.md";
const OPTIONAL: &str = "spec/optional.md";
const BASE_OPTIONAL: &str = "# Optional\n\nBase.\n";
const SOURCE_LESSONS: &str = "# Lessons\n\nSource.\n";
const TARGET_LESSONS: &str = "# Lessons\n\nTarget.\n";
const TARGET_OPTIONAL: &str = "# Optional\n\nTarget.\n";
const RESOLVED_LESSONS: &str = "# Lessons\n\nResolved.\n";

fn snapshot(root: &Path) -> Files {
    fn walk(root: &Path, dir: &Path, out: &mut Files) {
        if !dir.exists() {
            return;
        }
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let kind = entry.file_type().unwrap();
            assert!(!kind.is_symlink(), "unexpected fixture symlink: {path:?}");
            if kind.is_dir() {
                walk(root, &path, out);
            } else {
                assert!(kind.is_file(), "unexpected fixture file: {path:?}");
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .components()
                    .map(|p| p.as_os_str().to_str().unwrap())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(relative, fs::read(path).unwrap());
            }
        }
    }
    let mut out = Files::new();
    walk(root, root, &mut out);
    out
}

fn clean_command(program: impl AsRef<OsStr>, cwd: &Path, home: &Path, store: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("APPDATA", home.join("appdata"))
        .env("LOCALAPPDATA", home.join("localappdata"))
        .env("VIBE_STORE", store)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", home.join("gitconfig"))
        .env("LC_ALL", "C")
        .env("TZ", "UTC");
    #[cfg(windows)]
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = clean_command("git", cwd, cwd, &cwd.join("unused-store"))
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "maintenance.auto=false",
            "-c",
            "gc.auto=0",
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .env("GIT_AUTHOR_DATE", DATE)
        .env("GIT_COMMITTER_DATE", DATE)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "fixture Git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

#[derive(Debug, PartialEq, Eq)]
struct CodeState {
    files: Files,
    head: String,
    branch: String,
    index: String,
}
impl CodeState {
    fn capture(code: &Path) -> Self {
        Self {
            files: snapshot(code),
            head: git(code, &["rev-parse", "HEAD"]),
            branch: git(code, &["symbolic-ref", "HEAD"]),
            index: git(code, &["ls-files", "--stage"]),
        }
    }
    fn document(&mut self, logical: &str, bytes: &[u8]) {
        let path = if let Some(path) = logical.strip_prefix("spec/") {
            format!("docs/spec/{path}")
        } else {
            format!("docs/{}", logical)
        };
        self.files.insert(path, bytes.into());
    }
}

struct Fixture {
    owner: vibe_core::testscratch::Scratch,
    code: PathBuf,
    neutral: PathBuf,
    store: PathBuf,
}
impl Fixture {
    fn new(name: &str, branch: &str) -> Self {
        let owner = common::project(name);
        let code = owner.join("code");
        let neutral = owner.join("neutral");
        let store = owner.join("store");
        fs::create_dir_all(&code).unwrap();
        fs::create_dir_all(&neutral).unwrap();
        common::write(&code, "src/billing.rs", "// billing\n");
        common::write(&code, "tests/citations.rs", "// checkout/1\n");
        git(&code, &["init", "-b", branch]);
        git(&code, &["config", "maintenance.auto", "false"]);
        git(&code, &["config", "gc.auto", "0"]);
        git(&code, &["add", "."]);
        git(&code, &["commit", "-m", "initial"]);
        Self {
            owner,
            code,
            neutral,
            store,
        }
    }
    fn cli_input(&self, cwd: &Path, args: &[&str], input: &[u8]) -> Value {
        let mut command = clean_command(
            env!("CARGO_BIN_EXE_vibe-spec"),
            cwd,
            &self.owner,
            &self.store,
        );
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "CLI {args:?}: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "CLI {args:?} JSON: {error}: {}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }
    fn cli(&self, cwd: &Path, args: &[&str]) -> Value {
        self.cli_input(cwd, args, &[])
    }
    fn context(&self, cwd: &Path) -> Value {
        self.cli(cwd, &["context"])
    }
    fn selected_context(&self, project: &str, checkout: &Path) -> Value {
        self.cli(
            &self.neutral,
            &[
                "context",
                "--project",
                project,
                "--checkout",
                checkout.to_str().unwrap(),
            ],
        )
    }
    fn write(&self, cwd: &Path, selector: &[&str], path: &str, content: &str) {
        let mut args = vec!["context"];
        args.extend_from_slice(selector);
        let context = self.cli(cwd, &args);
        let mut args = vec![
            "document",
            "write",
            path,
            "--expect",
            context["revision_token"].as_str().unwrap(),
        ];
        args.extend_from_slice(selector);
        self.cli_input(cwd, &args, content.as_bytes());
    }
    fn read(&self, cwd: &Path, selector: &[&str], path: &str) -> Value {
        let mut args = vec!["document", "read", path];
        args.extend_from_slice(selector);
        self.cli(cwd, &args)
    }
    fn snapshot(&self, cwd: &Path) {
        let context = self.context(cwd);
        self.cli(
            cwd,
            &[
                "snapshot",
                "--expect",
                context["revision_token"].as_str().unwrap(),
            ],
        );
    }
    fn client(&self, cwd: &Path, defaults: &[&str]) -> Client {
        let mut command = clean_command(
            env!("CARGO_BIN_EXE_vibe-spec"),
            cwd,
            &self.owner,
            &self.store,
        );
        command.args(defaults).args(["mcp", "--stdio"]);
        Client::new(command)
    }
}

// This client only transports the ordinary legacy lifecycle and tool calls.
// Adversarial framing, limits and modern lifecycle coverage stays in mcp.rs.
struct Client {
    child: Child,
    input: Option<ChildStdin>,
    lines: mpsc::Receiver<String>,
    next: u64,
}
impl Client {
    fn new(mut command: Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if send.send(line.unwrap()).is_err() {
                    break;
                }
            }
        });
        let mut client = Self {
            child,
            input,
            lines,
            next: 1,
        };
        let initialized = client.rpc(
            "initialize",
            json!({
                "protocolVersion":"2025-11-25",
                "capabilities":{},
                "clientInfo":{"name":"project-e2e","version":"1"}
            }),
        );
        assert_eq!(initialized["protocolVersion"], "2025-11-25");
        writeln!(
            client.input.as_mut().unwrap(),
            "{}",
            json!({
                "jsonrpc":"2.0", "method":"notifications/initialized"
            })
        )
        .unwrap();
        client
    }
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        writeln!(
            self.input.as_mut().unwrap(),
            "{}",
            json!({
                "jsonrpc":"2.0", "id":id, "method":method, "params":params
            })
        )
        .unwrap();
        let line = self
            .lines
            .recv_timeout(Duration::from_secs(30))
            .expect("MCP response before EOF");
        let response: Value = serde_json::from_str(&line).expect("JSON-only MCP stdout");
        assert_eq!(response["id"], id, "{response}");
        assert!(response.get("error").is_none(), "{response}");
        response["result"].clone()
    }
    fn call(&mut self, name: &str, arguments: Value) -> Value {
        let response = self.rpc("tools/call", json!({"name":name,"arguments":arguments}));
        assert_ne!(response["isError"], true, "{response}");
        response["structuredContent"]["data"].clone()
    }
    fn write(&mut self, selector: Value, path: &str, content: &str) {
        let context = self.call("spec_context", selector.clone());
        let mut arguments = selector;
        arguments["path"] = json!(path);
        arguments["content"] = json!(content);
        arguments["expected_revision"] = context["revision_token"].clone();
        self.call("spec_write", arguments);
    }
    fn close(mut self) {
        self.input.take();
        let status = self.child.wait().unwrap();
        assert!(status.success(), "MCP EOF status: {status}");
        for line in self.lines.try_iter() {
            let _: Value = serde_json::from_str(&line).expect("JSON-only remaining stdout");
        }
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn fixture_documents() -> Files {
    let source = common::layered_tree("project-e2e-documents");
    let mut files: Files = snapshot(&source.join("docs/spec"))
        .into_iter()
        .map(|(path, bytes)| (format!("spec/{path}"), bytes))
        .collect();
    files.insert(
        DESIGN.into(),
        b"# Checkout design\n\nConfirm accepted carts.\n".to_vec(),
    );
    files.insert(
        PLAN.into(),
        b"# Checkout plan\n\n**Spec:** [Checkout design](../specs/2026-01-15-checkout-design.md)\n\n### Task 1: Confirm checkout\n\n- [ ] Implement checkout/1.\n".to_vec(),
    );
    files.insert(OPTIONAL.into(), BASE_OPTIONAL.as_bytes().into());
    files
}
fn materialize_repo(code: &Path, files: &Files) {
    for (path, bytes) in files {
        let relative = path
            .strip_prefix("spec/")
            .map(|p| format!("docs/spec/{p}"))
            .unwrap_or_else(|| format!("docs/{path}"));
        let destination = code.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(destination, bytes).unwrap();
    }
}
fn text(files: &Files, path: &str) -> String {
    String::from_utf8(files[path].clone()).unwrap()
}
fn appended(files: &Files, path: &str, line: &str) -> String {
    format!("{}\n{line}\n", text(files, path))
}
fn managed_files(context: &Value) -> Files {
    let mut out = Files::new();
    for (field, prefix) in [("spec_root", "spec"), ("artifact_root", "superpowers")] {
        for (path, bytes) in snapshot(Path::new(context[field].as_str().unwrap())) {
            out.insert(format!("{prefix}/{path}"), bytes);
        }
    }
    out
}
fn assert_inventory(response: &Value, expected: &Files) {
    let actual: BTreeSet<_> = response["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(actual, expected.keys().cloned().collect());
    assert_eq!(
        response["documents"].as_array().unwrap().len(),
        expected.len()
    );
    for item in response["documents"].as_array().unwrap() {
        assert_eq!(
            item["bytes"],
            expected[item["path"].as_str().unwrap()].len()
        );
    }
}
fn assert_context(context: &Value, project: &str, mode: &str, code: &Path, branch: &str) {
    assert_eq!(context["project_id"], project);
    assert_eq!(context["mode"], mode);
    assert_eq!(
        fs::canonicalize(context["checkout"].as_str().unwrap()).unwrap(),
        fs::canonicalize(code).unwrap()
    );
    assert_eq!(context["code_ref"], format!("refs/heads/{branch}"));
    assert_eq!(context["code_head"], git(code, &["rev-parse", "HEAD"]));
    assert_eq!(context["code_available"], true);
    assert!(
        context["revision_token"]
            .as_str()
            .is_some_and(|token| !token.is_empty())
    );
    if mode == "repo-first" {
        assert_eq!(
            fs::canonicalize(context["spec_root"].as_str().unwrap()).unwrap(),
            fs::canonicalize(code.join("docs/spec")).unwrap()
        );
        assert_eq!(
            fs::canonicalize(context["artifact_root"].as_str().unwrap()).unwrap(),
            fs::canonicalize(code.join("docs/superpowers")).unwrap()
        );
        assert!(context["spec_revision"].is_null());
    } else {
        let root = Path::new(context["spec_root"].as_str().unwrap());
        assert_eq!(root.file_name().unwrap(), "spec");
        assert_eq!(
            root.with_file_name("superpowers"),
            Path::new(context["artifact_root"].as_str().unwrap())
        );
        assert!(!root.starts_with(code));
        assert!(
            context["spec_revision"]
                .as_str()
                .is_some_and(|oid| !oid.is_empty())
        );
    }
}
fn assert_queries(client: &mut Client, selector: Value) {
    let mut arguments = selector.clone();
    arguments["feature"] = json!("checkout");
    assert_eq!(
        client.call("spec_feature", arguments.clone())["layer"],
        "superpowers"
    );
    let plan = client.call("spec_plan", arguments);
    assert_eq!(plan["rows"].as_array().unwrap().len(), 1);
    assert_eq!(plan["rows"][0]["id"], "Task 1");
    assert_eq!(plan["rows"][0]["status"], "todo");
    let mut arguments = selector;
    arguments["scope"] = json!("with-code");
    assert_eq!(client.call("spec_validate", arguments)["errors"], 0);
}

fn attached_clone(f: &Fixture, project: &str, name: &str, branch: Option<&str>) -> PathBuf {
    let clone = f.owner.join(name);
    git(
        &f.neutral,
        &["clone", f.code.to_str().unwrap(), clone.to_str().unwrap()],
    );
    let clone = fs::canonicalize(clone).unwrap();
    git(&clone, &["config", "maintenance.auto", "false"]);
    git(&clone, &["config", "gc.auto", "0"]);
    f.cli(
        &f.neutral,
        &[
            "project",
            "attach",
            "--project",
            project,
            "--checkout",
            clone.to_str().unwrap(),
        ],
    );
    if let Some(branch) = branch {
        git(&clone, &["checkout", "-b", branch]);
    }
    clone
}

#[test]
fn branch_management_keeps_base_and_existing_target_selection() {
    let f = Fixture::new("branch-management-selection", "main");
    let initialized = f.cli(&f.code, &["project", "init", "--mode", "global-first"]);
    let project = initialized["project"]["id"].as_str().unwrap();
    let docs = fixture_documents();
    for path in [DESIGN, PLAN] {
        f.write(&f.code, &[], path, &text(&docs, path));
    }
    let clone = attached_clone(&f, project, "clone", None);
    let main = f.context(&f.code);
    let expected = managed_files(&main);
    let code_before = CodeState::capture(&f.code);
    let clone_before = CodeState::capture(&clone);
    let ordinary = clean_command(
        env!("CARGO_BIN_EXE_vibe-spec"),
        &f.neutral,
        &f.owner,
        &f.store,
    )
    .args(["context", "--project", project, "--branch", "main"])
    .output()
    .unwrap();
    assert!(!ordinary.status.success());
    assert!(String::from_utf8_lossy(&ordinary.stderr).contains("ambiguous_checkout"));
    let topic = f.cli(
        &f.neutral,
        &[
            "branch",
            "init",
            "topic",
            "--from",
            "main",
            "--project",
            project,
            "--checkout",
            f.code.to_str().unwrap(),
        ],
    );
    assert_eq!(
        topic["branch"]["parent_spec_revision"],
        main["spec_revision"]
    );
    assert_eq!(managed_files(&topic["context"]), expected);
    assert_eq!(topic["context"]["code_available"], false);
    assert!(topic["context"]["checkout"].is_null());
    assert!(topic["context"]["code_head"].is_null());
    assert_eq!(topic["branch"]["last_code_head"], main["code_head"]);
    assert_eq!(topic["branch"]["last_code_ref"], "refs/heads/main");
    let store_before = snapshot(&f.store);
    let existing = f.cli(
        &f.neutral,
        &[
            "branch",
            "init",
            "main",
            "--from",
            "main",
            "--project",
            project,
            "--checkout",
            clone.to_str().unwrap(),
        ],
    );
    assert_context(
        &existing["context"],
        project,
        "global-first",
        &clone,
        "main",
    );
    assert_eq!(
        snapshot(&f.store),
        store_before,
        "idempotent branch changed storage"
    );
    let neutral = f.cli(
        &f.neutral,
        &[
            "branch",
            "init",
            "main",
            "--from",
            "main",
            "--project",
            project,
        ],
    );
    assert!(neutral["context"]["checkout"].is_null());
    assert!(neutral["context"]["code_head"].is_null());
    assert_eq!(neutral["context"]["code_available"], false);
    assert_eq!(managed_files(&neutral["context"]), expected);
    assert_eq!(snapshot(&f.store), store_before);
    assert_eq!(CodeState::capture(&f.code), code_before);
    assert_eq!(CodeState::capture(&clone), clone_before);

    git(&f.code, &["checkout", "-b", "third"]);
    let code_before = CodeState::capture(&f.code);
    let ordinary = clean_command(
        env!("CARGO_BIN_EXE_vibe-spec"),
        &f.neutral,
        &f.owner,
        &f.store,
    )
    .args([
        "context",
        "--project",
        project,
        "--branch",
        "main",
        "--checkout",
        f.code.to_str().unwrap(),
    ])
    .output()
    .unwrap();
    assert!(!ordinary.status.success());
    assert!(String::from_utf8_lossy(&ordinary.stderr).contains("context_mismatch"));
    let stored_only = f.cli(
        &f.neutral,
        &[
            "branch",
            "init",
            "stored-only",
            "--from",
            "main",
            "--project",
            project,
            "--checkout",
            f.code.to_str().unwrap(),
        ],
    );
    assert_eq!(
        stored_only["branch"]["parent_spec_revision"],
        main["spec_revision"]
    );
    assert!(stored_only["context"]["checkout"].is_null());
    assert!(stored_only["context"]["code_head"].is_null());
    assert_eq!(stored_only["context"]["code_ref"], "refs/heads/stored-only");
    assert_eq!(stored_only["context"]["code_available"], false);
    assert!(stored_only["branch"]["last_code_head"].is_null());
    assert!(stored_only["branch"]["last_code_ref"].is_null());
    assert_eq!(managed_files(&stored_only["context"]), expected);
    assert_eq!(CodeState::capture(&f.code), code_before);
    assert_eq!(CodeState::capture(&clone), clone_before);

    git(&f.code, &["checkout", "-b", "already-on-target"]);
    let code_before = CodeState::capture(&f.code);
    let target = f.cli(
        &f.neutral,
        &[
            "branch",
            "init",
            "already-on-target",
            "--from",
            "main",
            "--project",
            project,
            "--checkout",
            f.code.to_str().unwrap(),
        ],
    );
    assert_context(
        &target["context"],
        project,
        "global-first",
        &f.code,
        "already-on-target",
    );
    assert_eq!(
        target["branch"]["parent_spec_revision"],
        main["spec_revision"]
    );
    assert_eq!(
        target["branch"]["last_code_head"],
        target["context"]["code_head"]
    );
    assert_eq!(
        target["branch"]["last_code_ref"],
        "refs/heads/already-on-target"
    );
    assert_eq!(managed_files(&target["context"]), expected);
    assert_eq!(CodeState::capture(&f.code), code_before);
    assert_eq!(CodeState::capture(&clone), clone_before);
}

#[test]
fn branch_management_new_target_response_does_not_scan_other_clones() {
    let f = Fixture::new("branch-management-target-clones", "main");
    let initialized = f.cli(&f.code, &["project", "init", "--mode", "global-first"]);
    let project = initialized["project"]["id"].as_str().unwrap();
    let a = attached_clone(&f, project, "clone-a", Some("target"));
    let b = attached_clone(&f, project, "clone-b", Some("target"));
    let main = f.context(&f.code);
    let expected = managed_files(&main);
    let code_before = CodeState::capture(&f.code);
    let a_before = CodeState::capture(&a);
    let b_before = CodeState::capture(&b);
    let target = f.cli(
        &f.neutral,
        &[
            "branch",
            "init",
            "target",
            "--from",
            "main",
            "--project",
            project,
            "--checkout",
            f.code.to_str().unwrap(),
        ],
    );
    assert_eq!(
        target["branch"]["parent_spec_revision"],
        main["spec_revision"]
    );
    assert_eq!(managed_files(&target["context"]), expected);
    assert!(target["context"]["checkout"].is_null());
    assert!(target["context"]["code_head"].is_null());
    assert_eq!(target["context"]["code_available"], false);
    let before_retry = snapshot(&f.store);
    let repeated = f.cli(
        &f.neutral,
        &[
            "branch",
            "init",
            "target",
            "--from",
            "main",
            "--project",
            project,
            "--checkout",
            f.code.to_str().unwrap(),
        ],
    );
    assert_eq!(repeated, target);
    assert_eq!(snapshot(&f.store), before_retry);
    assert_eq!(CodeState::capture(&f.code), code_before);
    assert_eq!(CodeState::capture(&a), a_before);
    assert_eq!(CodeState::capture(&b), b_before);
}

#[test]
fn branch_management_explicit_project_ignores_only_unrelated_ambient_checkout() {
    let f = Fixture::new("branch-management-explicit-project", "main");
    let initialized = f.cli(&f.code, &["project", "init", "--mode", "global-first"]);
    let project = initialized["project"]["id"].as_str().unwrap();
    let other = f.owner.join("unrelated");
    fs::create_dir(&other).unwrap();
    git(&other, &["init", "-b", "unrelated"]);
    git(&other, &["commit", "--allow-empty", "-m", "unrelated"]);
    let before = CodeState::capture(&other);
    let code_before = CodeState::capture(&f.code);
    let target = f.cli(
        &other,
        &[
            "branch",
            "init",
            "target",
            "--from",
            "main",
            "--project",
            project,
        ],
    );
    assert!(target["context"]["checkout"].is_null());
    assert!(target["context"]["code_head"].is_null());
    assert!(target["branch"]["last_code_head"].is_null());
    assert!(target["branch"]["last_code_ref"].is_null());
    assert_eq!(target["context"]["code_available"], false);
    let store_before = snapshot(&f.store);
    for target in ["target", "new-target"] {
        let output = clean_command(env!("CARGO_BIN_EXE_vibe-spec"), &other, &f.owner, &f.store)
            .args([
                "branch",
                "init",
                target,
                "--from",
                "main",
                "--project",
                project,
                "--checkout",
                other.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("context_mismatch"));
        assert_eq!(snapshot(&f.store), store_before);
    }
    assert_eq!(CodeState::capture(&other), before);
    assert_eq!(CodeState::capture(&f.code), code_before);
}

#[test]
fn branch_management_preserves_anchor_root_overrides_before_publication() {
    for kind in ["env-mismatch", "local-mismatch", "env-base", "local-base"] {
        let f = Fixture::new("branch-management-overrides", "main");
        let initialized = f.cli(&f.code, &["project", "init", "--mode", "global-first"]);
        let project = initialized["project"]["id"].as_str().unwrap();
        let main = f.context(&f.code);
        git(&f.code, &["checkout", "-b", "third"]);
        let configured = if kind.ends_with("base") {
            PathBuf::from(main["spec_root"].as_str().unwrap())
        } else {
            f.owner.join("wrong-root")
        };
        if kind.starts_with("local") {
            common::write(
                &f.code,
                ".vibe/spec.json",
                &json!({"root":configured}).to_string(),
            );
        }
        let code_before = CodeState::capture(&f.code);
        let store_before = snapshot(&f.store);
        let mut command = clean_command(
            env!("CARGO_BIN_EXE_vibe-spec"),
            &f.neutral,
            &f.owner,
            &f.store,
        );
        command.args([
            "branch",
            "init",
            "new-target",
            "--from",
            "main",
            "--project",
            project,
            "--checkout",
            f.code.to_str().unwrap(),
        ]);
        if kind.starts_with("env") {
            command.env("VIBE_SPEC_ROOT", &configured);
        }
        let output = command.output().unwrap();
        assert!(!output.status.success(), "{kind} published branch");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("context_mismatch"),
            "{kind}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            snapshot(&f.store),
            store_before,
            "{kind} partially published branch"
        );
        assert_eq!(
            CodeState::capture(&f.code),
            code_before,
            "{kind} modified code"
        );
    }
}

fn management_cli_selector_placement(kind: &str) {
    let f = Fixture::new("management-selector-placement", "main");
    let b = f.cli(&f.code, &["project", "init", "--mode", "global-first"]);
    let project_b = b["project"]["id"].as_str().unwrap();
    let a = f.owner.join("project-a");
    fs::create_dir(&a).unwrap();
    git(&a, &["init", "-b", "a-branch"]);
    git(&a, &["commit", "--allow-empty", "-m", "project A"]);
    let initialized = f.cli(&a, &["project", "init", "--mode", "global-first"]);
    let project_a = initialized["project"]["id"].as_str().unwrap();
    assert_ne!(project_a, project_b);
    let a_context = f.context(&a);
    let b_context = f.context(&f.code);
    let a_before = managed_files(&a_context);
    let b_before = managed_files(&b_context);
    let a_code_before = CodeState::capture(&a);
    let b_code_before = CodeState::capture(&f.code);
    let prefix = match kind {
        "project" => vec!["--project", project_a],
        "checkout" => vec!["--checkout", a.to_str().unwrap()],
        "branch" => vec!["--branch", "a-branch"],
        _ => unreachable!(),
    };
    let attempted = "# Lessons\n\nWrong-placement content must not reach project B.\n";
    let input = f.owner.join("replacement.md");
    fs::write(&input, attempted).unwrap();
    let store_before = snapshot(&f.store);
    for input in [&input, &f.owner.join("must-not-be-read.md")] {
        let mut args = prefix.clone();
        args.extend([
            "document",
            "write",
            "spec/lessons.md",
            "--input",
            input.to_str().unwrap(),
            "--expect",
            b_context["revision_token"].as_str().unwrap(),
        ]);
        let output = clean_command(env!("CARGO_BIN_EXE_vibe-spec"), &f.code, &f.owner, &f.store)
            .args(args)
            .output()
            .unwrap();
        let store_after = snapshot(&f.store);
        assert!(
            !output.status.success(),
            "misplaced {kind} succeeded; changed store: {}; B lessons: {}",
            store_after != store_before,
            fs::read_to_string(
                Path::new(b_context["spec_root"].as_str().unwrap()).join("lessons.md")
            )
            .unwrap()
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("invalid_request") && stderr.contains("must follow"),
            "{stderr}"
        );
        assert_eq!(
            store_after, store_before,
            "misplaced {kind} changed storage"
        );
        assert_eq!(managed_files(&a_context), a_before);
        assert_eq!(managed_files(&b_context), b_before);
        assert_eq!(CodeState::capture(&a), a_code_before);
        assert_eq!(CodeState::capture(&f.code), b_code_before);
    }
    // --root keeps its existing Runtime-override meaning for management calls.
    let rooted = f.cli(
        &f.code,
        &[
            "--root",
            b_context["spec_root"].as_str().unwrap(),
            "context",
        ],
    );
    assert_eq!(rooted["project_id"], project_b);
    assert_eq!(rooted["spec_root"], b_context["spec_root"]);
    let correct = match kind {
        "project" => vec!["--project", project_a],
        "checkout" => vec!["--checkout", a.to_str().unwrap()],
        "branch" => vec!["--project", project_a, "--branch", "a-branch"],
        _ => unreachable!(),
    };
    let content = "# Lessons\n\nCorrect placement selects project A.\n";
    let mut args = vec![
        "document",
        "write",
        "spec/lessons.md",
        "--expect",
        a_context["revision_token"].as_str().unwrap(),
    ];
    args.extend(correct);
    f.cli_input(&f.code, &args, content.as_bytes());
    let mut expected_a = a_before;
    expected_a.insert("spec/lessons.md".into(), content.as_bytes().into());
    assert_eq!(managed_files(&a_context), expected_a);
    assert_eq!(managed_files(&b_context), b_before);
    assert_eq!(CodeState::capture(&a), a_code_before);
    assert_eq!(CodeState::capture(&f.code), b_code_before);
}

#[test]
fn management_cli_rejects_misplaced_project_before_inputs_and_effects() {
    management_cli_selector_placement("project");
}

#[test]
fn management_cli_rejects_misplaced_checkout_before_inputs_and_effects() {
    management_cli_selector_placement("checkout");
}

#[test]
fn management_cli_rejects_misplaced_branch_before_inputs_and_effects() {
    management_cli_selector_placement("branch");
}

#[test]
fn context_safety_hooks_reject_selectors_before_reading_input() {
    struct MustNotRead;
    impl std::io::Read for MustNotRead {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            panic!("unsupported hook selectors must be refused before stdin");
        }
    }
    let f = Fixture::new("hook-context-selector", "main");
    let runtime = vibe_spec::Runtime {
        cwd: f.code.clone(),
        env: BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), f.owner.as_os_str().to_owned()),
            ("VIBE_STORE".into(), f.store.as_os_str().to_owned()),
        ]),
    };
    let baseline = ["vibe-spec", "hook", "guard"].map(std::ffi::OsString::from);
    let baseline = vibe_spec::run_cli(&baseline, &runtime, &mut std::io::Cursor::new(b"{}"));
    assert_eq!(baseline.exit, 0);
    let rooted = [
        "vibe-spec",
        "--root",
        f.owner.to_str().unwrap(),
        "hook",
        "guard",
    ]
    .map(std::ffi::OsString::from);
    let rooted = vibe_spec::run_cli(&rooted, &runtime, &mut std::io::Cursor::new(b"{}"));
    assert_eq!(
        (rooted.exit, rooted.stdout, rooted.stderr),
        (baseline.exit, baseline.stdout, baseline.stderr)
    );
    for selector in [
        ["--project", "p-unused"],
        ["--checkout", f.code.to_str().unwrap()],
        ["--branch", "main"],
    ] {
        let argv =
            ["vibe-spec", selector[0], selector[1], "hook", "guard"].map(std::ffi::OsString::from);
        let result = vibe_spec::run_cli(&argv, &runtime, &mut MustNotRead);
        assert_eq!(result.exit, 1);
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(
            error.contains("invalid_request") && error.contains("do not support"),
            "{error}"
        );
    }
}

fn setup_seed_requires_current_checkout(existing: bool) {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    let f = Fixture::new("setup-seed-current-checkout", "main");
    git(&f.code, &["branch", "other"]);
    if existing {
        f.cli(&f.code, &["project", "init", "--mode", "repo-first"]);
        git(&f.code, &["add", "docs"]);
        git(&f.code, &["commit", "-m", "committed main specs"]);
    }
    let expected_revision = f.cli(&f.code, &["project", "list"])["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let registry_path = f.store.join("spec/registry.json");
    let registry_before = fs::read(&registry_path).ok();
    let switched = Arc::new(AtomicBool::new(false));
    let post_switch = Arc::new(Mutex::new(None));
    let code = f.code.clone();
    let projects = f.store.join("spec/projects");
    let observed_switch = switched.clone();
    let observed_code = post_switch.clone();
    let control = vibe_spec::InvocationControl::new(move || {
        let ready = fs::read_dir(&projects).ok().is_some_and(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .path()
                    .join(if existing {
                        "project.lock"
                    } else {
                        "setup.json"
                    })
                    .is_file()
            })
        });
        if ready && !observed_switch.swap(true, Ordering::SeqCst) {
            git(&code, &["checkout", "other"]);
            *observed_code.lock().unwrap() = Some(CodeState::capture(&code));
        }
        false
    });
    let api = vibe_spec::SpecApi::new(vibe_spec::Runtime {
        cwd: f.code.clone(),
        env: BTreeMap::from([
            ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
            ("HOME".into(), f.owner.as_os_str().to_owned()),
            ("VIBE_STORE".into(), f.store.as_os_str().to_owned()),
        ]),
    })
    .unwrap()
    .with_control(control);
    let error = api
        .execute(vibe_spec::SpecRequest::ProjectInit(
            vibe_spec::InitProjectRequest {
                selector: vibe_spec::ContextSelector::default(),
                mode: vibe_spec::StorageMode::RepoFirst,
                name: None,
                expected_revision,
            },
        ))
        .unwrap_err();
    assert!(
        switched.load(Ordering::SeqCst),
        "did not reach the controlled setup boundary"
    );
    assert_eq!(error.code, "context_changed");
    assert!(
        !f.code.join("docs/spec").exists(),
        "refused setup seeded the switched branch"
    );
    assert_eq!(fs::read(&registry_path).ok(), registry_before);
    let expected = post_switch.lock().unwrap();
    assert_eq!(
        &CodeState::capture(&f.code),
        expected.as_ref().unwrap(),
        "Vibe changed code after the fixture-owned checkout switch"
    );
}

#[test]
fn context_safety_existing_repo_setup_does_not_seed_a_switched_branch() {
    setup_seed_requires_current_checkout(true);
}

#[test]
fn context_safety_new_repo_setup_does_not_seed_a_switched_branch() {
    setup_seed_requires_current_checkout(false);
}

// Break caught: root access hardcodes main, a worktree leaks writes into its
// sibling, or an adapter changes code Git/index instead of only managed docs.
#[test]
fn repo_first_cli_mcp_worktrees_follow_actual_main_and_git_merge() {
    let f = Fixture::new("project-e2e-repo", "trunk-live");
    let base = fixture_documents();
    materialize_repo(&f.code, &base);
    git(&f.code, &["add", "docs"]);
    git(&f.code, &["commit", "-m", "spec baseline"]);
    let before_setup = CodeState::capture(&f.code);
    let initialized = f.cli(&f.code, &["project", "init", "--mode", "repo-first"]);
    let project = initialized["project"]["id"].as_str().unwrap();
    assert_eq!(CodeState::capture(&f.code), before_setup);
    let worktree = f.owner.join("worktree");
    git(
        &f.code,
        &[
            "worktree",
            "add",
            "-b",
            "feature",
            worktree.to_str().unwrap(),
        ],
    );
    let worktree = fs::canonicalize(worktree).unwrap();
    f.cli(
        &f.neutral,
        &[
            "project",
            "attach",
            "--project",
            project,
            "--checkout",
            worktree.to_str().unwrap(),
        ],
    );
    let mut main_state = CodeState::capture(&f.code);
    let mut worktree_state = CodeState::capture(&worktree);
    let main = f.cli(&f.neutral, &["context", "--project", project]);
    assert_context(&main, project, "repo-first", &f.code, "trunk-live");
    assert_context(
        &f.context(&worktree),
        project,
        "repo-first",
        &worktree,
        "feature",
    );
    assert_eq!(managed_files(&main), base);
    assert_eq!(managed_files(&f.context(&worktree)), base);
    let mut main_expected = base.clone();
    let mut feature_expected = base.clone();
    let main_product = appended(&base, "spec/product.md", "CLI main fact.");
    let main_design = appended(&base, DESIGN, "CLI artifact fact.");
    for (path, value) in [("spec/product.md", &main_product), (DESIGN, &main_design)] {
        f.write(&f.code, &[], path, value);
        main_expected.insert(path.into(), value.as_bytes().into());
        main_state.document(path, value.as_bytes());
    }
    let mut main_client = f.client(&f.code, &[]);
    assert_eq!(
        main_client.call("spec_read", json!({"path":"spec/product.md"}))["content"],
        main_product
    );
    assert_inventory(
        &main_client.call("spec_documents", json!({})),
        &main_expected,
    );
    assert_queries(&mut main_client, json!({}));
    let feature_tech = appended(&base, "spec/tech.md", "MCP worktree fact.");
    let feature_plan = appended(&base, PLAN, "MCP artifact fact.");
    let mut feature_client = f.client(&worktree, &[]);
    for (path, value) in [("spec/tech.md", &feature_tech), (PLAN, &feature_plan)] {
        feature_client.write(json!({}), path, value);
        feature_expected.insert(path.into(), value.as_bytes().into());
        worktree_state.document(path, value.as_bytes());
        assert_eq!(f.read(&worktree, &[], path)["content"], *value);
    }
    assert_inventory(
        &feature_client.call("spec_documents", json!({})),
        &feature_expected,
    );
    assert_queries(&mut feature_client, json!({}));
    for checkout in [&f.code, &worktree] {
        assert_eq!(f.cli(checkout, &["validate", "--json"])["errors"], 0);
        assert_eq!(
            f.cli(checkout, &["plan", "--feature", "checkout", "--json"])["rows"][0]["id"],
            "Task 1"
        );
    }
    assert_eq!(managed_files(&f.context(&f.code)), main_expected);
    assert_eq!(managed_files(&f.context(&worktree)), feature_expected);
    assert_eq!(CodeState::capture(&f.code), main_state);
    assert_eq!(CodeState::capture(&worktree), worktree_state);
    let refresh = main_client.call("spec_refresh", json!({}));
    for field in ["applied", "conflict", "needs_input"] {
        assert_eq!(refresh[field], json!([]));
    }
    main_client.close();
    feature_client.close();

    // These are fixture-owned code commits/merge, never Vibe operations.
    git(&f.code, &["add", "docs"]);
    git(&f.code, &["commit", "-m", "main spec change"]);
    git(&worktree, &["add", "docs"]);
    git(&worktree, &["commit", "-m", "feature spec change"]);
    git(
        &f.code,
        &["merge", "--no-ff", "feature", "-m", "merge specs"],
    );
    let main_after_merge = CodeState::capture(&f.code);
    let worktree_after_merge = CodeState::capture(&worktree);
    let mut merged = main_expected;
    merged.insert("spec/tech.md".into(), feature_tech.as_bytes().into());
    merged.insert(PLAN.into(), feature_plan.as_bytes().into());
    let main = f.cli(&f.neutral, &["context", "--project", project]);
    assert_context(&main, project, "repo-first", &f.code, "trunk-live");
    assert_eq!(managed_files(&main), merged);
    assert_eq!(managed_files(&f.context(&worktree)), feature_expected);
    let mut reopened = f.client(&f.neutral, &["--project", project]);
    assert_eq!(
        reopened.call("spec_context", json!({}))["code_head"],
        main["code_head"]
    );
    assert_inventory(&reopened.call("spec_documents", json!({})), &merged);
    for path in ["spec/product.md", "spec/tech.md", DESIGN, PLAN] {
        assert_eq!(
            reopened.call("spec_read", json!({"path":path}))["content"],
            text(&merged, path)
        );
        assert_eq!(f.read(&f.code, &[], path)["content"], text(&merged, path));
    }
    assert_queries(&mut reopened, json!({}));
    assert_eq!(
        reopened.call("spec_refresh", json!({}))["applied"],
        json!([])
    );
    reopened.close();
    assert_eq!(CodeState::capture(&f.code), main_after_merge);
    assert_eq!(CodeState::capture(&worktree), worktree_after_merge);
    let project_store = f.store.join("spec/projects").join(project);
    assert!(!project_store.join("reconciliations").exists());
}

// Break caught: global storage omits feature artifacts, clone identity duplicates
// storage, squash history auto-merges, or saved conflict resolution loses state.
#[test]
fn global_first_clone_tree_squash_saved_plan_conflict_and_reopen() {
    let f = Fixture::new("project-e2e-global", "main");
    git(
        &f.code,
        &["remote", "add", "origin", "git@github.com:Example/E2E.git"],
    );
    let mut code_state = CodeState::capture(&f.code);
    let initialized = f.cli(&f.code, &["project", "init", "--mode", "global-first"]);
    let project = initialized["project"]["id"].as_str().unwrap();
    let after_setup = snapshot(&f.store);
    let repeated = f.cli(&f.code, &["project", "init", "--mode", "global-first"]);
    assert_eq!(repeated["project"]["id"], project);
    assert_eq!(snapshot(&f.store), after_setup);
    assert_eq!(CodeState::capture(&f.code), code_state);
    let mut shared = fixture_documents();
    for (path, bytes) in &shared {
        f.write(&f.code, &[], path, std::str::from_utf8(bytes).unwrap());
    }
    let main = f.context(&f.code);
    assert_context(&main, project, "global-first", &f.code, "main");
    assert_eq!(managed_files(&main), shared);
    assert_inventory(&f.cli(&f.code, &["document", "list"]), &shared);
    assert_eq!(CodeState::capture(&f.code), code_state);

    let clone = f.owner.join("clone");
    git(
        &f.neutral,
        &["clone", f.code.to_str().unwrap(), clone.to_str().unwrap()],
    );
    let clone = fs::canonicalize(clone).unwrap();
    git(&clone, &["config", "maintenance.auto", "false"]);
    git(&clone, &["config", "gc.auto", "0"]);
    git(
        &clone,
        &[
            "remote",
            "set-url",
            "origin",
            "https://github.com/Example/E2E.git",
        ],
    );
    let mut clone_state = CodeState::capture(&clone);
    // Automatic remote identity matching must work before any explicit attachment.
    let clone_context = f.context(&clone);
    assert_context(&clone_context, project, "global-first", &clone, "main");
    assert_eq!(clone_context["spec_root"], main["spec_root"]);
    assert_eq!(clone_context["artifact_root"], main["artifact_root"]);
    assert_eq!(managed_files(&clone_context), shared);
    f.cli(
        &f.neutral,
        &[
            "project",
            "attach",
            "--project",
            project,
            "--checkout",
            clone.to_str().unwrap(),
        ],
    );
    let cli_product = appended(&shared, "spec/product.md", "CLI clone fact.");
    f.write(&f.code, &[], "spec/product.md", &cli_product);
    shared.insert("spec/product.md".into(), cli_product.as_bytes().into());
    let mcp_design = appended(&shared, DESIGN, "MCP shared artifact fact.");
    let mut clone_client = f.client(&clone, &[]);
    assert_eq!(
        clone_client.call("spec_read", json!({"path":"spec/product.md"}))["content"],
        cli_product
    );
    clone_client.write(json!({}), DESIGN, &mcp_design);
    shared.insert(DESIGN.into(), mcp_design.as_bytes().into());
    assert_eq!(f.read(&f.code, &[], DESIGN)["content"], mcp_design);
    assert_inventory(&clone_client.call("spec_documents", json!({})), &shared);
    assert_queries(&mut clone_client, json!({}));
    assert_eq!(f.cli(&clone, &["validate", "--json"])["errors"], 0);
    assert_eq!(managed_files(&f.context(&f.code)), shared);
    assert_eq!(managed_files(&f.context(&clone)), shared);
    assert_eq!(CodeState::capture(&f.code), code_state);
    assert_eq!(CodeState::capture(&clone), clone_state);
    clone_client.close();

    f.cli(
        &f.neutral,
        &[
            "branch",
            "init",
            "topic",
            "--from",
            "main",
            "--project",
            project,
        ],
    );
    let main_before_source = f.context(&f.code);
    let source = f.cli(
        &f.neutral,
        &["context", "--project", project, "--branch", "topic"],
    );
    assert_eq!(managed_files(&source), shared);
    assert_ne!(source["spec_root"], main_before_source["spec_root"]);
    assert_ne!(source["artifact_root"], main_before_source["artifact_root"]);
    assert_eq!(source["code_available"], false);
    let branch_selector = ["--project", project, "--branch", "topic"];
    f.write(
        &f.neutral,
        &branch_selector,
        "spec/lessons.md",
        SOURCE_LESSONS,
    );
    let source_plan = appended(&shared, PLAN, "Source branch artifact.");
    f.write(&f.neutral, &branch_selector, PLAN, &source_plan);
    let source = f.cli(
        &f.neutral,
        &["context", "--project", project, "--branch", "topic"],
    );
    f.cli(
        &f.neutral,
        &[
            "document",
            "remove",
            OPTIONAL,
            "--expect",
            source["revision_token"].as_str().unwrap(),
            "--project",
            project,
            "--branch",
            "topic",
        ],
    );
    let mut source_expected = shared.clone();
    source_expected.insert("spec/lessons.md".into(), SOURCE_LESSONS.as_bytes().into());
    source_expected.insert(PLAN.into(), source_plan.as_bytes().into());
    source_expected.remove(OPTIONAL);
    let source = f.cli(
        &f.neutral,
        &["context", "--project", project, "--branch", "topic"],
    );
    assert_eq!(managed_files(&source), source_expected);
    let main_still = f.context(&f.code);
    assert_eq!(managed_files(&main_still), shared);
    assert_eq!(
        main_still["spec_revision"],
        main_before_source["spec_revision"]
    );
    f.write(&f.code, &[], "spec/lessons.md", TARGET_LESSONS);
    f.write(&f.code, &[], OPTIONAL, TARGET_OPTIONAL);
    let mut target_expected = shared.clone();
    target_expected.insert("spec/lessons.md".into(), TARGET_LESSONS.as_bytes().into());
    target_expected.insert(OPTIONAL.into(), TARGET_OPTIONAL.as_bytes().into());
    // Clean explicit snapshots capture exact code/spec bindings.
    f.snapshot(&f.code);
    assert_eq!(CodeState::capture(&f.code), code_state);
    assert_eq!(CodeState::capture(&clone), clone_state);

    git(&f.code, &["checkout", "-b", "topic"]);
    common::write(&f.code, "src/checkout.rs", "// fixture feature\n");
    git(&f.code, &["add", "src/checkout.rs"]);
    git(&f.code, &["commit", "-m", "feature code"]);
    code_state = CodeState::capture(&f.code);
    f.snapshot(&f.code);
    assert_eq!(managed_files(&f.context(&f.code)), source_expected);
    assert_eq!(CodeState::capture(&f.code), code_state);
    assert_eq!(CodeState::capture(&clone), clone_state);
    git(&f.code, &["checkout", "main"]);
    code_state = CodeState::capture(&f.code);
    let target_before_squash = f.selected_context(project, &f.code);
    assert_eq!(managed_files(&target_before_squash), target_expected);
    assert_eq!(CodeState::capture(&f.code), code_state);
    // Fixture-owned squash completes independently of the conflicting spec tree.
    git(&f.code, &["merge", "--squash", "topic"]);
    git(&f.code, &["commit", "-m", "squash feature code"]);
    code_state = CodeState::capture(&f.code);
    assert_eq!(
        git(&f.code, &["rev-list", "--parents", "-n", "1", "HEAD"])
            .split_whitespace()
            .count(),
        2
    );
    let selected = f.selected_context(project, &f.code);
    assert_eq!(selected["refresh"]["applied"], json!([]));
    assert_eq!(
        selected["spec_revision"],
        target_before_squash["spec_revision"]
    );
    assert_eq!(managed_files(&selected), target_expected);
    assert_eq!(CodeState::capture(&f.code), code_state);
    assert_eq!(CodeState::capture(&clone), clone_state);
    // Align the other clone through fixture-owned local Git actions. Refresh
    // can now classify squash history with one uniquely available target tip.
    git(&clone, &["fetch", f.code.to_str().unwrap(), "main"]);
    git(&clone, &["merge", "--ff-only", "FETCH_HEAD"]);
    clone_state = CodeState::capture(&clone);
    let selected = f.selected_context(project, &f.code);
    assert_eq!(selected["refresh"]["applied"], json!([]));
    assert!(
        !selected["refresh"]["needs_input"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        selected["spec_revision"],
        target_before_squash["spec_revision"]
    );
    assert_eq!(managed_files(&selected), target_expected);
    let mut selected_client = f.client(
        &f.neutral,
        &["--project", project, "--checkout", f.code.to_str().unwrap()],
    );
    let report = selected_client.call("spec_refresh", json!({}));
    assert_eq!(report["applied"], json!([]));
    assert_eq!(
        report["context"]["spec_revision"],
        target_before_squash["spec_revision"]
    );
    assert_queries(&mut selected_client, json!({}));
    assert_eq!(CodeState::capture(&f.code), code_state);
    assert_eq!(CodeState::capture(&clone), clone_state);

    let before_preview = snapshot(&f.store);
    let plan = f.cli(
        &f.neutral,
        &[
            "reconcile",
            "--from",
            "topic",
            "--into",
            "main",
            "--dry-run",
            "--project",
            project,
            "--checkout",
            f.code.to_str().unwrap(),
        ],
    );
    assert_eq!(
        snapshot(&f.store),
        before_preview,
        "preview wrote or refreshed the store"
    );
    assert_eq!(plan["target_spec_oid"], selected["spec_revision"]);
    assert_eq!(plan["source_spec_oid"], source["spec_revision"]);
    assert_eq!(
        fs::canonicalize(plan["target_checkout"].as_str().unwrap()).unwrap(),
        fs::canonicalize(&f.code).unwrap()
    );
    let plan_path = f.owner.join("saved-plan.json");
    fs::write(&plan_path, serde_json::to_vec(&plan).unwrap()).unwrap();
    let apply_args = [
        "reconcile",
        "--input",
        plan_path.to_str().unwrap(),
        "--expect",
        plan["target_revision"].as_str().unwrap(),
    ];
    let record = f.cli(&f.neutral, &apply_args);
    let record_id = record["id"].as_str().unwrap();
    assert_eq!(record["state"], "conflict");
    assert!(record["result_spec_oid"].is_null());
    assert_eq!(record["plan"], plan);
    assert_eq!(record["conflicts"], json!(["spec/lessons.md", OPTIONAL]));
    assert_eq!(
        record["stages"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["spec/lessons.md".to_owned(), OPTIONAL.to_owned()]
    );
    for stages in record["stages"].as_object().unwrap().values() {
        assert_eq!(stages.as_array().unwrap().len(), 3);
    }
    assert_eq!(managed_files(&selected), target_expected);
    assert_eq!(
        f.selected_context(project, &f.code)["spec_revision"],
        selected["spec_revision"],
        "conflicting apply advanced canonical target"
    );
    let mut proposed = target_expected.clone();
    proposed.insert(PLAN.into(), source_plan.as_bytes().into());
    let expected_nonconflicts: BTreeSet<_> = proposed
        .keys()
        .filter(|path| path.as_str() != "spec/lessons.md" && path.as_str() != OPTIONAL)
        .cloned()
        .collect();
    assert_eq!(
        record["merge_index"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        expected_nonconflicts
    );
    let workspace = f
        .store
        .join("spec/projects")
        .join(project)
        .join("reconciliations")
        .join(record_id)
        .join("workspace");
    let workspace_files = snapshot(&workspace);
    for path in &expected_nonconflicts {
        assert_eq!(
            workspace_files[path], proposed[path],
            "nonconflict workspace: {path}"
        );
    }
    assert_eq!(
        workspace_files
            .keys()
            .filter(|path| path.as_str() != ".git")
            .cloned()
            .collect::<BTreeSet<_>>(),
        proposed.keys().cloned().collect()
    );
    let mut tokens: Option<(Value, Value)> = None;
    for (path, base, source, target) in [
        (
            "spec/lessons.md",
            json!(text(&shared, "spec/lessons.md")),
            json!(SOURCE_LESSONS),
            json!(TARGET_LESSONS),
        ),
        (
            OPTIONAL,
            json!(BASE_OPTIONAL),
            Value::Null,
            json!(TARGET_OPTIONAL),
        ),
    ] {
        let read = f.cli(
            &f.neutral,
            &["reconcile", "read", record_id, path, "--project", project],
        );
        let remote = selected_client.call(
            "spec_reconciliation_read",
            json!({
                "project":project, "record_id":record_id, "path":path
            }),
        );
        assert_eq!(read, remote);
        assert_eq!(read["path"], path);
        assert_eq!(read["base"], base);
        assert_eq!(read["source"], source);
        assert_eq!(read["target"], target);
        if path == "spec/lessons.md" {
            assert!(read["proposed"].as_str().unwrap().contains("<<<<<<<"));
        } else {
            assert_eq!(read["proposed"], TARGET_OPTIONAL);
        }
        assert_ne!(read["target_revision"], read["resolution_revision"]);
        let current = (
            read["target_revision"].clone(),
            read["resolution_revision"].clone(),
        );
        if let Some(previous) = &tokens {
            assert_eq!(&current, previous);
        }
        tokens = Some(current);
    }
    assert_eq!(
        f.cli(
            &f.neutral,
            &["reconcile", "status", record_id, "--project", project]
        ),
        record
    );
    assert_eq!(CodeState::capture(&f.code), code_state);
    assert_eq!(CodeState::capture(&clone), clone_state);
    let (target_token, workspace_token) = tokens.unwrap();
    let bundle_path = f.owner.join("resolution.json");
    fs::write(
        &bundle_path,
        serde_json::to_vec(&json!({"files":[
            {"path":"spec/lessons.md","content":RESOLVED_LESSONS},
            {"path":OPTIONAL,"content":null}
        ]}))
        .unwrap(),
    )
    .unwrap();
    let resolved = f.cli(
        &f.neutral,
        &[
            "reconcile",
            "resolve",
            record_id,
            "--input",
            bundle_path.to_str().unwrap(),
            "--expect",
            target_token.as_str().unwrap(),
            "--expect-resolution",
            workspace_token.as_str().unwrap(),
            "--project",
            project,
        ],
    );
    assert_eq!(resolved["state"], "applied");
    assert_eq!(resolved["id"], record_id);
    assert_eq!(resolved["diagnostics"]["errors"], 0);
    let mut final_expected = target_expected;
    final_expected.insert("spec/lessons.md".into(), RESOLVED_LESSONS.as_bytes().into());
    final_expected.insert(PLAN.into(), source_plan.as_bytes().into());
    final_expected.remove(OPTIONAL);
    assert_eq!(managed_files(&selected), final_expected);
    assert_eq!(
        managed_files(&source),
        source_expected,
        "resolution altered retained source"
    );
    selected_client.close();

    let mut reopened = f.client(
        &f.neutral,
        &["--project", project, "--checkout", f.code.to_str().unwrap()],
    );
    assert_eq!(
        reopened.call(
            "spec_reconciliation_status",
            json!({
                "project":project, "record_id":record_id
            })
        ),
        resolved
    );
    let final_context = reopened.call("spec_context", json!({}));
    assert_context(&final_context, project, "global-first", &f.code, "main");
    assert_eq!(final_context["spec_revision"], resolved["result_spec_oid"]);
    assert_inventory(&reopened.call("spec_documents", json!({})), &final_expected);
    for path in final_expected.keys() {
        assert_eq!(
            reopened.call("spec_read", json!({"path":path}))["content"],
            text(&final_expected, path)
        );
    }
    assert_queries(&mut reopened, json!({}));
    reopened.close();
    assert_eq!(
        f.cli(
            &f.neutral,
            &["reconcile", "status", record_id, "--project", project]
        ),
        resolved
    );
    assert_eq!(
        f.read(&f.code, &[], "spec/lessons.md")["content"],
        RESOLVED_LESSONS
    );
    assert_eq!(f.cli(&f.code, &["validate", "--json"])["errors"], 0);
    assert!(
        !Path::new(final_context["spec_root"].as_str().unwrap())
            .join("optional.md")
            .exists()
    );
    let final_store = snapshot(&f.store);
    assert_eq!(
        f.cli(&f.neutral, &apply_args),
        resolved,
        "exact saved-plan retry changed its receipt"
    );
    assert_eq!(snapshot(&f.store), final_store);
    assert_eq!(CodeState::capture(&f.code), code_state);
    assert_eq!(CodeState::capture(&clone), clone_state);
    assert!(!f.code.join("docs").exists());
    assert!(!clone.join("docs").exists());
}
