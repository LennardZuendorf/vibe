#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use vibe_flow::{cli, paths::Env};
struct Empty;
impl Env for Empty {
    fn get(&self, _: &str) -> Option<String> {
        None
    }
}
static NEXT: AtomicU64 = AtomicU64::new(0);
fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "flow-runtime-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(p.join(".agents/flow")).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flow/state-machine.json"),
        p.join(".agents/flow/state-machine.json"),
    )
    .unwrap();
    p
}
fn run(p: &Path, args: &[&str]) -> (u8, String, String) {
    let a: Vec<OsString> = ["vibe-flow"]
        .into_iter()
        .chain(args.iter().copied())
        .map(Into::into)
        .collect();
    let (c, o, e) = cli::run(p, &a, &Empty);
    (
        c,
        String::from_utf8(o).unwrap(),
        String::from_utf8(e).unwrap(),
    )
}
fn upgrade(p: &Path) {
    let r = run(p, &["state", "upgrade", "run-1", "plan-a", "scope-a"]);
    assert_eq!(r.0, 0, "{r:?}");
}
fn request(revision: u64, target: &str) -> String {
    json!({"v":1,"run_id":"run-1","expected_revision":revision,"target":target,"feature":"demo","approval_ref":null,"candidate_digest":null,"action_ref":null}).to_string()
}
#[test] // Task 5: all v1 writers must refuse upgraded metadata.
fn upgrade_preserves_metadata_and_legacy_writer_refuses_downgrade() {
    let p = root();
    upgrade(&p);
    let before = fs::read(p.join(".vibe/run/flow/state.json")).unwrap();
    let state: Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(state["v"], 2);
    assert_eq!(state["revision"], 0);
    assert_ne!(run(&p, &["state", "set", "quick.fix"]).0, 0);
    assert_eq!(
        fs::read(p.join(".vibe/run/flow/state.json")).unwrap(),
        before
    );
}
#[test] // Task 5: known targets alone cannot skip progression or stale CAS.
fn transitions_check_edges_run_and_revision_without_mutation() {
    let p = root();
    upgrade(&p);
    assert_eq!(
        run(&p, &["state", "transition", &request(0, "feature.design")]).0,
        0
    );
    for body in [
        request(0, "feature.plan"),
        request(1, "feature.impl"),
        request(1, "quick.fix"),
        request(1, "feature.plan").replace("run-1", "other"),
    ] {
        let before = fs::read(p.join(".vibe/run/flow/state.json")).unwrap();
        assert_ne!(run(&p, &["state", "transition", &body]).0, 0);
        assert_eq!(
            fs::read(p.join(".vibe/run/flow/state.json")).unwrap(),
            before
        );
    }
    assert_eq!(
        run(&p, &["state", "transition", &request(1, "feature.plan")]).0,
        0
    );
    let forged =
        request(2, "feature.impl").replace("\"approval_ref\":null", "\"approval_ref\":\"confirm\"");
    let denied = run(&p, &["state", "transition", &forged]);
    assert_ne!(denied.0, 0);
    assert!(denied.2.contains("unavailable"), "{denied:?}");
}
#[test] // Task 5: corrupt upgraded state cannot fall back or auto-recover.
fn corrupt_current_requires_explicit_recovery() {
    let p = root();
    upgrade(&p);
    fs::write(p.join(".vibe/run/flow/state.json"), "{broken").unwrap();
    assert_ne!(
        run(&p, &["state", "transition", &request(0, "feature.design")]).0,
        0
    );
    assert_eq!(
        fs::read(p.join(".vibe/run/flow/state.json")).unwrap(),
        b"{broken"
    );
    assert_ne!(
        run(&p, &["state", "upgrade", "run-2", "plan", "scope"]).0,
        0
    );
}
#[test] // Task 6: v2 Stop never treats a re-entry continuation as passed evidence.
fn upgraded_stop_fails_closed_even_on_reentry() {
    let p = root();
    upgrade(&p);
    let mut state: Value =
        serde_json::from_slice(&fs::read(p.join(".vibe/run/flow/state.json")).unwrap()).unwrap();
    state["flow"] = json!("feature");
    state["phase"] = json!("verify");
    state["feature"] = json!("demo");
    fs::write(p.join(".vibe/run/flow/state.json"), state.to_string()).unwrap();
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let out = vibe_flow::hook::stop(&paths, b"{\"stop_hook_active\":true}");
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("unavailable"));
}
#[test] // Task 5: present busy lock prevents even adoption reads/writes.
fn concurrent_writer_is_refused_and_leaves_cursor_unchanged() {
    let p = root();
    fs::create_dir_all(p.join(".vibe/run/flow")).unwrap();
    fs::write(p.join(".vibe/run/flow/runtime.lock"), "foreign").unwrap();
    assert_ne!(
        run(&p, &["state", "upgrade", "run-1", "plan", "scope"]).0,
        0
    );
    assert!(!p.join(".vibe/run/flow/state.json").exists());
}
#[test] // Task 5: recovery requires the exact damaged bytes and retains them.
fn explicit_recovery_rejects_wrong_digest_and_preserves_original() {
    let p = root();
    fs::create_dir_all(p.join(".vibe/run/flow")).unwrap();
    fs::write(p.join(".vibe/run/flow/state.json"), "broken").unwrap();
    assert_ne!(
        run(
            &p,
            &["state", "recover", "wrong", "recovered", "plan", "scope"]
        )
        .0,
        0
    );
    let digest = vibe_flow::candidate::digest(&p, b"broken").unwrap();
    let result = run(
        &p,
        &["state", "recover", &digest, "recovered", "plan", "scope"],
    );
    assert_eq!(result.0, 0, "{result:?}");
    assert_eq!(
        fs::read(p.join(".vibe/run/flow/runs/recovered/recovered-cursor")).unwrap(),
        b"broken"
    );
    let v: Value =
        serde_json::from_slice(&fs::read(p.join(".vibe/run/flow/state.json")).unwrap()).unwrap();
    assert_eq!(v["phase"], "idle");
}
#[test] // Task 5: immutable worker claims bind current plan, revision and task dependencies.
fn task_records_are_published_before_cursor_references_and_reject_stale_plan() {
    let p = root();
    assert!(
        std::process::Command::new("git")
            .current_dir(&p)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let brief = json!({"task_id":"task-5","run_id":"run-1","plan_digest":"plan-a","requirements":["R3"],"read_paths":[],"write_scope":["source"],"dependencies":[],"checks":["tests"],"role":"implementer"});
    let result = json!({"task_id":"task-5","status":"done","changed_paths":[],"check_results":checks(),"findings":[],"artifacts":[],"candidate_digest":vibe_flow::candidate::snapshot(&p).unwrap()});
    assert!(vibe_flow::runtime::record_task(&paths, 0, &brief, &result).is_ok());
    let state = vibe_flow::runtime::read(&paths).unwrap();
    let reference = state["task_state"]["task-5"]["result_ref"]
        .as_str()
        .unwrap();
    assert!(p.join(".vibe/run/flow/runs/run-1").join(reference).exists());
    let before = fs::read(p.join(".vibe/run/flow/state.json")).unwrap();
    let mut stale = brief;
    stale["plan_digest"] = json!("old");
    assert!(vibe_flow::runtime::record_task(&paths, 1, &stale, &result).is_err());
    assert_eq!(
        fs::read(p.join(".vibe/run/flow/state.json")).unwrap(),
        before
    );
}
#[test] // Task 7 remains unavailable: hooks must not degrade upgraded mutations to warnings.
fn upgraded_guard_allows_reads_and_plan_edits_but_denies_product_and_process_writes() {
    let p = root();
    upgrade(&p);
    assert_eq!(
        run(&p, &["state", "transition", &request(0, "feature.design")]).0,
        0
    );
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    for tool in [
        json!({"tool_name":"Bash","tool_input":{"command":"touch source"}}),
        json!({"tool_name":"Write","tool_input":{"file_path":"source"}}),
        json!({"tool_name":"Agent","tool_input":{}}),
    ] {
        assert_eq!(
            vibe_flow::hook::guard(&paths, tool.to_string().as_bytes()).code,
            2
        );
    }
    assert_eq!(
        vibe_flow::hook::guard(
            &paths,
            b"{\"tool_name\":\"Read\",\"tool_input\":{\"file_path\":\"source\"}}"
        )
        .code,
        0
    );
    assert_eq!(vibe_flow::hook::guard(&paths,b"{\"tool_name\":\"Write\",\"tool_input\":{\"file_path\":\"docs/superpowers/specs/2026-11-11-demo-design.md\"}}").code,0);
}
#[test] // Task 5: every implementation re-entry needs authority; abort remains possible.
fn verify_reentry_denies_without_authority_but_explicit_abort_is_safe() {
    let p = root();
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let mut cursor = vibe_flow::runtime::read(&paths).unwrap();
    cursor["flow"] = json!("feature");
    cursor["phase"] = json!("verify");
    cursor["feature"] = json!("demo");
    fs::write(paths.cursor_path(), cursor.to_string()).unwrap();
    assert_ne!(
        run(&p, &["state", "transition", &request(0, "feature.impl")]).0,
        0
    );
    let mut abort: Value = serde_json::from_str(&request(0, "idle")).unwrap();
    abort["action_ref"] = json!("abort");
    assert_eq!(run(&p, &["state", "transition", &abort.to_string()]).0, 0);
}
#[test] // Task 5: damaged v2 cannot be downgraded by invoking a compatibility writer.
fn corrupted_adopted_runtime_still_refuses_legacy_writer() {
    let p = root();
    upgrade(&p);
    fs::write(p.join(".vibe/run/flow/state.json"), "broken").unwrap();
    assert_ne!(run(&p, &["state", "set", "feature.impl", "demo"]).0, 0);
    assert_eq!(
        fs::read(p.join(".vibe/run/flow/state.json")).unwrap(),
        b"broken"
    );
}
#[test] // Task 5: a crash after immutable publication can resume the exact state decision.
fn interrupted_publication_reuses_matching_record_without_overwriting_it() {
    let p = root();
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let machine = vibe_flow::machine::Machine::load(paths.data_dir()).unwrap();
    let current = vibe_flow::runtime::read(&paths).unwrap();
    let request: Value = serde_json::from_str(&request(0, "feature.design")).unwrap();
    let mut next = vibe_flow::transition::evaluate(&machine, &current, &request).unwrap();
    next["updated"] = json!("2020-01-01T00:00:00Z");
    let record = p.join(".vibe/run/flow/runs/run-1/revision-1.json");
    let bytes = serde_json::to_vec_pretty(&next).unwrap();
    fs::write(&record, &bytes).unwrap();
    assert!(vibe_flow::runtime::transition(&paths, &request).is_ok());
    assert_eq!(fs::read(&record).unwrap(), bytes);
    assert_eq!(vibe_flow::runtime::read(&paths).unwrap(), next);
}
#[test] // Task 5: planning access never covers neighboring feature files or escaping links.
fn planning_path_scope_rejects_neighbor_and_symlink_escape() {
    let p = root();
    upgrade(&p);
    assert_eq!(
        run(&p, &["state", "transition", &request(0, "feature.design")]).0,
        0
    );
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    for path in [
        "docs/superpowers/specs/2026-other-design.md",
        "docs/superpowers/specs/../specs/2026-demo-design.md",
        ".vibe/run/flow/state.json",
    ] {
        let payload = json!({"tool_name":"Write","tool_input":{"file_path":path}});
        assert_eq!(
            vibe_flow::hook::guard(&paths, payload.to_string().as_bytes()).code,
            2
        );
    }
    #[cfg(unix)]
    {
        let outside = root();
        std::os::unix::fs::symlink(&outside, p.join("docs")).unwrap();
        let payload = json!({"tool_name":"Write","tool_input":{"file_path":"docs/superpowers/specs/2026-demo-design.md"}});
        assert_eq!(
            vibe_flow::hook::guard(&paths, payload.to_string().as_bytes()).code,
            2
        );
    }
}
#[test] // Task 5/6: preparation is reversible and precedes final verification and ship.
fn lifecycle_requires_preparation_and_keeps_ship_gated() {
    let p = root();
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let machine = vibe_flow::machine::Machine::load(paths.data_dir()).unwrap();
    let mut cursor = vibe_flow::runtime::read(&paths).unwrap();
    cursor["flow"] = json!("feature");
    cursor["phase"] = json!("impl");
    cursor["feature"] = json!("demo");
    let req = |target: &str| serde_json::from_str::<Value>(&request(0, target)).unwrap();
    assert!(vibe_flow::transition::evaluate(&machine, &cursor, &req("feature.verify")).is_err());
    assert!(vibe_flow::transition::evaluate(&machine, &cursor, &req("feature.compound")).is_ok());
    cursor["phase"] = json!("compound");
    assert!(vibe_flow::transition::evaluate(&machine, &cursor, &req("feature.verify")).is_ok());
    cursor["phase"] = json!("verify");
    assert!(vibe_flow::transition::evaluate(&machine, &cursor, &req("feature.ship")).is_err());
    assert!(vibe_flow::transition::evaluate(&machine, &cursor, &req("feature.plan")).is_ok());
}
#[test] // Task 5: persisted adoption prevents hook downgrade after cursor deletion/replacement.
fn adopted_hook_never_falls_back_for_missing_or_versionless_current() {
    let p = root();
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    fs::remove_file(paths.cursor_path()).unwrap();
    assert_eq!(
        vibe_flow::hook::guard(
            &paths,
            b"{\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"touch product\"}}"
        )
        .code,
        2
    );
    fs::write(
        paths.cursor_path(),
        "{\"flow\":\"idle\",\"phase\":\"idle\"}",
    )
    .unwrap();
    assert_eq!(vibe_flow::hook::stop(&paths, b"{}").code, 2);
}
#[test] // Task 5: worker result publication retries exact records, never overwrites conflicts.
fn interrupted_task_result_publication_reuses_identical_record() {
    let p = root();
    assert!(
        std::process::Command::new("git")
            .current_dir(&p)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let brief = json!({"task_id":"task-5","run_id":"run-1","plan_digest":"plan-a","requirements":["R3"],"read_paths":[],"write_scope":["source"],"dependencies":[],"checks":["tests"],"role":"implementer"});
    let result = json!({"task_id":"task-5","status":"done","changed_paths":[],"check_results":checks(),"findings":[],"artifacts":[],"candidate_digest":vibe_flow::candidate::snapshot(&p).unwrap()});
    let path = p.join(".vibe/run/flow/runs/run-1/task-task-5-revision-1.json");
    let bytes = serde_json::to_vec_pretty(&json!({"source":"agent","brief":brief,"result":result}))
        .unwrap();
    fs::write(&path, &bytes).unwrap();
    assert!(vibe_flow::runtime::record_task(&paths, 0, &brief, &result).is_ok());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}
fn checks() -> Value {
    json!([{"id":"tests","argv":["cargo","test"],"cwd":"/repo","exit_code":0,"output_digest":"output-a","output_path":"output.txt","started_at":"2026-11-11T10:00:00Z","finished_at":"2026-11-11T10:01:00Z"}])
}
#[test]
fn done_task_rejects_missing_failed_or_wrong_required_check() {
    let p = root();
    assert!(
        std::process::Command::new("git")
            .current_dir(&p)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let brief = json!({"task_id":"task-5","run_id":"run-1","plan_digest":"plan-a","requirements":["R3"],"read_paths":[],"write_scope":["source"],"dependencies":[],"checks":["tests"],"role":"implementer"});
    let good = json!({"task_id":"task-5","status":"done","changed_paths":[],"check_results":checks(),"findings":[],"artifacts":[],"candidate_digest":vibe_flow::candidate::snapshot(&p).unwrap()});
    for badcheck in [
        json!([]),
        {
            let mut c = checks();
            c[0]["exit_code"] = json!(1);
            c
        },
        {
            let mut c = checks();
            c[0]["id"] = json!("unrelated");
            c
        },
    ] {
        let mut bad = good.clone();
        bad["check_results"] = badcheck;
        assert!(vibe_flow::runtime::record_task(&paths, 0, &brief, &bad).is_err());
    }
    assert_eq!(vibe_flow::runtime::read(&paths).unwrap()["revision"], 0);
}
#[cfg(unix)]
#[test]
fn planning_alias_cannot_overwrite_runtime_or_product() {
    for target in [".vibe/run/flow/state.json", "product"] {
        let p = root();
        upgrade(&p);
        assert_eq!(
            run(&p, &["state", "transition", &request(0, "feature.design")]).0,
            0
        );
        let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
        fs::write(p.join("product"), "keep").unwrap();
        fs::create_dir_all(p.join("docs/superpowers/specs")).unwrap();
        std::os::unix::fs::symlink(
            p.join(target),
            p.join("docs/superpowers/specs/2026-demo-design.md"),
        )
        .unwrap();
        assert_eq!(vibe_flow::hook::guard(&paths,b"{\"tool_name\":\"Write\",\"tool_input\":{\"file_path\":\"docs/superpowers/specs/2026-demo-design.md\"}}").code,2);
    }
}
#[test]
fn semantic_corruption_denies_hooks_get_and_transition_and_can_recover() {
    let p = root();
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let base = vibe_flow::runtime::read(&paths).unwrap();
    for mutation in [
        json!({"flow":"unknown"}),
        json!({"phase":"unknown"}),
        json!({"feature":42}),
        json!({"flow":"feature","phase":"plan","feature":null}),
    ] {
        let mut bad = base.clone();
        for (k, v) in mutation.as_object().unwrap() {
            bad[k] = v.clone();
        }
        fs::write(paths.cursor_path(), bad.to_string()).unwrap();
        assert_ne!(run(&p, &["state", "get"]).0, 0);
        assert_eq!(vibe_flow::hook::stop(&paths, b"{}").code, 2);
        assert_ne!(
            run(&p, &["state", "transition", &request(0, "feature.design")]).0,
            0
        );
    }
    let bytes = fs::read(paths.cursor_path()).unwrap();
    let digest = vibe_flow::candidate::digest(&p, &bytes).unwrap();
    assert_eq!(
        run(&p, &["state", "recover", &digest, "fixed", "plan", "scope"]).0,
        0
    );
}
#[test]
fn dependent_task_rechecks_persisted_required_checks_and_missing_records() {
    let p = root();
    assert!(
        std::process::Command::new("git")
            .current_dir(&p)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let mut brief = json!({"task_id":"task-5","run_id":"run-1","plan_digest":"plan-a","requirements":["R3"],"read_paths":[],"write_scope":["source"],"dependencies":[],"checks":["tests"],"role":"implementer"});
    let mut result = json!({"task_id":"task-5","status":"done","changed_paths":[],"check_results":checks(),"findings":[],"artifacts":[],"candidate_digest":vibe_flow::candidate::snapshot(&p).unwrap()});
    vibe_flow::runtime::record_task(&paths, 0, &brief, &result).unwrap();
    let state = vibe_flow::runtime::read(&paths).unwrap();
    let path = p.join(".vibe/run/flow/runs/run-1").join(
        state["task_state"]["task-5"]["result_ref"]
            .as_str()
            .unwrap(),
    );
    let mut stored: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    stored["result"]["check_results"] = json!([]);
    fs::write(&path, stored.to_string()).unwrap();
    brief["task_id"] = json!("task-6");
    brief["dependencies"] = json!(["task-5"]);
    result["task_id"] = json!("task-6");
    assert!(vibe_flow::runtime::record_task(&paths, 1, &brief, &result).is_err());
    fs::remove_file(path).unwrap();
    assert!(vibe_flow::runtime::record_task(&paths, 1, &brief, &result).is_err());
}
#[test] // Task 5: a dependency binds its accepted result, not later independent task bytes.
fn dependent_task_can_publish_its_own_changes_without_rewriting_accepted_dependency() {
    let p = root();
    assert!(
        std::process::Command::new("git")
            .current_dir(&p)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    upgrade(&p);
    let paths = vibe_flow::paths::FlowPaths::resolve(Some(&p), None, &p, &Empty);
    let mut brief = json!({"task_id":"task-1","run_id":"run-1","plan_digest":"plan-a","requirements":["R3"],"read_paths":[],"write_scope":["first"],"dependencies":[],"checks":["tests"],"role":"implementer"});
    fs::write(p.join("first"), "task one").unwrap();
    let mut result = json!({"task_id":"task-1","status":"done","changed_paths":["first"],"check_results":checks(),"findings":[],"artifacts":[],"candidate_digest":vibe_flow::candidate::snapshot(&p).unwrap()});
    vibe_flow::runtime::record_task(&paths, 0, &brief, &result).unwrap();
    let accepted = vibe_flow::runtime::read(&paths).unwrap();
    let dependency = p.join(".vibe/run/flow/runs/run-1").join(
        accepted["task_state"]["task-1"]["result_ref"]
            .as_str()
            .unwrap(),
    );
    let original = fs::read(&dependency).unwrap();
    fs::write(p.join("second"), "task two").unwrap();
    brief["task_id"] = json!("task-2");
    brief["write_scope"] = json!(["second"]);
    brief["dependencies"] = json!(["task-1"]);
    result["task_id"] = json!("task-2");
    result["changed_paths"] = json!(["second"]);
    result["candidate_digest"] = json!(vibe_flow::candidate::snapshot(&p).unwrap());
    assert_ne!(
        accepted["task_state"]["task-1"]["candidate_digest"],
        result["candidate_digest"]
    );
    let mut tampered: Value = serde_json::from_slice(&original).unwrap();
    tampered["result"]["candidate_digest"] = result["candidate_digest"].clone();
    fs::write(&dependency, tampered.to_string()).unwrap();
    assert!(vibe_flow::runtime::record_task(&paths, 1, &brief, &result).is_err());
    fs::write(&dependency, &original).unwrap();
    assert!(vibe_flow::runtime::record_task(&paths, 1, &brief, &result).is_ok());
    let after = vibe_flow::runtime::read(&paths).unwrap();
    assert_eq!(
        after["task_state"]["task-1"],
        accepted["task_state"]["task-1"]
    );
    assert_eq!(fs::read(dependency).unwrap(), original);
    assert_eq!(
        after["task_state"]["task-2"]["candidate_digest"],
        result["candidate_digest"]
    );
}
