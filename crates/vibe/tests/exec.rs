#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Plan items, the executor, state, links and the shared adapter helpers.
mod common;
use common::Home;
use std::fs;
use std::path::{Path, PathBuf};
use vibe::adapters::{Ctx, Mode, enabled_harnesses, project_all, skill_dirs};
use vibe::exec::{apply, with_lock};
use vibe::paths::{Harness, Paths};
use vibe::plan::{
    CliOp, Note, OwnedEdit, PlanItem, Projection, Stamp, block_item, guard_target, json_file_item,
};
use vibe::state::{self, State};
use vibe::store;
use vibe_core::dirs::Env;
use vibe_core::json::{self, Value};
use vibe_core::markers::{Style, hash_bytes};

fn paths(home: &Home) -> Paths {
    let vars = [("HOME".to_string(), home.dir.to_string_lossy().into_owned())];
    Paths::from_env(&Env::new(vars, false)).unwrap()
}

fn fw(path: &Path, bytes: &str) -> PlanItem {
    PlanItem::FileWrite {
        harness: Harness::Codex,
        show: path.display().to_string(),
        path: path.to_path_buf(),
        bytes: bytes.as_bytes().to_vec(),
    }
}

fn proj(items: Vec<PlanItem>, edits: Vec<(usize, OwnedEdit)>) -> Projection {
    Projection {
        items,
        edits,
        ..Projection::default()
    }
}

fn run(home: &Home, p: Projection, s: State) -> (vibe::exec::Outcome, State) {
    apply(&paths(home), p, s, false, "/nonexistent/vibe-test/claude")
}

fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            let m = fs::symlink_metadata(&p).unwrap();
            if m.is_dir() {
                out.push((p.clone(), b"<dir>".to_vec()));
                stack.push(p);
            } else if m.file_type().is_symlink() {
                out.push((
                    p.clone(),
                    fs::read_link(&p)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else {
                let bytes = fs::read(&p).unwrap();
                out.push((p, bytes));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn files_are_all_or_nothing() {
    let home = Home::new("exec-files-atomic");
    home.write("a.txt", "A0");
    home.write("b.txt", "B0");
    home.write("blocker", "i am a file");
    let items = vec![
        fw(&home.path("a.txt"), "A1"),
        fw(&home.path("b.txt"), "B1"),
        fw(&home.path("blocker/c.txt"), "C1"),
    ];
    let before = snapshot(&home.dir);
    let (out, st) = run(&home, proj(items, vec![]), State::default());
    assert_eq!(out.failed.len(), 1, "{:?}", out.failed);
    assert_eq!(snapshot(&home.dir), before);
    assert_eq!(st, State::default());
}

#[test]
fn file_write_creates_parents_and_rollback_removes_them() {
    let home = Home::new("exec-parents");
    home.write("blocker", "file");
    let items = vec![
        fw(&home.path("new/deep/x.txt"), "X"),
        fw(&home.path("blocker/y.txt"), "Y"),
    ];
    let (out, _) = run(&home, proj(items, vec![]), State::default());
    assert_eq!(out.failed.len(), 1, "{:?}", out.failed);
    assert!(!home.path("new").exists());
    let (out, _) = run(
        &home,
        proj(vec![fw(&home.path("new/deep/x.txt"), "X")], vec![]),
        State::default(),
    );
    assert!(out.failed.is_empty());
    assert_eq!(home.read("new/deep/x.txt"), "X");
}

#[cfg(unix)]
#[test]
fn claude_child_env_is_pinned_to_paths() {
    let home = Home::new("claude-env");
    let log = home.path("env.log");
    let bin = home.write_exec(
        "bin/claude",
        &format!(
            "#!/bin/sh\nprintf '%s|%s\\n' \"$HOME\" \"$CLAUDE_CONFIG_DIR\" >> '{}'\nexit 0\n",
            log.display()
        ),
    );
    let add = |name: &str| PlanItem::ClaudeCli {
        op: vibe::plan::CliOp::Add {
            name: name.into(),
            json: "{}".into(),
        },
    };
    let bin = bin.to_string_lossy().into_owned();
    let (out, _) = apply(
        &paths(&home),
        proj(vec![add("x")], vec![]),
        State::default(),
        false,
        &bin,
    );
    assert!(out.failed.is_empty(), "{:?}", out.failed);
    let mut p = paths(&home);
    p.claude_dir = home.path("custom-claude");
    p.claude_config_env = Some(home.path("custom-claude"));
    let (out, _) = apply(
        &p,
        proj(vec![add("y")], vec![]),
        State::default(),
        false,
        &bin,
    );
    assert!(out.failed.is_empty(), "{:?}", out.failed);
    let mut p = paths(&home);
    p.claude_config_env = Some(home.path(".claude"));
    let (out, _) = apply(
        &p,
        proj(vec![add("z")], vec![]),
        State::default(),
        false,
        &bin,
    );
    assert!(out.failed.is_empty(), "{:?}", out.failed);
    let lines: Vec<String> = std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    let h = home.dir.display().to_string();
    assert_eq!(lines[0], format!("{h}|"));
    assert_eq!(
        lines[1],
        format!("{h}|{}", home.path("custom-claude").display())
    );
    assert_eq!(lines[2], format!("{h}|{}", home.path(".claude").display()));
    assert_ne!(std::env::var("HOME").ok().as_deref(), Some(h.as_str()));
}

#[cfg(unix)]
#[test]
fn link_failure_undoes_links_then_files() {
    let home = Home::new("exec-link-fail");
    home.write("a.txt", "A0");
    home.write("blocker", "file");
    let items = vec![
        fw(&home.path("a.txt"), "A1"),
        PlanItem::LinkCreate {
            path: home.path("l1"),
            target: home.path("a.txt"),
        },
        PlanItem::LinkCreate {
            path: home.path("blocker/l2"),
            target: home.path("a.txt"),
        },
    ];
    let before = snapshot(&home.dir);
    let edits = vec![(
        0,
        OwnedEdit::Target {
            key: "codex:mcp".into(),
            hash: Some("h".into()),
        },
    )];
    let (out, st) = run(&home, proj(items, edits), State::default());
    assert!(!out.failed.is_empty());
    assert_eq!(snapshot(&home.dir), before);
    assert_eq!(st, State::default());
    assert!(!paths(&home).state_file().exists());
}

#[cfg(unix)]
#[test]
fn file_remove_restored_on_link_failure() {
    let home = Home::new("exec-remove-restore");
    home.write("gone.txt", "precious");
    home.write("blocker", "file");
    let items = vec![
        PlanItem::FileRemove {
            harness: Harness::Codex,
            show: "gone.txt".into(),
            path: home.path("gone.txt"),
        },
        PlanItem::LinkCreate {
            path: home.path("blocker/l"),
            target: home.path("blocker"),
        },
    ];
    let (out, _) = run(&home, proj(items, vec![]), State::default());
    assert!(!out.failed.is_empty());
    assert_eq!(home.read("gone.txt"), "precious");
}

#[cfg(unix)]
#[test]
fn file_remove_removes_and_reports() {
    let home = Home::new("exec-remove");
    home.write("gone.txt", "x");
    let items = vec![PlanItem::FileRemove {
        harness: Harness::Codex,
        show: "gone.txt".into(),
        path: home.path("gone.txt"),
    }];
    let (out, _) = run(&home, proj(items, vec![]), State::default());
    assert!(out.failed.is_empty());
    assert!(!home.path("gone.txt").exists());
    assert_eq!(out.applied, ["remove gone.txt"]);
}

#[cfg(unix)]
#[test]
fn link_remove_only_if_still_link_to_target() {
    let home = Home::new("exec-link-remove");
    home.write("t/f", "x");
    home.write("other/f", "y");
    let target = home.path("t");
    // replaced by a real dir
    fs::create_dir_all(home.path("real")).unwrap();
    // replaced by a link elsewhere
    std::os::unix::fs::symlink(home.path("other"), home.path("foreign")).unwrap();
    // still ours
    std::os::unix::fs::symlink(&target, home.path("mine")).unwrap();
    let items = ["real", "foreign", "mine"]
        .into_iter()
        .map(|n| PlanItem::LinkRemove {
            path: home.path(n),
            target: target.clone(),
        })
        .collect();
    let (out, _) = run(&home, proj(items, vec![]), State::default());
    assert!(out.failed.is_empty());
    assert!(home.path("real").is_dir());
    assert!(fs::symlink_metadata(home.path("foreign")).is_ok());
    assert!(fs::symlink_metadata(home.path("mine")).is_err());
    let kept = out
        .notes
        .iter()
        .filter(|n| matches!(n, Note::Kept(_)))
        .count();
    assert_eq!(kept, 2, "{:?}", out.notes);
}

#[cfg(unix)]
#[test]
fn link_create_refuses_real_dir_and_foreign_link() {
    let home = Home::new("exec-link-refuse");
    fs::create_dir_all(home.path("real")).unwrap();
    home.write("elsewhere/f", "x");
    home.write("t/f", "x");
    std::os::unix::fs::symlink(home.path("elsewhere"), home.path("foreign")).unwrap();
    let items = ["real", "foreign"]
        .into_iter()
        .map(|n| PlanItem::LinkCreate {
            path: home.path(n),
            target: home.path("t"),
        })
        .collect();
    let edits = vec![(
        0,
        OwnedEdit::Link {
            path: "real".into(),
            target: "t".into(),
            present: true,
        },
    )];
    let before = snapshot(&home.dir);
    let (out, st) = run(&home, proj(items, edits), State::default());
    assert!(out.failed.is_empty());
    assert_eq!(snapshot(&home.dir), before);
    let skipped = out
        .notes
        .iter()
        .filter(|n| matches!(n, Note::Skipped(_)))
        .count();
    assert_eq!(skipped, 2);
    assert!(st.owned.links.is_empty());
}

#[cfg(unix)]
#[test]
fn link_create_makes_parent_and_rollback_removes_created_parent() {
    let home = Home::new("exec-link-parent");
    home.write("t/f", "x");
    home.write("blocker", "file");
    let link = home.path("p/q/skill");
    let created = |p: &Path| OwnedEdit::Created {
        path: p.display().to_string(),
    };
    let edits = vec![
        (0, created(&home.path("p"))),
        (0, created(&home.path("p/q"))),
    ];
    let items = vec![PlanItem::LinkCreate {
        path: link.clone(),
        target: home.path("t"),
    }];
    let (out, st) = run(&home, proj(items, edits), State::default());
    assert!(out.failed.is_empty());
    assert!(vibe::links::is_link_to(&link, &home.path("t")));
    assert_eq!(st.owned.created.len(), 2);

    let home = Home::new("exec-link-parent-rollback");
    home.write("t/f", "x");
    home.write("blocker", "file");
    let edits = vec![
        (0, created(&home.path("p"))),
        (0, created(&home.path("p/q"))),
    ];
    let items = vec![
        PlanItem::LinkCreate {
            path: home.path("p/q/skill"),
            target: home.path("t"),
        },
        PlanItem::LinkCreate {
            path: home.path("blocker/x"),
            target: home.path("t"),
        },
    ];
    let (out, st) = run(&home, proj(items, edits), State::default());
    assert!(!out.failed.is_empty());
    assert!(!home.path("p").exists());
    assert!(st.owned.created.is_empty());
}

#[cfg(unix)]
fn add(name: &str) -> PlanItem {
    PlanItem::ClaudeCli {
        op: CliOp::Add {
            name: name.into(),
            json: r#"{"type":"stdio","command":"x"}"#.into(),
        },
    }
}

#[cfg(unix)]
fn owned(name: &str) -> OwnedEdit {
    OwnedEdit::McpClaude {
        name: name.into(),
        canonical: Some(json::parse(r#"{"type":"stdio","command":"x"}"#).unwrap()),
    }
}

#[cfg(unix)]
fn stamp() -> Option<Stamp> {
    Some(Stamp {
        store_hash: "S".into(),
        mcp_hash: "M".into(),
    })
}

#[cfg(unix)]
#[test]
fn cli_items_run_last_and_record_success_only() {
    let home = Home::new("exec-cli");
    let wrapper = failing_wrapper(&home, "beta");
    home.write("a.txt", "A0");
    let items = vec![
        add("alpha"),
        fw(&home.path("a.txt"), "A1"),
        add("beta"),
        PlanItem::ClaudeCli {
            op: CliOp::Remove {
                name: "gamma".into(),
            },
        },
    ];
    let edits = vec![
        (0, owned("alpha")),
        (2, owned("beta")),
        (
            3,
            OwnedEdit::McpClaude {
                name: "gamma".into(),
                canonical: None,
            },
        ),
    ];
    let mut p = proj(items, edits);
    p.stamp = stamp();
    let mut s = State::default();
    s.owned.mcp_claude.push(("gamma".into(), Value::Null));
    let (out, st) = apply(&paths(&home), p, s, false, wrapper.to_str().unwrap());
    let calls = home.claude_calls();
    assert_eq!(
        calls[0],
        [
            "mcp",
            "add-json",
            "--scope",
            "user",
            "alpha",
            "{\"type\":\"stdio\",\"command\":\"x\"}"
        ]
    );
    assert_eq!(calls.len(), 3);
    assert_eq!(home.read("a.txt"), "A1");
    assert_eq!(out.failed.len(), 1, "{:?}", out.failed);
    assert!(out.failed[0].starts_with("claude mcp add beta:"));
    let names: Vec<&str> = st
        .owned
        .mcp_claude
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(names, ["alpha"]);
    assert_eq!(st.store_hash, "");
    let saved = state::load(&paths(&home)).unwrap();
    assert_eq!(saved, st);
}

#[cfg(unix)]
fn failing_wrapper(home: &Home, name: &str) -> PathBuf {
    let fake = common::fake_claude(home).join("claude");
    home.write_exec(
        "wrap/claude",
        &format!(
            "#!/bin/sh\nFAKE_FAIL_NAME='{name}' FAKE_EXIT=3 exec '{}' \"$@\"\n",
            fake.display()
        ),
    )
}

#[cfg(unix)]
#[test]
fn state_saved_after_cli_stage() {
    let home = Home::new("exec-state-cli");
    let wrapper = failing_wrapper(&home, "beta");
    let mut p = proj(
        vec![add("alpha"), add("beta")],
        vec![(0, owned("alpha")), (1, owned("beta"))],
    );
    p.stamp = stamp();
    let (out, st) = apply(
        &paths(&home),
        p,
        State::default(),
        false,
        wrapper.to_str().unwrap(),
    );
    assert_eq!(out.failed.len(), 1);
    assert_eq!(st.owned.mcp_claude.len(), 1);
    assert_eq!(st.store_hash, "");
    assert_eq!(st.mcp_hash, "");
    assert_eq!(state::load(&paths(&home)).unwrap(), st);

    // all succeed: stamps recorded
    let mut p = proj(vec![add("alpha")], vec![(0, owned("alpha"))]);
    p.stamp = stamp();
    let (out, st) = apply(
        &paths(&home),
        p,
        State::default(),
        false,
        wrapper.to_str().unwrap(),
    );
    assert!(out.failed.is_empty());
    assert_eq!((st.store_hash.as_str(), st.mcp_hash.as_str()), ("S", "M"));

    // stage (a) fails: no state file
    let home = Home::new("exec-state-nofile");
    home.write("blocker", "file");
    let mut p = proj(
        vec![fw(&home.path("blocker/x"), "x"), add("alpha")],
        vec![(1, owned("alpha"))],
    );
    p.stamp = stamp();
    let (out, st) = apply(
        &paths(&home),
        p,
        State::default(),
        false,
        "/nonexistent/vibe-test/claude",
    );
    assert!(!out.failed.is_empty());
    assert_eq!(st, State::default());
    assert!(!paths(&home).state_file().exists());
}

#[cfg(unix)]
#[test]
fn missing_claude_binary_is_a_skip_note_not_a_failure() {
    let home = Home::new("exec-no-claude");
    let p = proj(vec![add("alpha")], vec![(0, owned("alpha"))]);
    let (out, st) = apply(
        &paths(&home),
        p,
        State::default(),
        false,
        "/nonexistent/claude",
    );
    assert!(out.failed.is_empty(), "{:?}", out.failed);
    assert!(out.notes.contains(&Note::Skipped(
        "claude: CLI not found \u{2014} MCP not projected".to_string()
    )));
    assert!(st.owned.mcp_claude.is_empty());
}

#[test]
fn dry_run_changes_nothing() {
    let home = Home::new("exec-dry");
    home.write("a.txt", "A0");
    let items = vec![
        fw(&home.path("a.txt"), "A1"),
        fw(&home.path("new/b.txt"), "B"),
        PlanItem::LinkCreate {
            path: home.path("l"),
            target: home.path("a.txt"),
        },
        PlanItem::ClaudeCli {
            op: CliOp::Remove { name: "x".into() },
        },
    ];
    let before = snapshot(&home.dir);
    let mut p = proj(items, vec![(0, OwnedEdit::Created { path: "z".into() })]);
    p.stamp = Some(Stamp {
        store_hash: "S".into(),
        mcp_hash: "M".into(),
    });
    let (out, st) = apply(
        &paths(&home),
        p,
        State::default(),
        true,
        "/nonexistent/claude",
    );
    assert_eq!(snapshot(&home.dir), before);
    assert_eq!(st, State::default());
    assert!(out.failed.is_empty());
    assert!(out.applied[0].starts_with("would write "));
    assert_eq!(out.applied.len(), 4);
}

#[test]
fn state_roundtrip_and_corrupt() {
    let home = Home::new("exec-state-rt");
    let p = paths(&home);
    assert_eq!(state::load(&p).unwrap(), State::default());
    let mut s = State {
        store_hash: "a".into(),
        mcp_hash: "b".into(),
        ..State::default()
    };
    s.targets.insert("codex:mcp".into(), "h".into());
    s.owned.mcp_claude.push((
        "alpha".into(),
        json::parse(r#"{"type":"stdio","command":"x"}"#).unwrap(),
    ));
    s.owned.mcp_names.insert("codex".into(), vec!["n".into()]);
    s.owned.hooks.insert("claude".into(), "cmd".into());
    s.owned.plugins.insert("opencode".into(), "/p.js".into());
    s.owned.links.push(("/l".into(), "/t".into()));
    s.owned.created.push("/c".into());
    fs::create_dir_all(&p.state_dir).unwrap();
    fs::write(p.state_file(), json::to_pretty(&state::to_value(&s))).unwrap();
    assert_eq!(state::load(&p).unwrap(), s);
    assert!(
        home.read(".local/state/vibe/state.json")
            .contains("\"targets\": {\n    \"codex:mcp\": {\n      \"hash\": \"h\"")
    );

    let write = |t: &str| fs::write(p.state_file(), t).unwrap();
    write(r#"{"v":2}"#);
    assert!(state::load(&p).is_err());
    write("{nope");
    assert!(state::load(&p).is_err());
    write(r#"{"store_hash":"x"}"#);
    assert!(state::load(&p).is_err());
    write(r#"{"v":1,"store_hash":7}"#);
    assert!(state::load(&p).is_err());
    write(r#"{"v":1,"store_hash":"x","future":{"a":1},"owned":{"zzz":1}}"#);
    let loaded = state::load(&p).unwrap();
    assert_eq!(loaded.store_hash, "x");
    let again = state::to_value(&loaded);
    assert!(again.path(&["future"]).is_none());
}

#[test]
fn unchanged_run_does_not_touch_state_mtime() {
    let home = Home::new("exec-state-mtime");
    let p = paths(&home);
    let mut s = State::default();
    s.targets.insert("codex:mcp".into(), "h".into());
    let (_, s) = run(
        &home,
        proj(
            vec![],
            vec![(usize::MAX, OwnedEdit::Created { path: "/c".into() })],
        ),
        s,
    );
    let first = fs::metadata(p.state_file()).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    let (out, s2) = run(
        &home,
        proj(
            vec![],
            vec![(usize::MAX, OwnedEdit::Created { path: "/c".into() })],
        ),
        s.clone(),
    );
    assert!(out.failed.is_empty());
    assert_eq!(s2, s);
    assert_eq!(
        fs::metadata(p.state_file()).unwrap().modified().unwrap(),
        first
    );
}

#[test]
fn lock_busy_returns_none() {
    use vibe_core::lock::{Acquired, Lock, STALE};
    let home = Home::new("exec-lock");
    let p = paths(&home);
    assert_eq!(with_lock(&p, || 7).unwrap(), Some(7));
    let Acquired::Got(_held) = Lock::acquire(&p.lock_file(), STALE).unwrap() else {
        panic!("lock should be free");
    };
    assert_eq!(with_lock(&p, || 7).unwrap(), None);
}

#[test]
fn debug_redacts_file_bytes_and_cli_json() {
    let item = fw(Path::new("/x"), "SECRET-VALUE");
    assert!(!format!("{item:?}").contains("SECRET"));
    let cli = PlanItem::ClaudeCli {
        op: CliOp::Add {
            name: "n".into(),
            json: "SECRET-JSON".into(),
        },
    };
    assert!(!format!("{cli:?}").contains("SECRET"));
}

#[test]
fn merge_shifts_indexes() {
    let mut a = proj(vec![fw(Path::new("/a"), "a")], vec![]);
    let b = proj(
        vec![fw(Path::new("/b"), "b")],
        vec![
            (0, OwnedEdit::Created { path: "b".into() }),
            (usize::MAX, OwnedEdit::Created { path: "m".into() }),
        ],
    );
    a.merge(b);
    assert_eq!(a.edits[0].0, 1);
    assert_eq!(a.edits[1].0, usize::MAX);
}

fn ctx_run<T>(home: &Home, f: impl FnOnce(&Ctx) -> T) -> T {
    let p = paths(home);
    let st = store::load(&p).unwrap();
    let state = State::default();
    let bin = PathBuf::from("/bin/vibe");
    let harnesses = enabled_harnesses(&p, &st, None, false);
    f(&Ctx {
        paths: &p,
        store: &st,
        state: &state,
        vibe_bin: &bin,
        mode: Mode::Sync,
        harnesses: &harnesses,
    })
}

fn dirs_for(home: &Home, set: &[Harness]) -> Vec<PathBuf> {
    let p = paths(home);
    let st = store::load(&p).unwrap();
    let state = State::default();
    let bin = PathBuf::from("/bin/vibe");
    skill_dirs(&Ctx {
        paths: &p,
        store: &st,
        state: &state,
        vibe_bin: &bin,
        mode: Mode::Sync,
        harnesses: set,
    })
}

#[test]
fn skill_dirs_follow_the_effective_harness_set() {
    let home = Home::new("skill-dirs");
    let agents = home.path(".agents/skills");
    let claude = home.path(".claude/skills");
    assert_eq!(dirs_for(&home, &[Harness::Codex]), vec![agents.clone()]);
    assert_eq!(dirs_for(&home, &[Harness::Claude]), vec![claude.clone()]);
    assert_eq!(
        dirs_for(&home, &[Harness::Claude, Harness::Droid]),
        [agents.clone(), claude.clone()]
    );
    assert_eq!(dirs_for(&home, &[Harness::Opencode]), vec![agents]);
    assert!(dirs_for(&home, &[]).is_empty());
}

#[test]
fn project_all_plans_with_the_set_only() {
    let home = Home::new("project-all");
    let p = ctx_run(&home, project_all);
    assert!(
        p.items.len() == 1
            && matches!(
                &p.items[0],
                PlanItem::FileWrite { path, .. } if path.ends_with("plugins/vibe-store.js")
            ),
        "{p:?}"
    );
}

fn block(home: &Home, path: &Path, body: Option<&str>) -> Projection {
    ctx_run(home, |c| {
        block_item(
            c,
            Harness::Codex,
            "codex:instructions",
            path,
            Style::Html,
            body,
        )
    })
}

#[test]
fn block_item_creates_file_with_target_hash() {
    let home = Home::new("block-create");
    let path = home.path("t/AGENTS.md");
    let p = block(&home, &path, Some("hello\n"));
    assert_eq!(p.items.len(), 1);
    let PlanItem::FileWrite { bytes, .. } = &p.items[0] else {
        panic!()
    };
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.starts_with("<!-- vibe:store:begin v=1 hash="));
    let want = OwnedEdit::Target {
        key: "codex:instructions".into(),
        hash: Some(hash_bytes(b"hello\n")),
    };
    assert!(p.edits.contains(&(0, want)));
    assert!(p.edits.contains(&(
        0,
        OwnedEdit::Created {
            path: path.display().to_string()
        }
    )));
}

#[test]
fn block_item_idempotent_returns_no_item() {
    let home = Home::new("block-idem");
    let path = home.path("AGENTS.md");
    let p = block(&home, &path, Some("hello\n"));
    let PlanItem::FileWrite { bytes, .. } = &p.items[0] else {
        panic!()
    };
    fs::write(&path, bytes).unwrap();
    let again = block(&home, &path, Some("hello\n"));
    assert!(again.items.is_empty());
    assert!(again.notes.is_empty());
    // a state that forgot the hash gets a correction edit, no item
    assert_eq!(again.edits.len(), 1);
}

#[cfg(unix)]
#[test]
fn block_item_symlink_refused() {
    let home = Home::new("block-symlink");
    home.write("real.md", "x\n");
    std::os::unix::fs::symlink(home.path("real.md"), home.path("AGENTS.md")).unwrap();
    let p = block(&home, &home.path("AGENTS.md"), Some("b\n"));
    assert!(p.items.is_empty());
    assert!(matches!(&p.notes[0], Note::Refused(m) if m.contains("is a symlink")));
    assert!(matches!(
        guard_target(&home.path("AGENTS.md")),
        Err(Note::Refused(_))
    ));
    assert_eq!(home.read("real.md"), "x\n");
}

#[test]
fn block_item_invalid_utf8_refused() {
    let home = Home::new("block-utf8");
    fs::write(home.path("AGENTS.md"), [0xff, 0xfe, b'\n']).unwrap();
    let p = block(&home, &home.path("AGENTS.md"), Some("b\n"));
    assert!(p.items.is_empty());
    assert!(matches!(&p.notes[0], Note::Refused(m) if m.contains("UTF-8")));
}

#[test]
fn block_item_hand_edit_note() {
    let home = Home::new("block-hand");
    let path = home.path("AGENTS.md");
    let p = block(&home, &path, Some("one\n"));
    let PlanItem::FileWrite { bytes, .. } = &p.items[0] else {
        panic!()
    };
    let edited = String::from_utf8(bytes.clone())
        .unwrap()
        .replace("one", "hacked");
    fs::write(&path, edited).unwrap();
    let p = block(&home, &path, Some("one\n"));
    assert_eq!(p.items.len(), 1);
    assert!(matches!(&p.notes[0], Note::Info(m) if m.contains("edited inside the vibe block")));
}

#[test]
fn block_item_marker_error_refused() {
    let home = Home::new("block-reversed");
    let path = home.path("AGENTS.md");
    fs::write(
        &path,
        "<!-- vibe:store:end -->\n<!-- vibe:store:begin v=1 hash=0000000000000000 -->\n",
    )
    .unwrap();
    let p = block(&home, &path, Some("b\n"));
    assert!(p.items.is_empty());
    assert!(matches!(&p.notes[0], Note::Refused(_)));
}

#[test]
fn block_item_crlf_file_idempotent() {
    let home = Home::new("block-crlf");
    let path = home.path("AGENTS.md");
    fs::write(&path, "line one\r\nline two\r\n").unwrap();
    let p = block(&home, &path, Some("body\n"));
    let PlanItem::FileWrite { bytes, .. } = &p.items[0] else {
        panic!()
    };
    assert!(bytes.starts_with(b"line one\r\nline two\r\n"));
    fs::write(&path, bytes).unwrap();
    assert!(block(&home, &path, Some("body\n")).items.is_empty());
}

#[test]
fn block_item_remove_when_body_none() {
    let home = Home::new("block-remove");
    let path = home.path("AGENTS.md");
    fs::write(&path, "mine\n").unwrap();
    let p = block(&home, &path, Some("body\n"));
    let PlanItem::FileWrite { bytes, .. } = &p.items[0] else {
        panic!()
    };
    fs::write(&path, bytes).unwrap();
    let rm = block(&home, &path, None);
    let PlanItem::FileWrite { bytes, .. } = &rm.items[0] else {
        panic!()
    };
    assert_eq!(bytes, b"mine\n");
    assert!(rm.edits.contains(&(
        0,
        OwnedEdit::Target {
            key: "codex:instructions".into(),
            hash: None
        }
    )));
    fs::write(&path, "mine\n").unwrap();
    assert!(block(&home, &path, None).items.is_empty());
    assert!(
        block(&home, &home.path("missing.md"), None)
            .items
            .is_empty()
    );
}

fn jf(home: &Home, path: &Path, set: bool) -> Projection {
    ctx_run(home, |c| {
        json_file_item(c, Harness::Droid, path, |o| {
            if set {
                o.insert("added", Value::Bool(true));
            }
        })
    })
}

#[test]
fn json_file_item_unchanged_value_no_item() {
    let home = Home::new("json-unchanged");
    home.write("m.json", "{ \"a\":1,   \"b\":[ ] }");
    assert!(jf(&home, &home.path("m.json"), false).items.is_empty());
    assert!(
        jf(&home, &home.path("missing.json"), false)
            .items
            .is_empty()
    );
}

#[test]
fn json_file_item_invalid_json_refused() {
    let home = Home::new("json-invalid");
    home.write("m.json", "{nope");
    let p = jf(&home, &home.path("m.json"), true);
    assert!(p.items.is_empty());
    assert!(matches!(&p.notes[0], Note::Refused(_)));
    home.write("m.json", "[1]");
    assert!(matches!(
        &jf(&home, &home.path("m.json"), true).notes[0],
        Note::Refused(_)
    ));
}

#[test]
fn json_file_item_missing_file_created() {
    let home = Home::new("json-missing");
    let path = home.path("d/m.json");
    let p = jf(&home, &path, true);
    assert_eq!(p.items.len(), 1);
    assert!(p.edits.contains(&(
        0,
        OwnedEdit::Created {
            path: path.display().to_string()
        }
    )));
}

#[test]
fn json_file_item_keeps_foreign_keys() {
    let home = Home::new("json-foreign");
    home.write("m.json", "{\"z\":1.50,\"a\":{\"k\":[1,2]}}");
    let p = jf(&home, &home.path("m.json"), true);
    let PlanItem::FileWrite { bytes, .. } = &p.items[0] else {
        panic!()
    };
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert_eq!(
        text,
        "{\n  \"z\": 1.50,\n  \"a\": {\n    \"k\": [\n      1,\n      2\n    ]\n  },\n  \"added\": true\n}\n"
    );
    assert!(
        !p.edits
            .iter()
            .any(|(_, e)| matches!(e, OwnedEdit::Created { .. }))
    );
}

#[cfg(unix)]
#[test]
fn file_remove_rollback_restores_mode_and_bytes() {
    use std::os::unix::fs::PermissionsExt;
    let home = Home::new("exec-remove-mode");
    home.write("gone.txt", "precious");
    home.write("blocker", "file");
    fs::set_permissions(home.path("gone.txt"), fs::Permissions::from_mode(0o644)).unwrap();
    let items = vec![
        PlanItem::FileRemove {
            harness: Harness::Codex,
            show: "gone.txt".into(),
            path: home.path("gone.txt"),
        },
        PlanItem::LinkCreate {
            path: home.path("blocker/l"),
            target: home.path("blocker"),
        },
    ];
    let (out, _) = run(&home, proj(items, vec![]), State::default());
    assert!(!out.failed.is_empty());
    assert_eq!(home.read("gone.txt"), "precious");
    let mode = fs::metadata(home.path("gone.txt"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o644);
}

#[cfg(unix)]
#[test]
fn file_remove_on_symlink_removes_link_and_rollback_recreates_it() {
    let home = Home::new("exec-remove-symlink");
    home.write("real.md", "keep me");
    home.write("blocker", "file");
    std::os::unix::fs::symlink(home.path("real.md"), home.path("AGENTS.md")).unwrap();
    let rm = || PlanItem::FileRemove {
        harness: Harness::Codex,
        show: "AGENTS.md".into(),
        path: home.path("AGENTS.md"),
    };
    let (out, _) = run(&home, proj(vec![rm()], vec![]), State::default());
    assert!(out.failed.is_empty());
    assert!(fs::symlink_metadata(home.path("AGENTS.md")).is_err());
    assert_eq!(home.read("real.md"), "keep me");

    std::os::unix::fs::symlink(home.path("real.md"), home.path("AGENTS.md")).unwrap();
    let items = vec![
        rm(),
        PlanItem::LinkCreate {
            path: home.path("blocker/l"),
            target: home.path("blocker"),
        },
    ];
    let (out, _) = run(&home, proj(items, vec![]), State::default());
    assert!(!out.failed.is_empty());
    let m = fs::symlink_metadata(home.path("AGENTS.md")).unwrap();
    assert!(m.file_type().is_symlink());
    assert_eq!(
        fs::read_link(home.path("AGENTS.md")).unwrap(),
        home.path("real.md")
    );
    assert_eq!(home.read("real.md"), "keep me");
}

#[test]
fn failed_state_save_returns_the_old_state() {
    let home = Home::new("exec-save-fail");
    home.write("blocker", "file");
    let mut p = paths(&home);
    p.state_dir = home.path("blocker/state");
    let (out, st) = apply(
        &p,
        proj(
            vec![],
            vec![(usize::MAX, OwnedEdit::Created { path: "/c".into() })],
        ),
        State::default(),
        false,
        "/nonexistent/vibe-test/claude",
    );
    assert_eq!(out.failed.len(), 1, "{:?}", out.failed);
    assert_eq!(st, State::default());
}

#[test]
fn debug_redacts_mcp_canonical_json() {
    let secret = json::parse(r#"{"env":{"K":"SECRET-VALUE"}}"#).unwrap();
    let edit = OwnedEdit::McpClaude {
        name: "n".into(),
        canonical: Some(secret.clone()),
    };
    assert!(!format!("{edit:?}").contains("SECRET"));
    let mut s = State::default();
    s.owned.mcp_claude.push(("n".into(), secret));
    let dump = format!("{s:?}");
    assert!(!dump.contains("SECRET"));
    assert!(dump.contains("\"n\""));
}

#[test]
fn apply_refreshes_the_held_lock() {
    use vibe_core::lock::{Acquired, Lock, STALE};
    let home = Home::new("exec-refresh");
    let p = paths(&home);
    let Acquired::Got(_held) = Lock::acquire(&p.lock_file(), STALE).unwrap() else {
        panic!("busy")
    };
    fs::File::options()
        .write(true)
        .open(p.lock_file())
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(120))
        .unwrap();
    let plan = proj(vec![fw(&home.path("a.txt"), "A")], vec![]);
    let (out, _) = apply(&p, plan, State::default(), false, "/nonexistent/claude");
    assert!(out.failed.is_empty(), "{:?}", out.failed);
    let age = std::time::SystemTime::now()
        .duration_since(fs::metadata(p.lock_file()).unwrap().modified().unwrap())
        .unwrap();
    assert!(age < std::time::Duration::from_secs(30), "{age:?}");
}
