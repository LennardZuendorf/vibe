#![allow(clippy::unwrap_used, clippy::expect_used)]
//! spec-projects-mcp/8: actual child stdio, never an in-memory protocol substitute.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::Duration,
};
struct Client {
    child: Child,
    input: Option<ChildStdin>,
    lines: mpsc::Receiver<String>,
    next: u64,
    modern: bool,
}
impl Client {
    fn new(
        cwd: &std::path::Path,
        store: &std::path::Path,
        modern: bool,
        defaults: &[&str],
    ) -> Self {
        Self::new_env(cwd, store, modern, defaults, &[])
    }
    fn new_env(
        cwd: &std::path::Path,
        store: &std::path::Path,
        modern: bool,
        defaults: &[&str],
        extra: &[(&str, &std::ffi::OsStr)],
    ) -> Self {
        let bin = code_bin(cwd);
        let mut paths = vec![bin];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let mut child = Command::new(env!("CARGO_BIN_EXE_vibe-spec"))
            .current_dir(cwd)
            .env("VIBE_STORE", store)
            .env("VIBE_DESIGN_LINT", "1")
            .env("PATH", std::env::join_paths(paths).unwrap())
            .envs(extra.iter().copied())
            .args(defaults)
            .args(["mcp", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if tx.send(line.unwrap()).is_err() {
                    break;
                }
            }
        });
        let mut c = Self {
            child,
            input,
            lines,
            next: 1,
            modern,
        };
        if modern {
            let discovered = c.rpc("server/discover", json!({}));
            assert!(
                discovered["result"]["supportedVersions"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("2026-07-28")),
                "{discovered}"
            );
        } else {
            let initialized=c.rpc("initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}));
            assert_eq!(
                initialized["result"]["protocolVersion"], "2025-11-25",
                "{initialized}"
            );
            c.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        }
        c
    }
    fn send(&mut self, v: Value) {
        writeln!(self.input.as_mut().unwrap(), "{v}").unwrap();
    }
    fn receive(&self) -> Value {
        let line = self
            .lines
            .recv_timeout(Duration::from_secs(20))
            .expect("streaming MCP response before EOF");
        serde_json::from_str(&line).expect("protocol-only stdout")
    }
    fn rpc(&mut self, method: &str, mut params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        if self.modern {
            params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}});
        }
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        let value = self.receive();
        assert_eq!(value["id"], id, "{value}");
        if let Some(path) = std::env::var_os("VIBE_MCP_TEST_TRANSCRIPT") {
            static WRITER: std::sync::Mutex<()> = std::sync::Mutex::new(());
            let _guard = WRITER.lock().unwrap();
            let mut out = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap();
            writeln!(
                out,
                "{}",
                json!({"method":method,"params":params,"response":value})
            )
            .unwrap();
        }
        value
    }
    fn call(&mut self, name: &str, args: Value) -> Value {
        self.rpc("tools/call", json!({"name":name,"arguments":args}))
    }
    fn close(mut self) {
        self.input.take();
        assert!(self.child.wait().unwrap().success());
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn code_bin(code: &std::path::Path) -> std::path::PathBuf {
    let bin = code.with_file_name("fake-bin");
    std::fs::create_dir_all(&bin).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for name in ["curl", "npx"] {
            let script = bin.join(name);
            std::fs::write(
                &script,
                b"#!/bin/sh\nprintf called > \"$0.called\"\nexit 73\n",
            )
            .unwrap();
            std::fs::set_permissions(script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    bin
}
fn fixture() -> (
    vibe_core::testscratch::Scratch,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let s = vibe_core::testscratch::scratch("vibe-spec", "mcp");
    let code = s.join("code");
    let store = s.join("store");
    std::fs::create_dir(&code).unwrap();
    std::fs::create_dir(&store).unwrap();
    (s, code, store)
}
fn data(v: &Value) -> &Value {
    assert!(v.get("error").is_none(), "{v}");
    assert_ne!(v["result"]["isError"], true, "{v}");
    &v["result"]["structuredContent"]["data"]
}
#[test]
fn both_lifecycles_list_complete_tools_and_reject_bad_arguments() {
    let (_s, code, store) = fixture();
    for modern in [false, true] {
        let mut c = Client::new(&code, &store, modern, &[]);
        let listed = c.rpc("tools/list", json!({}));
        let tools = listed["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 28);
        for tool in tools {
            assert_eq!(tool["inputSchema"]["type"], "object");
            assert_eq!(tool["inputSchema"]["additionalProperties"], false);
            assert_eq!(tool["outputSchema"]["type"], "object");
            assert_eq!(tool["annotations"]["openWorldHint"], false);
        }
        assert_eq!(
            data(&c.call("spec_projects", json!({})))["projects"],
            json!([])
        );
        for (tool, args) in [
            ("spec_projects", json!({"argv":["--help"]})),
            (
                "spec_write",
                json!({"path":"spec/product.md","content":42,"expected_revision":"x"}),
            ),
            ("spec_context", json!({"checkout":code,"branch":"main"})),
            ("spec_lessons", json!({"tags":[]})),
        ] {
            let r = c.call(tool, args);
            assert_eq!(r["result"]["isError"], true, "{r}");
            assert_eq!(
                r["result"]["structuredContent"]["error"]["code"],
                "invalid_arguments"
            );
        }
        assert!(c.call("not_a_tool", json!({})).get("error").is_some());
        c.close();
    }
}
#[test]
fn document_lifecycle_keeps_two_clients_isolated_and_refuses_stale_writes() {
    let (_s, code, store) = fixture();
    let other = code.with_file_name("other");
    std::fs::create_dir(&other).unwrap();
    let mut a = Client::new(&code, &store, false, &[]);
    let initialized = a.call(
        "spec_project_init",
        json!({"mode":"repo-first","expected_revision":"absent"}),
    );
    let project = data(&initialized)["project"]["id"].clone();
    let revision = data(&a.call("spec_projects", json!({})))["revision"].clone();
    let mut b = Client::new(&other, &store, true, &[]);
    let initialized = b.call(
        "spec_project_init",
        json!({"mode":"repo-first","expected_revision":revision}),
    );
    let other_project = data(&initialized)["project"]["id"].clone();
    assert_ne!(project, other_project);
    let before = a.call("spec_context", json!({"project":project}));
    let token = data(&before)["revision_token"].clone();
    let changed=a.call("spec_write",json!({"project":project,"path":"spec/concepts/new.md","content":"# Héllo 🐦\n","expected_revision":token}));
    data(&changed);
    let stale=a.call("spec_write",json!({"project":project,"path":"spec/concepts/new.md","content":"stale","expected_revision":token}));
    assert_eq!(
        stale["result"]["structuredContent"]["error"]["code"],
        "revision_changed"
    );
    assert_eq!(
        data(&a.call(
            "spec_read",
            json!({"project":project,"path":"spec/concepts/new.md"})
        ))["content"],
        "# Héllo 🐦\n"
    );
    assert_eq!(
        b.call(
            "spec_read",
            json!({"project":other_project,"path":"spec/concepts/new.md"})
        )["result"]["isError"],
        true
    );
    for tool in [
        "spec_project_show",
        "spec_documents",
        "spec_validate",
        "spec_list_specs",
        "spec_scan_merges",
        "spec_export",
        "spec_drift",
    ] {
        data(&a.call(tool, json!({"project":project})));
    }
    a.close();
    b.close();
}
#[test]
fn malformed_json_oversized_lines_and_eof_keep_sdk_framing() {
    let (_s, code, store) = fixture();
    let mut c = Client::new(&code, &store, false, &[]);
    writeln!(c.input.as_mut().unwrap(), "{{broken").unwrap();
    writeln!(c.input.as_mut().unwrap(), "{}", "x".repeat(8 * 1024 * 1024)).unwrap();
    let mut exact = json!({"jsonrpc":"2.0","id":900,"method":"tools/call","params":{"name":"spec_projects","arguments":{"__pad":""}}});
    let overhead = serde_json::to_vec(&exact).unwrap().len() + 1;
    exact["params"]["arguments"]["__pad"] = json!("x".repeat(8 * 1024 * 1024 - overhead));
    assert_eq!(
        serde_json::to_vec(&exact).unwrap().len() + 1,
        8 * 1024 * 1024
    );
    c.send(exact.clone());
    let accepted = c.receive();
    assert_eq!(accepted["id"], 900);
    assert_eq!(
        accepted["result"]["structuredContent"]["error"]["code"],
        "invalid_arguments"
    );
    exact["id"] = json!(901);
    let oversized = exact["params"]["arguments"]["__pad"]
        .as_str()
        .unwrap()
        .to_owned()
        + "x";
    exact["params"]["arguments"]["__pad"] = json!(oversized);
    c.send(exact);
    c.input.as_mut().unwrap().write_all(&[0xff, b'\n']).unwrap();
    c.send(json!({"jsonrpc":"2.0","id":903,"method":12}));
    let malformed = c.receive();
    assert_eq!(malformed["error"]["code"], -32600);
    assert!(malformed["id"].is_null());
    assert_eq!(
        data(&c.call("spec_projects", json!({})))["projects"],
        json!([])
    );
    c.input
        .as_mut()
        .unwrap()
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":999")
        .unwrap();
    c.close();
}

fn git(code: &std::path::Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(code)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "maintenance.auto=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
fn write(c: &mut Client, project: &Value, branch: &str, path: &str, content: &str) {
    let context = c.call("spec_context", json!({"project":project,"branch":branch}));
    let token = data(&context)["revision_token"].clone();
    data(&c.call("spec_write",json!({"project":project,"branch":branch,"path":path,"content":content,"expected_revision":token})));
}
#[test]
fn complete_conflict_journey_uses_exact_saved_plan_and_separate_workspace_token() {
    let (_s, code, store) = fixture();
    git(&code, &["init", "-b", "main"]);
    git(
        &code,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    let mut c = Client::new(&code, &store, true, &[]);
    let init = c.call(
        "spec_project_init",
        json!({"mode":"global-first","expected_revision":"absent"}),
    );
    let project = data(&init)["project"]["id"].clone();
    write(
        &mut c,
        &project,
        "main",
        "spec/lessons.md",
        "# Lessons\n\nBase.\n",
    );
    write(
        &mut c,
        &project,
        "main",
        "spec/optional.md",
        "# Optional\n\nBase.\n",
    );
    let show = c.call("spec_project_show", json!({"project":project}));
    data(&c.call("spec_branch_init",json!({"project":project,"name":"topic","from":"main","expected_revision":data(&show)["revision"],"expected_project_revision":data(&show)["project_revision"]})));
    write(
        &mut c,
        &project,
        "topic",
        "spec/lessons.md",
        "# Lessons\n\nSource.\n",
    );
    write(
        &mut c,
        &project,
        "main",
        "spec/lessons.md",
        "# Lessons\n\nTarget.\n",
    );
    let source_context = c.call("spec_context", json!({"project":project,"branch":"topic"}));
    data(&c.call("spec_remove",json!({"project":project,"branch":"topic","path":"spec/optional.md","expected_revision":data(&source_context)["revision_token"]})));
    write(
        &mut c,
        &project,
        "main",
        "spec/optional.md",
        "# Optional\n\nTarget.\n",
    );
    let preview = c.call(
        "spec_reconcile",
        json!({"project":project,"source":"topic","target":"main","dry_run":true}),
    );
    let stale_plan = data(&preview).clone();
    write(
        &mut c,
        &project,
        "topic",
        "spec/lessons.md",
        "# Lessons\n\nSource advanced.\n",
    );
    let stale = c.call(
        "spec_reconcile",
        json!({"plan":stale_plan,"expected_revision":stale_plan["target_revision"]}),
    );
    assert_eq!(
        stale["result"]["structuredContent"]["error"]["code"], "revision_changed",
        "{stale}"
    );
    let preview = c.call(
        "spec_reconcile",
        json!({"project":project,"source":"topic","target":"main","dry_run":true}),
    );
    let plan = data(&preview).clone();
    let applied = c.call(
        "spec_reconcile",
        json!({"plan":plan,"expected_revision":plan["target_revision"]}),
    );
    let record = data(&applied).clone();
    assert_eq!(record["state"], "conflict");
    let conflict = c.call(
        "spec_reconciliation_read",
        json!({"project":project,"record_id":record["id"],"path":"spec/lessons.md"}),
    );
    let conflict = data(&conflict).clone();
    assert_eq!(conflict["base"], "# Lessons\n\nBase.\n");
    assert_eq!(conflict["target"], "# Lessons\n\nTarget.\n");
    assert!(conflict["proposed"].as_str().unwrap().contains("<<<<<<<"));
    let bad=c.call("spec_reconciliation_resolve",json!({"project":project,"record_id":record["id"],"expected_target_revision":conflict["resolution_revision"],"expected_resolution_revision":conflict["target_revision"],"bundle":{"files":[{"path":"spec/lessons.md","content":"# Lessons\n\nResolved.\n"}]}}));
    assert_eq!(bad["result"]["isError"], true);
    let deletion = c.call(
        "spec_reconciliation_read",
        json!({"project":project,"record_id":record["id"],"path":"spec/optional.md"}),
    );
    assert!(data(&deletion)["source"].is_null());
    let resolved=c.call("spec_reconciliation_resolve",json!({"project":project,"record_id":record["id"],"expected_target_revision":conflict["target_revision"],"expected_resolution_revision":conflict["resolution_revision"],"bundle":{"files":[{"path":"spec/lessons.md","content":"# Lessons\n\nResolved.\n"},{"path":"spec/optional.md","content":null}]}}));
    assert_eq!(data(&resolved)["state"], "applied", "{resolved}");
    let status = c.call(
        "spec_reconciliation_status",
        json!({"project":project,"record_id":record["id"]}),
    );
    assert_eq!(
        data(&status)["result_spec_oid"],
        data(&resolved)["result_spec_oid"]
    );
    assert_eq!(
        data(&c.call(
            "spec_read",
            json!({"project":project,"branch":"main","path":"spec/lessons.md"})
        ))["content"],
        "# Lessons\n\nResolved.\n"
    );
    data(&c.call("spec_refresh", json!({"project":project,"branch":"main"})));
    assert_eq!(
        c.call(
            "spec_read",
            json!({"project":project,"branch":"main","path":"spec/optional.md"})
        )["result"]["structuredContent"]["error"]["code"],
        "document_not_found"
    );
    c.close();
}

#[test]
fn attach_uses_launch_checkout_and_returns_exact_metadata_schema() {
    let (_s, code, store) = fixture();
    let other = code.with_file_name("attach");
    std::fs::create_dir(&other).unwrap();
    let mut c = Client::new(&code, &store, false, &[]);
    let init = c.call(
        "spec_project_init",
        json!({"mode":"repo-first","expected_revision":"absent"}),
    );
    let project = data(&init)["project"]["id"].as_str().unwrap().to_owned();
    let rev = data(&init)["revision"].clone();
    c.close();
    let mut c = Client::new(
        &code,
        &store,
        false,
        &["--checkout", other.to_str().unwrap()],
    );
    let attached = c.call(
        "spec_project_attach",
        json!({"project":project,"expected_revision":rev}),
    );
    let result = data(&attached);
    assert!(attached["result"]["structuredContent"]["context"].is_null());
    assert!(result.get("context").is_none());
    assert!(
        result["project"]["checkouts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["path"] == json!(std::fs::canonicalize(&other).unwrap()))
    );
    c.close();
}

#[test]
fn escaped_outputs_and_request_id_budget_refuse_without_truncating_or_publishing() {
    let (_s, code, store) = fixture();
    git(&code, &["init", "-b", "main"]);
    git(
        &code,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    let mut c = Client::new(&code, &store, false, &[]);
    let init = c.call(
        "spec_project_init",
        json!({"mode":"global-first","expected_revision":"absent"}),
    );
    let project = data(&init)["project"]["id"].clone();
    let context = c.call("spec_context", json!({"project":project}));
    let token = data(&context)["revision_token"].clone();
    let oid = data(&context)["spec_revision"].clone();
    let id = "q".repeat(8 * 1024 * 1024 - 1500);
    c.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"spec_write","arguments":{"project":project,"path":"spec/oversize-result.md","content":"# Never published\n","expected_revision":token}}}));
    let refusal = c.receive();
    assert_eq!(refusal["id"].as_str().unwrap().len(), id.len());
    assert_eq!(
        refusal["result"]["structuredContent"]["error"]["code"],
        "size_limit"
    );
    assert!(serde_json::to_vec(&refusal).unwrap().len() < 8 * 1024 * 1024);
    assert_eq!(
        c.call(
            "spec_read",
            json!({"project":project,"path":"spec/oversize-result.md"})
        )["result"]["structuredContent"]["error"]["code"],
        "document_not_found"
    );
    assert_eq!(
        data(&c.call("spec_context", json!({"project":project})))["spec_revision"],
        oid
    );
    // Fits the incoming frame, but text+structured JSON escaping cannot fit output.
    write(
        &mut c,
        &project,
        "main",
        "spec/escaped.md",
        &format!("# Escapes\n{}", "\"".repeat(3 * 1024 * 1024)),
    );
    let refused = c.call(
        "spec_read",
        json!({"project":project,"path":"spec/escaped.md"}),
    );
    assert_eq!(
        refused["result"]["structuredContent"]["error"]["code"],
        "size_limit"
    );
    let context = c.call("spec_context", json!({"project":project}));
    let root = std::path::PathBuf::from(data(&context)["spec_root"].as_str().unwrap());
    std::fs::write(root.join("too-large.md"), vec![b'a'; 8 * 1024 * 1024 + 1]).unwrap();
    let context = c.call("spec_context", json!({"project":project}));
    let before = data(&context)["spec_revision"].clone();
    let refused = c.call(
        "spec_snapshot",
        json!({"project":project,"expected_revision":data(&context)["revision_token"]}),
    );
    assert_eq!(
        refused["result"]["structuredContent"]["error"]["code"],
        "size_limit"
    );
    assert_eq!(
        data(&c.call("spec_context", json!({"project":project})))["spec_revision"],
        before
    );
    c.close();
}

#[test]
fn every_lifecycle_tool_returns_its_typed_result_and_disables_external_lint() {
    let (_s, code, store) = fixture();
    git(&code, &["init", "-b", "main"]);
    git(
        &code,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    let mut c = Client::new(&code, &store, true, &[]);
    let init = c.call(
        "spec_project_init",
        json!({"mode":"global-first","expected_revision":"absent"}),
    );
    let project = data(&init)["project"]["id"].clone();
    let ctx = c.call("spec_context", json!({"project":project}));
    data(&c.call(
        "spec_init",
        json!({"project":project,"expected_revision":data(&ctx)["revision_token"]}),
    ));
    write(
        &mut c,
        &project,
        "main",
        "spec/features/sample/product.md",
        "---\ntype: feature-product\nfeature: sample\nsibling: tech.md\nparent: ../../product.md\nupdated: 2026-11-12\n---\n\n# Sample\n\n## Scope\n\n| Owns | Does not own |\n|---|---|\n| Sample | Other |\n",
    );
    write(
        &mut c,
        &project,
        "main",
        "spec/features/sample/tech.md",
        "---\ntype: feature-tech\nfeature: sample\nsibling: product.md\nparent: ../../tech.md\nupdated: 2026-11-12\n---\n\n# Sample tech\n\n<!-- merge -->\nSample records a local choice.\n<!-- /merge -->\n",
    );
    write(
        &mut c,
        &project,
        "main",
        "spec/features/sample/plan.md",
        "---\ntype: feature-plan\nfeature: sample\nparent: ../../plan.md\nupdated: 2026-11-12\n---\n\n# Plan\n\n| Unit | Status |\n|---|---|\n| sample/1 | DONE |\n\n### sample/1 — Record a choice\n\n**Files:** local\n**What to do:** Record.\n**Verify:** Read.\n",
    );
    data(&c.call(
        "spec_feature",
        json!({"project":project,"feature":"sample"}),
    ));
    assert_eq!(
        data(&c.call("spec_plan", json!({"project":project,"feature":"sample"})))["rows"][0]["id"],
        "sample/1"
    );
    data(&c.call("spec_lessons", json!({"project":project,"tags":["mcp"]})));
    data(&c.call("spec_trace", json!({"project":project})));
    let ctx = c.call("spec_context", json!({"project":project}));
    data(&c.call(
        "spec_snapshot",
        json!({"project":project,"expected_revision":data(&ctx)["revision_token"]}),
    ));
    let exported = c.call("spec_export", json!({"project":project}));
    data(&exported);
    let bundle = json!({"files":[{"path":"openspec/changes/copied/proposal.md","content":"# Proposal\n\n## Why\nRecord a copied choice.\n"},{"path":"openspec/changes/copied/design.md","content":"# Copied design\n"}]});
    let ctx = c.call("spec_context", json!({"project":project}));
    data(&c.call(
        "spec_import",
        json!({"project":project,"expected_revision":data(&ctx)["revision_token"],"bundle":bundle}),
    ));
    let ctx = c.call("spec_context", json!({"project":project}));
    data(&c.call("spec_promote",json!({"project":project,"feature":"sample","expected_revision":data(&ctx)["revision_token"],"dry_run":true})));
    let ctx = c.call("spec_context", json!({"project":project}));
    data(&c.call("spec_promote",json!({"project":project,"feature":"sample","expected_revision":data(&ctx)["revision_token"]})));
    let checked = c.call("spec_validate", json!({"project":project}));
    assert!(
        data(&checked)["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "external_lint_disabled")
    );
    write(
        &mut c,
        &project,
        "main",
        "spec/temporary.md",
        "# Temporary\n",
    );
    let ctx = c.call("spec_context", json!({"project":project}));
    data(&c.call("spec_remove",json!({"project":project,"path":"spec/temporary.md","expected_revision":data(&ctx)["revision_token"]})));
    c.close();
    assert!(!code_bin(&code).join("curl.called").exists());
    assert!(!code_bin(&code).join("npx.called").exists());
}

#[test]
fn cancelled_write_wait_and_published_write_are_observable_from_reopened_client() {
    let (_s, code, store) = fixture();
    git(&code, &["init", "-b", "main"]);
    git(
        &code,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    let mut c = Client::new(&code, &store, false, &[]);
    let init = c.call(
        "spec_project_init",
        json!({"mode":"global-first","expected_revision":"absent"}),
    );
    let project = data(&init)["project"]["id"].clone();
    let context = c.call("spec_context", json!({"project":project}));
    let context = data(&context).clone();
    let root = store.join("spec/projects").join(project.as_str().unwrap());
    let guard =
        match vibe_core::lock::Lock::acquire(&root.join("project.lock"), vibe_core::lock::STALE)
            .unwrap()
        {
            vibe_core::lock::Acquired::Got(g) => g,
            _ => panic!("fixture lock"),
        };
    c.send(json!({"jsonrpc":"2.0","id":600,"method":"tools/call","params":{"name":"spec_write","arguments":{"project":project,"path":"spec/cancel-before.md","content":"# Cancelled\n","expected_revision":context["revision_token"]}}}));
    std::thread::sleep(Duration::from_millis(100));
    c.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":600}}));
    std::thread::sleep(Duration::from_millis(30));
    drop(guard);
    let mut reopened = Client::new(&code, &store, true, &[]);
    assert_eq!(
        reopened.call(
            "spec_read",
            json!({"project":project,"path":"spec/cancel-before.md"})
        )["result"]["structuredContent"]["error"]["code"],
        "document_not_found"
    );
    let fresh = reopened.call("spec_context", json!({"project":project}));
    assert_eq!(data(&fresh)["spec_revision"], context["spec_revision"]);
    let show = reopened.call("spec_project_show", json!({"project":project}));
    let reference = data(&show)["branches"][0]["spec_ref"].as_str().unwrap();
    let reference = root.join("history.git").join(reference);
    c.send(json!({"jsonrpc":"2.0","id":601,"method":"tools/call","params":{"name":"spec_write","arguments":{"project":project,"path":"spec/cancel-after.md","content":"# Durable\n","expected_revision":data(&fresh)["revision_token"]}}}));
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if std::fs::read_to_string(&reference)
            .is_ok_and(|s| s.trim() != context["spec_revision"].as_str().unwrap())
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "publication before deadline"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    c.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":601}}));
    assert_eq!(
        data(&reopened.call(
            "spec_read",
            json!({"project":project,"path":"spec/cancel-after.md"})
        ))["content"],
        "# Durable\n"
    );
    let committed =
        data(&reopened.call("spec_context", json!({"project":project})))["spec_revision"].clone();
    assert_ne!(committed, context["spec_revision"]);
    assert_eq!(
        std::fs::read_to_string(reference).unwrap().trim(),
        committed.as_str().unwrap()
    );
    assert!(
        !std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().starts_with("transaction-"))
    );
    reopened.close();
    c.close();
}

#[cfg(unix)]
#[test]
fn cancelled_started_worker_retains_admission_until_synchronous_git_returns() {
    use std::os::unix::fs::PermissionsExt;
    let (owner, code, store) = fixture();
    let mut c = Client::new(&code, &store, false, &[]);
    let init = c.call(
        "spec_project_init",
        json!({"mode":"repo-first","expected_revision":"absent"}),
    );
    let project = data(&init)["project"]["id"].clone();
    let gate = owner.join("git-gate");
    let entered = owner.join("git-entered");
    std::fs::write(&gate, b"owned fixture gate").unwrap();
    let quoted = |p: &std::path::Path| format!("'{}'", p.to_str().unwrap().replace('\'', "'\\''"));
    let script = format!(
        "#!/bin/sh\nif [ -f {} ]; then\n case \" $* \" in *\" --show-toplevel \"*) printf x >> {}; while [ -f {} ]; do /bin/sleep 0.02; done;; esac\nfi\nexec /usr/bin/git \"$@\"\n",
        quoted(&gate),
        quoted(&entered),
        quoted(&gate)
    );
    let git = code_bin(&code).join("git");
    std::fs::write(&git, script).unwrap();
    std::fs::set_permissions(&git, std::fs::Permissions::from_mode(0o755)).unwrap();
    c.send(json!({"jsonrpc":"2.0","id":100,"method":"tools/call","params":{"name":"spec_context","arguments":{"project":project}}}));
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !entered.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "worker enters actual child Git"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    for id in 101..132 {
        c.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"spec_context","arguments":{"project":project}}}));
    }
    c.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":100}}));
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut probe = 132;
    loop {
        c.send(json!({"jsonrpc":"2.0","id":probe,"method":"tools/call","params":{"name":"spec_projects","arguments":{}}}));
        let mut rejected = Vec::new();
        let excess = loop {
            let response = c.receive();
            if response["id"] == probe {
                break response;
            }
            let id = response["id"].as_u64().unwrap();
            assert!((101..132).contains(&id));
            assert_eq!(
                response["result"]["structuredContent"]["error"]["code"],
                "busy"
            );
            rejected.push(id);
        };
        for id in &rejected {
            c.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"spec_context","arguments":{"project":project}}}));
        }
        if excess["result"]["structuredContent"]["error"]["code"] == "busy" {
            assert!(rejected.is_empty());
            break;
        }
        assert_ne!(excess["result"]["isError"], true);
        assert!(
            std::time::Instant::now() < deadline,
            "all32 admitted while one worker is held"
        );
        probe += 1;
    }
    assert_eq!(
        std::fs::read(&entered).unwrap(),
        b"x",
        "only one actual project operation may reach Git while31 wait"
    );
    std::fs::remove_file(gate).unwrap();
    let mut completed = std::collections::BTreeSet::new();
    for _ in 0..31 {
        let result = c.receive();
        let id = result["id"].as_u64().unwrap();
        assert!((101..132).contains(&id));
        assert_ne!(result["result"]["isError"], true, "{result}");
        assert!(completed.insert(id));
    }
    assert_eq!(completed, (101..132).collect());
    data(&c.call("spec_projects", json!({})));
    c.close();
}

#[test]
fn oversized_management_result_refuses_before_retaining_unfinishable_setup() {
    let (_s, code, store) = fixture();
    let mut c = Client::new(&code, &store, false, &[]);
    let refused = c.call(
        "spec_project_init",
        json!({"mode":"repo-first","name":"n".repeat(5*1024*1024),"expected_revision":"absent"}),
    );
    assert_eq!(
        refused["result"]["structuredContent"]["error"]["code"],
        "size_limit"
    );
    assert_eq!(
        data(&c.call("spec_projects", json!({})))["projects"],
        json!([])
    );
    data(&c.call(
        "spec_project_init",
        json!({"mode":"repo-first","name":"Admitted","expected_revision":"absent"}),
    ));
    c.close();
}

#[test]
fn launch_selection_is_immutable_and_call_project_or_branch_replaces_anchors() {
    let (_s, code, store) = fixture();
    git(&code, &["init", "-b", "main"]);
    git(
        &code,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    let mut c = Client::new(&code, &store, false, &[]);
    let init = c.call(
        "spec_project_init",
        json!({"mode":"global-first","expected_revision":"absent"}),
    );
    let project = data(&init)["project"]["id"].as_str().unwrap().to_owned();
    let show = c.call("spec_project_show", json!({"project":project}));
    data(&c.call("spec_branch_init",json!({"project":project,"name":"topic","from":"main","expected_revision":data(&show)["revision"],"expected_project_revision":data(&show)["project_revision"]})));
    c.close();
    let other = code.with_file_name("other");
    std::fs::create_dir(&other).unwrap();
    let mut other_client = Client::new(&other, &store, false, &[]);
    let projects = other_client.call("spec_projects", json!({}));
    let other_init = other_client.call(
        "spec_project_init",
        json!({"mode":"repo-first","expected_revision":data(&projects)["revision"]}),
    );
    let other_project = data(&other_init)["project"]["id"].clone();
    other_client.close();
    let mut c = Client::new(
        &code,
        &store,
        true,
        &["--project", &project, "--checkout", code.to_str().unwrap()],
    );
    let topic = c.call("spec_context", json!({"branch":"topic"}));
    assert_eq!(data(&topic)["code_ref"], "refs/heads/topic");
    assert_eq!(data(&topic)["code_available"], false);
    assert!(data(&topic)["checkout"].is_null());
    let other = c.call("spec_context", json!({"project":other_project}));
    assert_eq!(data(&other)["project_id"], other_project);
    let original = c.call("spec_context", json!({}));
    assert_eq!(data(&original)["project_id"], project);
    assert_eq!(data(&original)["code_ref"], "refs/heads/main");
    let token = data(&original)["revision_token"].clone();
    git(&code, &["checkout", "-b", "topic"]);
    let stale = c.call(
        "spec_write",
        json!({"path":"spec/moved.md","content":"# Stale selection\n","expected_revision":token}),
    );
    assert!(
        matches!(
            stale["result"]["structuredContent"]["error"]["code"].as_str(),
            Some("context_changed" | "revision_changed")
        ),
        "{stale}"
    );
    assert!(!code.join("docs/spec/moved.md").exists());
    c.close();
}

#[test]
fn promotion_checklist_and_completion_preview_apply_have_distinct_exact_contracts() {
    let (_s, code, store) = fixture();
    git(&code, &["init", "-b", "main"]);
    git(
        &code,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    let mut c = Client::new(&code, &store, true, &[]);
    let init = c.call(
        "spec_project_init",
        json!({"mode":"global-first","expected_revision":"absent"}),
    );
    let project = data(&init)["project"]["id"].clone();
    write(
        &mut c,
        &project,
        "main",
        "superpowers/specs/2026-11-12-checklist-design.md",
        "# Checklist design\n\nRecord the chosen behavior.\n",
    );
    for preview in [true, false] {
        let ctx = c.call("spec_context", json!({"project":project}));
        let promoted=c.call("spec_promote",json!({"project":project,"feature":"checklist","expected_revision":data(&ctx)["revision_token"],"dry_run":preview}));
        assert!(data(&promoted).get("checklist").is_some());
        assert_eq!(data(&promoted).get("receipt").is_none(), preview);
    }
    write(
        &mut c,
        &project,
        "main",
        "spec/lessons.md",
        "# Lessons\n\nRecorded the chosen behavior.\n",
    );
    for preview in [true, false] {
        let ctx = c.call("spec_context", json!({"project":project}));
        let promoted=c.call("spec_promote",json!({"project":project,"feature":"checklist","expected_revision":data(&ctx)["revision_token"],"dry_run":preview,"done":true}));
        assert!(data(&promoted)["baseline"].is_string());
        assert_eq!(data(&promoted).get("receipt").is_none(), preview);
    }
    c.close();
}

#[test]
fn two_stdio_processes_racing_the_same_revision_publish_exactly_one_write() {
    let (_s, code, store) = fixture();
    git(&code, &["init", "-b", "main"]);
    git(
        &code,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    let mut a = Client::new(&code, &store, false, &[]);
    let init = a.call(
        "spec_project_init",
        json!({"mode":"global-first","expected_revision":"absent"}),
    );
    let project = data(&init)["project"]["id"].clone();
    let mut b = Client::new(&code, &store, false, &[]);
    let context = a.call("spec_context", json!({"project":project}));
    let token = data(&context)["revision_token"].clone();
    for (client, id, content) in [(&mut a, 700, "# First\n"), (&mut b, 800, "# Second\n")] {
        client.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"spec_write","arguments":{"project":project,"path":"spec/race.md","content":content,"expected_revision":token}}}));
    }
    let first = a.receive();
    let second = b.receive();
    let a_won = first["result"]["isError"] == false;
    assert_ne!(
        a_won,
        second["result"]["isError"] == false,
        "exactly one writer wins"
    );
    let loser = if a_won { &second } else { &first };
    assert_eq!(
        loser["result"]["structuredContent"]["error"]["code"],
        "revision_changed"
    );
    assert_eq!(
        data(&a.call(
            "spec_read",
            json!({"project":project,"path":"spec/race.md"})
        ))["content"],
        if a_won { "# First\n" } else { "# Second\n" }
    );
    a.close();
    b.close();
}

#[cfg(unix)]
#[test]
fn quote_heavy_management_result_is_admitted_before_publication_with_native_paths() {
    use std::os::unix::ffi::OsStrExt;
    fn setup() -> (vibe_core::testscratch::Scratch, Client, Value) {
        let owner = vibe_core::testscratch::scratch("vibe-spec", "wire-management");
        let code = owner.join("co\"de");
        let store = owner.join("st\"ore");
        std::fs::create_dir(&code).unwrap();
        std::fs::create_dir(&store).unwrap();
        git(&code, &["init", "-b", "main"]);
        git(
            &code,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "fixture",
            ],
        );
        let mut c = Client::new(&code, &store, true, &[]);
        let initialized=c.call("spec_project_init",json!({"mode":"global-first","checkout":{"unix_bytes":code.as_os_str().as_bytes()},"expected_revision":"absent"}));
        let project = data(&initialized)["project"]["id"].clone();
        let show = c.call("spec_project_show", json!({"project":project}));
        let plain = std::iter::repeat_n("p".repeat(200), 20)
            .collect::<Vec<_>>()
            .join("/");
        data(&c.call("spec_branch_init",json!({"project":project,"name":plain,"from":"main","expected_revision":data(&show)["revision"],"expected_project_revision":data(&show)["project_revision"]})));
        (owner, c, project)
    }
    fn replace(value: &mut Value, old: &str, new: &str, old_id: &str, new_id: &str) {
        match value {
            Value::String(s) => *s = s.replace(old, new).replace(old_id, new_id),
            Value::Array(a) => {
                for v in a {
                    replace(v, old, new, old_id, new_id)
                }
            }
            Value::Object(o) => {
                for v in o.values_mut() {
                    replace(v, old, new, old_id, new_id)
                }
            }
            _ => {}
        }
    }
    let (first_owner, mut first, first_project) = setup();
    let quoted = std::iter::repeat_n("\"".repeat(200), 15)
        .collect::<Vec<_>>()
        .join("/");
    let show = first.call("spec_project_show", json!({"project":first_project}));
    let mut planned=first.call("spec_branch_init",json!({"project":first_project,"name":quoted,"from":"main","expected_revision":data(&show)["revision"],"expected_project_revision":data(&show)["project_revision"]}));
    data(&planned);
    first.close();
    let (second_owner, mut second, project) = setup();
    replace(
        &mut planned,
        first_owner.to_str().unwrap(),
        second_owner.to_str().unwrap(),
        first_project.as_str().unwrap(),
        project.as_str().unwrap(),
    );
    let prefix = "request-\"\\";
    planned["id"] = json!(prefix);
    let length = serde_json::to_vec(&planned).unwrap().len() + 1;
    let id = prefix.to_owned() + &"r".repeat(8 * 1024 * 1024 - length + 1);
    planned["id"] = json!(id);
    assert_eq!(
        serde_json::to_vec(&planned).unwrap().len() + 1,
        8 * 1024 * 1024 + 1
    );
    let before = second.call("spec_project_show", json!({"project":project}));
    let request = json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}},"name":"spec_branch_init","arguments":{"project":project,"name":quoted,"from":"main","expected_revision":data(&before)["revision"],"expected_project_revision":data(&before)["project_revision"]}}});
    assert!(serde_json::to_vec(&request).unwrap().len() < 8 * 1024 * 1024);
    second.send(request);
    let refused = second.receive();
    assert_eq!(refused["id"], id);
    assert_eq!(
        refused["result"]["structuredContent"]["error"]["code"],
        "size_limit"
    );
    let after = second.call("spec_project_show", json!({"project":project}));
    assert_eq!(
        data(&after)["project_revision"],
        data(&before)["project_revision"],
        "refusal must precede branch metadata publication"
    );
    second.close();
}

#[cfg(unix)]
#[test]
fn omitted_project_discovery_keeps_async_service_responsive_and_retains_permits() {
    use std::os::unix::fs::PermissionsExt;
    let (owner, code, store) = fixture();
    let mut c = Client::new(&code, &store, false, &[]);
    let initialized = c.call(
        "spec_project_init",
        json!({"mode":"repo-first","expected_revision":"absent"}),
    );
    let project = data(&initialized)["project"]["id"].clone();
    let gate = owner.join("discovery-gate");
    let entered = owner.join("discovery-entered");
    std::fs::write(&gate, b"owned fixture gate").unwrap();
    let quote = |p: &std::path::Path| format!("'{}'", p.to_str().unwrap().replace('\'', "'\\''"));
    let script = format!(
        "#!/bin/sh\nif [ -f {} ]; then case \" $* \" in *\" --show-toplevel \"*) printf x >> {}; while [ -f {} ]; do /bin/sleep 0.02; done;; esac; fi\nexec /usr/bin/git \"$@\"\n",
        quote(&gate),
        quote(&entered),
        quote(&gate)
    );
    let git = code_bin(&code).join("git");
    std::fs::write(&git, script).unwrap();
    std::fs::set_permissions(git, std::fs::Permissions::from_mode(0o755)).unwrap();
    for id in [100, 101] {
        c.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"spec_context","arguments":{}}}));
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !std::fs::read(&entered).is_ok_and(|b| b.len() >= 2) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    c.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":100}}));
    c.send(json!({"jsonrpc":"2.0","id":200,"method":"tools/call","params":{"name":"spec_projects","arguments":{}}}));
    let responsive = c.lines.recv_timeout(Duration::from_secs(2)).ok();
    if responsive.is_none() {
        std::fs::remove_file(&gate).unwrap();
        c.close();
        panic!("two pure Git discoveries must not occupy async service workers");
    }
    let responsive: Value = serde_json::from_str(responsive.as_deref().unwrap()).unwrap();
    assert_eq!(responsive["id"], 200);
    data(&responsive);
    for id in 102..132 {
        c.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"spec_context","arguments":{"project":project}}}));
    }
    let mut probe = 132;
    loop {
        c.send(json!({"jsonrpc":"2.0","id":probe,"method":"tools/call","params":{"name":"spec_projects","arguments":{}}}));
        let mut rejected = Vec::new();
        let response = loop {
            let result = c.receive();
            if result["id"] == probe {
                break result;
            }
            let id = result["id"].as_u64().unwrap();
            assert!((102..132).contains(&id));
            assert_eq!(
                result["result"]["structuredContent"]["error"]["code"],
                "busy"
            );
            rejected.push(id);
        };
        for id in &rejected {
            c.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"spec_context","arguments":{"project":project}}}));
        }
        if response["result"]["structuredContent"]["error"]["code"] == "busy" {
            assert!(rejected.is_empty());
            break;
        }
        data(&response);
        assert!(std::time::Instant::now() < deadline);
        probe += 1;
    }
    std::fs::remove_file(gate).unwrap();
    let mut ids = std::collections::BTreeSet::new();
    for _ in 0..31 {
        let result = c.receive();
        let id = result["id"].as_u64().unwrap();
        assert!((101..132).contains(&id));
        assert_ne!(result["result"]["isError"], true);
        assert!(ids.insert(id));
    }
    assert_eq!(ids, (101..132).collect());
    data(&c.call("spec_projects", json!({})));
    c.close();
}

#[cfg(unix)]
#[test]
fn management_refresh_composes_full_result_before_registration_and_records() {
    use std::os::unix::ffi::OsStrExt;
    fn setup() -> (
        vibe_core::testscratch::Scratch,
        Client,
        Value,
        std::path::PathBuf,
        std::path::PathBuf,
    ) {
        let owner = vibe_core::testscratch::scratch("vibe-spec", "composed-management");
        let code = owner.join("code");
        let store = owner.join("st\"ore");
        std::fs::create_dir(&code).unwrap();
        std::fs::create_dir(&store).unwrap();
        git(&code, &["init", "-b", "main"]);
        git(
            &code,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "fixture",
            ],
        );
        git(
            &code,
            &[
                "remote",
                "add",
                "origin",
                "https://example.invalid/owner/composed.git",
            ],
        );
        let mut c = Client::new(&code, &store, true, &[]);
        let initialized = c.call(
            "spec_project_init",
            json!({"mode":"global-first","expected_revision":"absent"}),
        );
        let project = data(&initialized)["project"]["id"].clone();
        for n in 0..12 {
            let name = format!("source-{n}");
            let show = c.call("spec_project_show", json!({"project":project}));
            data(&c.call("spec_branch_init",json!({"project":project,"name":name,"from":"main","expected_revision":data(&show)["revision"],"expected_project_revision":data(&show)["project_revision"]})));
            write(
                &mut c,
                &project,
                &name,
                "spec/lessons.md",
                &format!("# Lessons\n\nSource {n}.\n"),
            );
        }
        let clone = owner.join("clone\"code");
        git(
            &code,
            &[
                "clone",
                "--shared",
                code.to_str().unwrap(),
                clone.to_str().unwrap(),
            ],
        );
        git(
            &clone,
            &[
                "remote",
                "set-url",
                "origin",
                "https://example.invalid/owner/composed.git",
            ],
        );
        (owner, c, project, clone, store)
    }
    fn replace(value: &mut Value, old: &str, new: &str) {
        match value {
            Value::String(s) => *s = s.replace(old, new),
            Value::Array(a) => {
                for v in a {
                    replace(v, old, new)
                }
            }
            Value::Object(o) => {
                for v in o.values_mut() {
                    replace(v, old, new)
                }
            }
            _ => {}
        }
    }
    let (first_owner, mut first, first_project, first_clone, _) = setup();
    let show = first.call("spec_project_show", json!({"project":first_project}));
    let mut planned=first.call("spec_project_init",json!({"mode":"global-first","project":first_project,"checkout":{"unix_bytes":first_clone.as_os_str().as_bytes()},"expected_revision":data(&show)["revision"]}));
    assert_eq!(
        data(&planned)["context"]["refresh"]["needs_input"]
            .as_array()
            .unwrap()
            .len(),
        12
    );
    first.close();
    let (second_owner, mut second, project, clone, store) = setup();
    replace(
        &mut planned,
        first_owner.to_str().unwrap(),
        second_owner.to_str().unwrap(),
    );
    let prefix = "composed-\"\\";
    planned["id"] = json!(prefix);
    let length = serde_json::to_vec(&planned).unwrap().len() + 1;
    let id = prefix.to_owned() + &"i".repeat(8 * 1024 * 1024 - length + 1);
    planned["id"] = json!(id);
    assert_eq!(
        serde_json::to_vec(&planned).unwrap().len() + 1,
        8 * 1024 * 1024 + 1
    );
    let before = second.call("spec_project_show", json!({"project":project}));
    let project_root = store.join("spec/projects").join(project.as_str().unwrap());
    let metadata = std::fs::read(project_root.join("project.json")).unwrap();
    let records = project_root.join("reconciliations");
    assert!(!records.exists());
    let request = json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}},"name":"spec_project_init","arguments":{"mode":"global-first","project":project,"checkout":{"unix_bytes":clone.as_os_str().as_bytes()},"expected_revision":data(&before)["revision"]}}});
    assert!(serde_json::to_vec(&request).unwrap().len() < 8 * 1024 * 1024);
    second.send(request);
    let refused = second.receive();
    assert_eq!(refused["id"], id);
    assert_eq!(
        refused["result"]["structuredContent"]["error"]["code"],
        "size_limit"
    );
    let after = second.call("spec_project_show", json!({"project":project}));
    assert_eq!(
        data(&after)["revision"],
        data(&before)["revision"],
        "no new checkout registration on a planned composite-size refusal"
    );
    assert_eq!(
        std::fs::read(project_root.join("project.json")).unwrap(),
        metadata
    );
    assert!(
        !records.exists(),
        "no new NeedsInput records on a planned composite-size refusal"
    );
    second.close();
}

fn reconcile_known_final_receipt(mode: u8) {
    fn git_text(root: &std::path::Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    }
    fn setup(
        mode: u8,
    ) -> (
        vibe_core::testscratch::Scratch,
        Client,
        Value,
        Value,
        std::path::PathBuf,
    ) {
        let (owner, code, store) = fixture();
        git(&code, &["init", "-b", "main"]);
        git(
            &code,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "fixture",
            ],
        );
        let mut c = Client::new(&code, &store, true, &[]);
        let initialized = c.call(
            "spec_project_init",
            json!({"mode":"global-first","expected_revision":"absent"}),
        );
        let project = data(&initialized)["project"]["id"].clone();
        let show = c.call("spec_project_show", json!({"project":project}));
        data(&c.call("spec_branch_init",json!({"project":project,"name":"topic","from":"main","expected_revision":data(&show)["revision"],"expected_project_revision":data(&show)["project_revision"]})));
        let root = store.join("spec/projects").join(project.as_str().unwrap());
        if mode == 1 {
            let show = c.call("spec_project_show", json!({"project":project}));
            let source = data(&show)["branches"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["code_ref"] == "refs/heads/topic")
                .unwrap()["spec_ref"]
                .as_str()
                .unwrap();
            let history = root.join("history.git");
            let tree = git_text(&history, &["rev-parse", &format!("{source}^{{tree}}")]);
            let orphan = git_text(&history, &["commit-tree", &tree, "-m", "unrelated root"]);
            git(&history, &["update-ref", source, &orphan]);
        } else if mode == 0 {
            let preview = c.call(
                "spec_reconcile",
                json!({"project":project,"source":"topic","target":"main","dry_run":true}),
            );
            let plan = data(&preview).clone();
            assert_eq!(
                data(&c.call(
                    "spec_reconcile",
                    json!({"plan":plan,"expected_revision":plan["target_revision"]})
                ))["state"],
                "applied"
            );
        } else {
            write(
                &mut c,
                &project,
                "topic",
                "spec/lessons.md",
                "# Lessons\n\nSource.\n",
            );
            if mode == 2 {
                write(
                    &mut c,
                    &project,
                    "main",
                    "spec/lessons.md",
                    "# Lessons\n\nTarget.\n",
                );
            } else {
                write(&mut c, &project, "topic", "spec/product.md", "# Invalid\n");
            }
        }
        let preview = c.call(
            "spec_reconcile",
            json!({"project":project,"source":"topic","target":"main","dry_run":true}),
        );
        let plan = data(&preview).clone();
        assert_eq!(plan["base_spec_oid"].is_null(), mode == 1);
        (owner, c, project, plan, root)
    }
    fn replace(v: &mut Value, old: &str, new: &str, old_id: &str, new_id: &str) {
        match v {
            Value::String(s) => *s = s.replace(old, new).replace(old_id, new_id),
            Value::Array(a) => {
                for v in a {
                    replace(v, old, new, old_id, new_id)
                }
            }
            Value::Object(o) => {
                for v in o.values_mut() {
                    replace(v, old, new, old_id, new_id)
                }
            }
            _ => {}
        }
    }
    fn records(root: &std::path::Path) -> std::collections::BTreeSet<std::ffi::OsString> {
        std::fs::read_dir(root.join("reconciliations"))
            .map(|entries| entries.map(|e| e.unwrap().file_name()).collect())
            .unwrap_or_default()
    }
    {
        let (first_owner, mut first, first_project, first_plan, _) = setup(mode);
        let mut future = first.call(
            "spec_reconcile",
            json!({"plan":first_plan,"expected_revision":first_plan["target_revision"]}),
        );
        assert_eq!(
            data(&future)["state"],
            match mode {
                0 => "applied",
                2 => "conflict",
                _ => "needs-input",
            }
        );
        if mode == 2 {
            assert!(!data(&future)["stages"].as_object().unwrap().is_empty());
            assert!(!data(&future)["merge_index"].as_object().unwrap().is_empty());
        }
        if mode == 3 {
            assert!(data(&future)["diagnostics"]["errors"].as_u64().unwrap() > 0);
        }
        first.close();
        let (second_owner, mut second, project, plan, root) = setup(mode);
        replace(
            &mut future,
            first_owner.to_str().unwrap(),
            second_owner.to_str().unwrap(),
            first_project.as_str().unwrap(),
            project.as_str().unwrap(),
        );
        let prefix = "receipt-\"\\";
        future["id"] = json!(prefix);
        let length = serde_json::to_vec(&future).unwrap().len() + 1;
        let id = prefix.to_owned() + &"r".repeat(8 * 1024 * 1024 - length + 1);
        future["id"] = json!(id);
        assert_eq!(
            serde_json::to_vec(&future).unwrap().len() + 1,
            8 * 1024 * 1024 + 1
        );
        let before = records(&root);
        let metadata = std::fs::read(root.join("project.json")).unwrap();
        let request = json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}},"name":"spec_reconcile","arguments":{"plan":plan,"expected_revision":plan["target_revision"]}}});
        assert!(serde_json::to_vec(&request).unwrap().len() < 8 * 1024 * 1024);
        second.send(request);
        let refused = second.receive();
        assert_eq!(refused["id"], id);
        assert_eq!(
            refused["result"]["structuredContent"]["error"]["code"],
            "size_limit"
        );
        assert_eq!(
            records(&root),
            before,
            "known final receipt refusal must precede the first durable Pending record (mode={mode})"
        );
        assert_eq!(std::fs::read(root.join("project.json")).unwrap(), metadata);
        second.close();
    }
}

#[test]
fn reconcile_known_final_noop_receipt_is_admitted_before_pending() {
    reconcile_known_final_receipt(0);
}
#[test]
fn reconcile_known_final_no_base_receipt_is_admitted_before_pending() {
    reconcile_known_final_receipt(1);
}
#[test]
fn reconcile_known_final_conflict_receipt_is_admitted_before_pending() {
    reconcile_known_final_receipt(2);
}
#[test]
fn reconcile_known_final_diagnostics_receipt_is_admitted_before_pending() {
    reconcile_known_final_receipt(3);
}

#[test]
fn reconciliation_conflict_marker_growth_is_admitted_before_pending() {
    let (_owner, code, store) = fixture();
    git(&code, &["init", "-b", "main"]);
    git(
        &code,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    let mut c = Client::new(&code, &store, true, &[]);
    let initialized = c.call(
        "spec_project_init",
        json!({"mode":"global-first","expected_revision":"absent"}),
    );
    let project = data(&initialized)["project"]["id"].clone();
    let show = c.call("spec_project_show", json!({"project":project}));
    data(&c.call("spec_branch_init",json!({"project":project,"name":"topic","from":"main","expected_revision":data(&show)["revision"],"expected_project_revision":data(&show)["project_revision"]})));
    write(
        &mut c,
        &project,
        "topic",
        "spec/lessons.md",
        &format!("# Lessons\n\n{}\n", "s".repeat(4 * 1024 * 1024 + 64)),
    );
    write(
        &mut c,
        &project,
        "main",
        "spec/lessons.md",
        &format!("# Lessons\n\n{}\n", "t".repeat(4 * 1024 * 1024 + 64)),
    );
    let preview = c.call(
        "spec_reconcile",
        json!({"project":project,"source":"topic","target":"main","dry_run":true}),
    );
    let plan = data(&preview).clone();
    let root = store.join("spec/projects").join(project.as_str().unwrap());
    let metadata = std::fs::read(root.join("project.json")).unwrap();
    let records = || {
        std::fs::read_dir(root.join("reconciliations"))
            .map(|entries| {
                entries
                    .map(|e| e.unwrap().file_name())
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default()
    };
    let before = records();
    let refused = c.call(
        "spec_reconcile",
        json!({"plan":plan,"expected_revision":plan["target_revision"]}),
    );
    assert_eq!(
        refused["result"]["structuredContent"]["error"]["code"],
        "size_limit"
    );
    assert_eq!(records(), before);
    assert_eq!(std::fs::read(root.join("project.json")).unwrap(), metadata);
    assert!(!std::fs::read_dir(root).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".response-admission-")
    }));
    c.close();
}

#[test]
fn immutable_validation_config_refuses_management_before_registration() {
    fn prepare() -> (
        vibe_core::testscratch::Scratch,
        std::path::PathBuf,
        std::path::PathBuf,
        Value,
        std::path::PathBuf,
    ) {
        let (owner, code, store) = fixture();
        git(&code, &["init", "-b", "main"]);
        git(
            &code,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "fixture",
            ],
        );
        git(&code, &["branch", "topic"]);
        let mut c = Client::new(&code, &store, true, &[]);
        let initialized = c.call(
            "spec_project_init",
            json!({"mode":"global-first","expected_revision":"absent"}),
        );
        let project = data(&initialized)["project"]["id"].clone();
        let show = c.call("spec_project_show", json!({"project":project}));
        data(&c.call("spec_branch_init",json!({"project":project,"name":"topic","from":"main","expected_revision":data(&show)["revision"],"expected_project_revision":data(&show)["project_revision"]})));
        data(&c.call("spec_refresh", json!({"project":project,"branch":"main"})));
        write(
            &mut c,
            &project,
            "main",
            "spec/lessons.md",
            "# Lessons\n\nTarget fact.\n",
        );
        let context = c.call("spec_context", json!({"project":project,"branch":"main"}));
        data(&c.call("spec_snapshot",json!({"project":project,"branch":"main","expected_revision":data(&context)["revision_token"]})));
        git(&code, &["checkout", "topic"]);
        git(
            &code,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "topic",
            ],
        );
        write(
            &mut c,
            &project,
            "topic",
            "spec/extra.md",
            "# Extra\n\nSource fact.\n",
        );
        let context = c.call("spec_context", json!({"project":project,"branch":"topic"}));
        data(&c.call("spec_snapshot",json!({"project":project,"branch":"topic","expected_revision":data(&context)["revision_token"]})));
        git(&code, &["checkout", "main"]);
        git(
            &code,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "merge",
                "--no-ff",
                "topic",
                "-m",
                "merge",
            ],
        );
        let extra = owner.join("new-management-checkout");
        git(
            &code,
            &[
                "worktree",
                "add",
                "--force",
                extra.to_str().unwrap(),
                "main",
            ],
        );
        c.close();
        (owner, code, store, project, extra)
    }
    fn substitute(v: &mut Value, old: &str, new: &str, old_id: &str, new_id: &str) {
        match v {
            Value::String(s) => *s = s.replace(old, new).replace(old_id, new_id),
            Value::Array(a) => {
                for v in a {
                    substitute(v, old, new, old_id, new_id)
                }
            }
            Value::Object(o) => {
                for v in o.values_mut() {
                    substitute(v, old, new, old_id, new_id)
                }
            }
            _ => {}
        }
    }
    let key = format!("immutable-\"\\{}", "strict".repeat(2000));
    let strictness = json!({key.clone():42}).to_string();
    let message =
        format!("VIBE_SPEC_STRICTNESS: strictness.{key} must be \"warn\", \"error\" or \"off\"");
    // Start with a real complete automatic management result. Only its refresh
    // classification changes when this immutable validator config is invalid.
    let (first_owner, first_code, first_store, first_project, first_extra) = prepare();
    let mut first = Client::new(&first_code, &first_store, true, &[]);
    let show = first.call("spec_project_show", json!({"project":first_project}));
    let source = data(&show)["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["code_ref"] == "refs/heads/topic")
        .unwrap()["id"]
        .clone();
    let mut future=first.call("spec_project_init",json!({"project":first_project,"checkout":first_extra,"mode":"global-first","expected_revision":data(&show)["revision"]}));
    assert_eq!(
        data(&future)["context"]["refresh"]["applied"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let summary = json!({"applied":[],"conflict":[],"needs_input":[],"diagnostics":[{"code":"invalid_config","message":message,"source":source}]});
    future["result"]["structuredContent"]["context"]["refresh"] = summary.clone();
    future["result"]["structuredContent"]["data"]["context"]["refresh"] = summary;
    future["result"]["content"][0]["text"] =
        json!(future["result"]["structuredContent"].to_string());
    first.close();
    let (owner, code, store, project, extra) = prepare();
    substitute(
        &mut future,
        first_owner.to_str().unwrap(),
        owner.to_str().unwrap(),
        first_project.as_str().unwrap(),
        project.as_str().unwrap(),
    );
    let prefix = "config-\"\\";
    future["id"] = json!(prefix);
    let length = serde_json::to_vec(&future).unwrap().len() + 1;
    let id = prefix.to_owned() + &"k".repeat(8 * 1024 * 1024 - length + 1);
    future["id"] = json!(id);
    assert_eq!(
        serde_json::to_vec(&future).unwrap().len() + 1,
        8 * 1024 * 1024 + 1
    );
    let root = store.join("spec/projects").join(project.as_str().unwrap());
    let metadata = std::fs::read(root.join("project.json")).unwrap();
    let registry = std::fs::read(store.join("spec/registry.json")).unwrap();
    let records = || {
        std::fs::read_dir(root.join("reconciliations"))
            .map(|entries| {
                entries
                    .map(|e| e.unwrap().file_name())
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default()
    };
    let before = records();
    let mut c = Client::new_env(
        &code,
        &store,
        true,
        &[],
        &[("VIBE_SPEC_STRICTNESS", std::ffi::OsStr::new(&strictness))],
    );
    let show = c.call("spec_project_show", json!({"project":project}));
    let arguments = json!({"project":project,"checkout":extra,"mode":"global-first","expected_revision":data(&show)["revision"]});
    let request = json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}},"name":"spec_project_init","arguments":arguments}});
    assert!(serde_json::to_vec(&request).unwrap().len() < 8 * 1024 * 1024);
    c.send(request);
    let refused = c.receive();
    assert_eq!(refused["id"], id);
    assert_eq!(
        std::fs::read(store.join("spec/registry.json")).unwrap(),
        registry,
        "immutable config refusal must precede changed checkout registration"
    );
    assert_eq!(std::fs::read(root.join("project.json")).unwrap(), metadata);
    assert_eq!(records(), before);
    assert_eq!(
        refused["result"]["structuredContent"]["error"]["code"],
        "invalid_config"
    );
    assert_eq!(
        refused["result"]["structuredContent"]["error"]["message"],
        message
    );
    let small = c.call("spec_project_init", arguments.clone());
    assert_eq!(
        small["result"]["structuredContent"]["error"]["code"],
        "invalid_config"
    );
    assert_eq!(
        small["result"]["structuredContent"]["error"]["message"],
        message
    );
    assert_eq!(
        std::fs::read(store.join("spec/registry.json")).unwrap(),
        registry
    );
    assert_eq!(std::fs::read(root.join("project.json")).unwrap(), metadata);
    assert_eq!(records(), before);
    c.close();
    let mut normal = Client::new(&code, &store, true, &[]);
    let result = normal.call("spec_project_init", arguments);
    assert_eq!(
        data(&result)["context"]["refresh"]["applied"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "same exact eligible work remains available after refusal"
    );
    normal.close();
}
