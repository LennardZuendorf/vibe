// vibe-spec: literal-file
//! The hook surface: `vibe-flow hook <event>` (F6, plan 1.2).
//!
//! Carries the write-rule path classes verbatim (both roots, exactly as
//! `flow/content/policy.json` spells them) because the guard has to match the
//! data the policy is written in — hence the file-level literal marker.
//!
//! A hook reports through its **exit code**, not through a report: `0` means
//! pass or degrade (the platform cannot tell the two apart), `2` means block
//! and is fed back to the model. Everything this module cannot prove is a
//! block exits `0`, and the one foreign process whose output gets forwarded —
//! the `detect-context.sh decide` fallback — is **shape-gated** first: only a
//! single, reason-bearing `warn:`/`block:` line is forwarded, so a crash, a
//! banner, a partial line or a stray second line can never become a block.
//!
//! Two events, mirroring the frozen oracle pair in
//! `oracles/bash/flow/hooks-fallback/` and the shipped engine
//! (`oracles/js/commands/hook.mjs`):
//!
//! - [`GUARD`] (`PreToolUse`) — file tools get the write policy's verdict
//!   translated to exit codes; a `Bash` call gets the warn-only text sniffer.
//! - [`STOP`] (`Stop`) — the re-entry guard, the warn-only TDD nudge, and the
//!   one promoted tooth: a fresh evidence receipt in a `*.verify` state.
//!
//! Where the two oracles disagree, this port follows the **engine** (the
//! shipped behaviour) and the difference is pinned as an expected divergence
//! in `tests/parity/flow.json`, never "fixed" by editing the frozen oracle.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::cmd::Outcome;
use crate::cursor::read_cursor;
use crate::paths::FlowPaths;
use crate::policy::{Verdict, decide, load_policy};

/// The `PreToolUse` event name, as the engine spells it.
pub const GUARD: &str = "pre-tool-use-guard";
/// The `Stop` event name, as the engine spells it.
pub const STOP: &str = "stop-gate";

/// The gate's escape hatch, byte-identical to the oracle's line.
const ABORT_HINT: &str =
    "  not verifying? abort with: bash .agents/skills/vibe/scripts/set-state.sh idle";

/// The `git status` ceiling (32 MiB, the engine's `PORCELAIN_MAX_BUFFER`):
/// enough for roughly 250k untracked rows, bounded so a runaway status cannot
/// eat the machine. Exceeding it is an *undeterminable* tree, never "clean".
const PORCELAIN_CAP: usize = 32 * 1024 * 1024;

fn line(text: &str) -> String {
    format!("{text}\n")
}

fn pass() -> Outcome {
    Outcome::new(0, String::new(), String::new())
}

/// Reads the hook payload off stdin once. A read failure is an empty payload,
/// not an error: every caller degrades on one.
pub fn read_stdin() -> Vec<u8> {
    let mut buffer = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut buffer);
    buffer
}

/// Runs one event. An absent or unknown name is a silent pass — it is a typo
/// or a `--help`, and the caller must not have read stdin for it.
pub fn run(paths: &FlowPaths, event: &str, stdin: &[u8]) -> Outcome {
    match event {
        GUARD => guard(paths, stdin),
        STOP => stop(paths, stdin),
        _ => pass(),
    }
}

// ---------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------

/// Appends one line to the warnings relay the inject hook drains next turn.
/// A missing data home or an unwritable log is a silent no-op.
fn append_warn(paths: &FlowPaths, message: &str) {
    if !paths.data_dir().is_dir() {
        return;
    }
    let opened = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.warnings_log());
    let Ok(mut file) = opened else {
        return;
    };
    let _ = file.write_all(line(message).as_bytes());
}

// ---------------------------------------------------------------------------
// pre-tool-use-guard
// ---------------------------------------------------------------------------

/// The `PreToolUse` tooth: the write policy for file tools, a warn-only text
/// sniffer for `Bash`.
pub fn guard(paths: &FlowPaths, stdin: &[u8]) -> Outcome {
    match crate::runtime::hook_cursor(paths) {
        Err(e) => return Outcome::new(2, String::new(), format!("vibe-guard: BLOCKED — {e}\n")),
        Ok(Some(cursor)) => {
            let result = serde_json::from_slice::<Value>(stdin)
                .map_err(|e| e.to_string())
                .and_then(|payload| crate::runtime::guard(paths, &cursor, &payload));
            return match result {
                Ok(()) => pass(),
                Err(e) => Outcome::new(2, String::new(), format!("vibe-guard: BLOCKED — {e}\n")),
            };
        }
        Ok(None) => {}
    }
    if !paths.detect_script().exists() || stdin.is_empty() {
        return pass();
    }
    let payload: Value = serde_json::from_slice(stdin).unwrap_or(Value::Null);
    let tool = payload
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    if tool == "Bash" {
        let command = payload
            .pointer("/tool_input/command")
            .and_then(Value::as_str)
            .unwrap_or("");
        if command.is_empty() {
            return pass();
        }
        let Some(hit) = sniff_bash(command) else {
            return pass();
        };
        append_warn(
            paths,
            &format!(
                "guard: bash command appears to write {hit} (bypasses the file-tool guard) (warn-only)"
            ),
        );
        return Outcome::new(
            0,
            String::new(),
            line(&format!(
                "vibe-guard: warn — a bash command looks like it writes {hit} (a guarded path); prefer the flow's write surface over a raw shell redirect. (warn-only)"
            )),
        );
    }
    let Some(raw) = target_path(&payload) else {
        return pass();
    };
    let stripped = strip_leading_root(&raw, paths.root());
    let relative = stripped.strip_prefix("./").unwrap_or(&stripped).to_string();
    let verdict =
        in_process_verdict(paths, &relative).unwrap_or_else(|| spawned_verdict(paths, &relative));
    if let Some(reason) = verdict.strip_prefix("block:") {
        return Outcome::new(
            2,
            String::new(),
            line(&format!("vibe-guard: BLOCKED — {reason}"))
                + &line(
                    "vibe-guard: transition with set-state.sh, or edit within the current state's write rules.",
                ),
        );
    }
    if let Some(reason) = verdict.strip_prefix("warn:") {
        append_warn(paths, &format!("guard: {reason}"));
        return Outcome::new(
            0,
            String::new(),
            line(&format!("vibe-guard: warn — {reason}")),
        );
    }
    pass()
}

/// `tool_input.file_path`, else `tool_input.notebook_path`; both must be a
/// non-empty string.
fn target_path(payload: &Value) -> Option<String> {
    let input = payload.get("tool_input")?.as_object()?;
    for key in ["file_path", "notebook_path"] {
        let found = input
            .get(key)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty());
        if let Some(text) = found {
            return Some(text.to_string());
        }
    }
    None
}

/// `p` with a leading `<root>/` removed; `p == root` is left alone so the
/// policy, not this helper, has the last word on an odd spelling.
fn strip_leading_root(path: &str, root: &Path) -> String {
    let root = root.to_string_lossy();
    if path == root {
        return path.to_string();
    }
    let prefix = if root.ends_with('/') {
        root.to_string()
    } else {
        format!("{root}/")
    };
    path.strip_prefix(&prefix).unwrap_or(path).to_string()
}

/// The write-invariant verdict, answered in process. `None` means the policy
/// **data** is unusable (absent, unreadable, refused, or a warn/block with no
/// reason) — the one case that falls back to the script, because answering
/// `allow` from an empty rule set would make every hard block vanish.
fn in_process_verdict(paths: &FlowPaths, relative: &str) -> Option<String> {
    let loaded = load_policy(paths.data_dir());
    if !loaded.errors.is_empty() || loaded.policy.rules.is_empty() {
        return None;
    }
    let state = read_cursor(paths).map_or_else(|_| "idle".to_string(), |cursor| cursor.state);
    let decision = decide(&loaded.policy, relative, &state);
    if decision.verdict == Verdict::Allow {
        return Some("allow".to_string());
    }
    if decision.reason.is_empty() {
        return None;
    }
    Some(format!("{}:{}", decision.verdict.as_str(), decision.reason))
}

/// The script fallback, for policy data the engine cannot use. Its stdin is
/// nailed shut: a `decide` that decided to read more input must not wedge the
/// hook, and the hook's own stdin is already spent.
fn spawned_verdict(paths: &FlowPaths, relative: &str) -> String {
    let output = Command::new("bash")
        .arg(paths.detect_script())
        .arg("decide")
        .arg(relative)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let stdout = output.ok().map(|out| out.stdout);
    shape_gated(stdout.as_deref())
}

/// Forward the fallback's answer only when the output **proves** it is one:
/// a single line reading exactly `allow`, or `warn:<reason>` / `block:<reason>`
/// with a non-empty reason. Every other shape — a spawn failure, a banner, a
/// diagnostic, a reasonless verdict, a real verdict followed by a second line
/// — is an allow. A block is a claim about the user's write; it has to be
/// provable from the bytes, not inferred from "the process printed something".
///
/// Only **trailing newlines** are dropped, exactly as the bash fallback's
/// `$(…)` drops them: both verdict shapes are anchored at the first byte
/// (`is_verdict`'s `block:?*`, the fallback's `case "$VERDICT"`), so
/// `"  block:x"` is a diagnostic with a block inside it, not a block.
fn shape_gated(stdout: Option<&[u8]>) -> String {
    let Some(bytes) = stdout else {
        return "allow".to_string();
    };
    let lossy = String::from_utf8_lossy(bytes);
    let text = lossy.trim_end_matches('\n');
    if is_verdict(text) {
        text.to_string()
    } else {
        "allow".to_string()
    }
}

/// The verdict contract `detect-context.sh`'s own `is_verdict` states.
fn is_verdict(text: &str) -> bool {
    if text.contains('\n') {
        return false;
    }
    if text == "allow" {
        return true;
    }
    for prefix in ["warn:", "block:"] {
        if let Some(reason) = text.strip_prefix(prefix) {
            return !reason.is_empty();
        }
    }
    false
}

/// The destructive verbs the sniffer treats as a write: a whitespace-delimited
/// whole word, exactly as the oracle's `(^|\s)(tee|truncate|mv|cp|rm)(\s|$)`.
const WRITE_WORDS: [&str; 5] = ["tee", "truncate", "mv", "cp", "rm"];

const LESSONS: [&str; 2] = ["docs/spec/lessons.md", ".spec/lessons.md"];
const ROOT_SPEC: [&str; 8] = [
    "docs/spec/product.md",
    "docs/spec/tech.md",
    "docs/spec/design.md",
    "docs/spec/plan.md",
    ".spec/product.md",
    ".spec/tech.md",
    ".spec/design.md",
    ".spec/plan.md",
];

/// The warn-only `Bash` sniffer: the guarded path class a write-shaped command
/// appears to touch, or `None`. False positives are certain (it is a text
/// scan), which is exactly why it can only warn.
fn sniff_bash(command: &str) -> Option<String> {
    if !writes(command) {
        return None;
    }
    if let Some(index) = find_guarded(command, &LESSONS) {
        return Some(LESSONS[index].to_string());
    }
    if let Some(index) = find_guarded(command, &ROOT_SPEC) {
        let spelling = if ROOT_SPEC[index].starts_with("docs/") {
            "docs/spec/"
        } else {
            ".spec/"
        };
        return Some(format!(
            "a root {spelling}{{product,tech,design,plan}}.md doc"
        ));
    }
    if hits_state_json(command) {
        if command.contains("set-state.sh") {
            return None; // sanctioned writer: never warn
        }
        return Some(".agents/skills/vibe/state.json (use set-state.sh)".to_string());
    }
    None
}

/// A write-shaped operator anywhere: a redirect, a destructive verb, or an
/// in-place `sed`. Evaluated per line because both oracle legs see one line at
/// a time (`grep -E` here, `.` never matching `\n` in the engine).
fn writes(command: &str) -> bool {
    command.split('\n').any(|line| {
        line.contains('>')
            || line
                .split_ascii_whitespace()
                .any(|word| WRITE_WORDS.contains(&word))
            || sed_in_place(line)
    })
}

/// `(^|\s)sed(\s|$).*-i`: a `sed` word with `-i` anywhere after it on the line.
fn sed_in_place(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < line.len() {
        let start = index;
        while index < line.len() && !bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if &line[start..index] == "sed" && line[index..].contains("-i") {
            return true;
        }
        while index < line.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
    }
    false
}

/// The oracle's `(^|[^A-Za-z0-9_])` guard: the match must not be glued to a
/// word, so `xdocs/spec/lessons.md` is not a hit while `…/docs/spec/…` is.
fn boundary_before(text: &str, index: usize) -> bool {
    match text[..index].chars().next_back() {
        None => true,
        Some(char) => !(char.is_ascii_alphanumeric() || char == '_'),
    }
}

/// The leftmost boundary-anchored occurrence of any needle, as its index.
fn find_guarded(text: &str, needles: &[&str]) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut best: Option<(usize, usize)> = None;
    for (index, needle) in needles.iter().enumerate() {
        let wanted = needle.as_bytes();
        let mut at = 0;
        while at + wanted.len() <= bytes.len() {
            if &bytes[at..at + wanted.len()] == wanted && boundary_before(text, at) {
                if best.is_none_or(|(position, _)| at < position) {
                    best = Some((at, index));
                }
                break;
            }
            at += 1;
        }
    }
    best.map(|(_, index)| index)
}

/// `.agents/skills/vibe/state.json` is matched unanchored by the oracle;
/// `flow/state.json` is boundary-anchored.
fn hits_state_json(command: &str) -> bool {
    command.contains(".agents/skills/vibe/state.json")
        || find_guarded(command, &["flow/state.json"]).is_some()
}

// ---------------------------------------------------------------------------
// stop-gate
// ---------------------------------------------------------------------------

/// The `Stop` gate: re-entry guard, TDD nudge (warn-only), evidence receipt
/// (the one promoted block).
pub fn stop(paths: &FlowPaths, stdin: &[u8]) -> Outcome {
    match crate::runtime::hook_cursor(paths) {
        Err(e) => return Outcome::new(2, String::new(), format!("vibe-gate: BLOCKED — {e}\n")),
        Ok(Some(cursor)) => {
            if matches!(cursor["phase"].as_str(), Some("verify" | "ship"))
                && let Err(e) = crate::evidence::production_readiness(&cursor, None)
            {
                return Outcome::new(2, String::new(), format!("vibe-gate: BLOCKED — {e}\n"));
            }
            return pass();
        }
        Ok(None) => {}
    }
    if stop_hook_active(stdin) {
        return pass();
    }
    if !paths.detect_script().exists() {
        return pass();
    }
    let (state, feature) = cursor_state_feature(paths);
    // Only two predicates consult the working tree; an idle Stop must do no
    // git work at all (the hook has a 10 s platform timeout, and a killed
    // Stop hook does not block).
    let tree = if tree_needed(&state) {
        git_porcelain(paths.root())
    } else {
        Tree::unknown()
    };
    let mut stderr = String::new();
    let mut warn = |message: &str| {
        stderr.push_str(&line(&format!("vibe-gate: {message}")));
        append_warn(paths, &format!("gate: {message}"));
    };
    if let Some(message) = predicate_tdd(&state, &tree.text) {
        warn(&message);
    }
    let blocked = evidence_receipt_check(paths, &state, feature.as_deref(), &tree, &mut warn);
    match blocked {
        Some(blocked) => Outcome::new(2, String::new(), stderr + &blocked),
        None => Outcome::new(0, String::new(), stderr),
    }
}

/// A Stop that already fired must not block again, or the block loops
/// forever. `true` and `"true"` both count — reading a truthy value as
/// "already re-entered" costs one turn of gate enforcement, while reading it
/// as "first entry" wedges the session, so the fail-safe direction is this one.
fn stop_hook_active(stdin: &[u8]) -> bool {
    if stdin.is_empty() {
        return false;
    }
    let Ok(value) = serde_json::from_slice::<Value>(stdin) else {
        return false;
    };
    match value.get("stop_hook_active") {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::String(text)) => text == "true",
        _ => false,
    }
}

/// The cursor's state and feature; any failure is `idle` with no feature.
fn cursor_state_feature(paths: &FlowPaths) -> (String, Option<String>) {
    match read_cursor(paths) {
        Ok(cursor) => {
            let feature = feature_name(&cursor.feature);
            (cursor.state, feature)
        }
        Err(_) => ("idle".to_string(), None),
    }
}

/// The receipt name a persisted cursor feature resolves to.
///
/// Null, false and an empty string remain unnamed. Other scalar values follow
/// the frozen Bash reader with jq; numeric zero therefore names a receipt even
/// though the JS oracle treats it as unnamed. Bash without jq extracts quoted
/// strings only. Containers already differ between the oracles; use compact
/// JSON here. The cursor writer still accepts only strings or null.
fn feature_name(feature: &Value) -> Option<String> {
    match feature {
        Value::Null | Value::Bool(false) => None,
        Value::String(text) if text.is_empty() => None,
        Value::String(text) => Some(text.clone()),
        scalar => Some(scalar.to_string()),
    }
}

fn tree_needed(state: &str) -> bool {
    matches!(
        state,
        "feature.impl" | "quick.fix" | "feature.verify" | "quick.verify"
    )
}

/// What `git` answered. Three cases, and the difference between the last two
/// is load-bearing: "I could not look" is not "nothing changed".
struct Tree {
    /// `git status` answered; `text` is the truth, `""` meaning clean.
    determined: bool,
    /// There is no work tree here at all — the documented existence-only pass.
    no_git: bool,
    /// Decoded project-relative porcelain for the warn-only TDD predicate.
    text: String,
    entries: Vec<(String, String, Option<String>)>,
    /// Why an undetermined answer happened.
    why: String,
}

impl Tree {
    fn unknown() -> Tree {
        Tree {
            determined: false,
            no_git: false,
            text: String::new(),
            entries: Vec::new(),
            why: String::new(),
        }
    }
}

/// What the second `git` call produced: stdout bytes, or the reason it never
/// answered.
enum Status {
    Output(Vec<u8>),
    Failed(String),
}

/// The three-way answer, from what the two calls gave back (pure, so the
/// fail-closed path is testable without spawning anything).
fn tree_from(inside: bool, status: Status) -> Tree {
    if !inside {
        return Tree {
            determined: false,
            no_git: true,
            text: String::new(),
            entries: Vec::new(),
            why: String::new(),
        };
    }
    match status {
        Status::Failed(why) => Tree {
            determined: false,
            no_git: false,
            text: String::new(),
            entries: Vec::new(),
            why,
        },
        Status::Output(bytes) => {
            let mut records = bytes.split(|b| *b == 0).filter(|r| !r.is_empty());
            let mut entries = Vec::new();
            let mut rows = Vec::new();
            while let Some(record) = records.next() {
                if record.len() < 3 {
                    continue;
                }
                let xy = String::from_utf8_lossy(&record[..2]).into_owned();
                let path = String::from_utf8_lossy(&record[3..]).into_owned();
                let origin = if xy.contains(['R', 'C']) {
                    records.next()
                } else {
                    None
                };
                rows.push(match origin {
                    Some(origin) => format!("{xy} {} -> {path}", String::from_utf8_lossy(origin)),
                    None => format!("{xy} {path}"),
                });
                entries.push((
                    xy,
                    path,
                    origin.map(|o| String::from_utf8_lossy(o).into_owned()),
                ));
            }
            Tree {
                determined: true,
                no_git: false,
                text: rows.join("\n"),
                entries,
                why: String::new(),
            }
        }
    }
}

/// `git status --porcelain -z -uall -- .` — one row per untracked FILE, which is what
/// makes the staleness scan able to name the file the human touched and to
/// exclude this harness's own markers without excluding the project's files
/// sitting beside them.
fn git_porcelain(root: &Path) -> Tree {
    let check = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-inside-work-tree", "--show-prefix"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    let check = match check {
        Ok(check) if check.status.success() => check,
        _ => return tree_from(false, Status::Failed(String::new())),
    };
    let check_text = String::from_utf8_lossy(&check.stdout);
    let prefix = check_text
        .split_once('\n')
        .map_or("", |(_, tail)| tail.strip_suffix('\n').unwrap_or(tail));
    let spawned = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "-z", "-uall", "--", "."])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(err) => return tree_from(true, Status::Failed(err.kind().to_string())),
    };
    let mut bytes = Vec::new();
    let overflowed = match child.stdout.take() {
        Some(mut out) => read_capped(&mut out, PORCELAIN_CAP, &mut bytes),
        None => true,
    };
    if overflowed {
        let _ = child.kill();
        let _ = child.wait();
        return tree_from(true, Status::Failed("ENOBUFS".to_string()));
    }
    match child.wait() {
        Ok(status) if status.success() => {
            let mut tree = tree_from(true, Status::Output(bytes));
            tree.entries = tree
                .entries
                .into_iter()
                .filter_map(|(xy, path, origin)| {
                    path.strip_prefix(prefix)
                        .filter(|p| !p.is_empty())
                        .map(|p| {
                            (
                                xy,
                                p.to_string(),
                                origin.map(|o| o.strip_prefix(prefix).unwrap_or(&o).to_string()),
                            )
                        })
                })
                .collect();
            tree.text = tree
                .entries
                .iter()
                .map(|(xy, p, origin)| match origin {
                    Some(o) => format!("{xy} {o} -> {p}"),
                    None => format!("{xy} {p}"),
                })
                .collect::<Vec<_>>()
                .join("\n");
            tree
        }
        _ => tree_from(true, Status::Failed("git status failed".to_string())),
    }
}

/// Reads at most `cap` bytes; `true` when the source held more.
fn read_capped(source: &mut impl Read, cap: usize, into: &mut Vec<u8>) -> bool {
    let mut limited = source.take(cap as u64 + 1);
    let _ = limited.read_to_end(into);
    into.len() > cap
}

/// The `/`-joined spelling a gate message shows for `path`.
///
/// The frozen oracle prints `$ROOT/<rel>` and its separator is always `/`, so
/// the gate text is byte-identical across platforms only if `\` never leaks
/// into it. Decided against `oracles/bash/flow/hooks-fallback/stop-gate.sh`:
/// the oracle is the contract, the test is not.
#[cfg(windows)]
fn shown_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(not(windows))]
fn shown_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Keep values inside one diagnostic line; escaping backslash makes it unambiguous.
fn diagnostic_value(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\\' => "\\\\".to_string(),
            ch if ch.is_control() && (ch as u32 <= 0x1f || ch == '\u{7f}') => {
                format!("\\x{:02x}", ch as u32)
            }
            ch => ch.to_string(),
        })
        .collect()
}

/// `(^|/)segment/` — the segment must begin the row or follow a `/`.
fn has_segment(row: &str, segment: &str) -> bool {
    let needle = format!("{segment}/");
    row.starts_with(&needle) || row.contains(&format!("/{needle}"))
}

/// Predicate 1 — impl touched `src/**` but no `tests/**`. Warn-only.
fn predicate_tdd(state: &str, changed: &str) -> Option<String> {
    if state != "feature.impl" && state != "quick.fix" {
        return None;
    }
    if changed.is_empty() {
        return None;
    }
    let rows = changed.split('\n').filter(|row| !row.is_empty());
    let mut touches_src = false;
    let mut touches_tests = false;
    for row in rows {
        touches_src |= has_segment(row, "src");
        touches_tests |= has_segment(row, "test") || has_segment(row, "tests");
    }
    if touches_src && !touches_tests {
        return Some(format!(
            "in {state}, src changed with no test changes — TDD expects a reproducing/covering test. (warn-only)"
        ));
    }
    None
}

/// The paths this harness itself writes on the turns it is gating. Exactly
/// three, so a project's own `.vibe/blocks/**` still counts as a change.
fn own_runtime_state(paths: &FlowPaths, path: &str) -> bool {
    let evidence = paths.posix_relative(&paths.legacy_evidence_dir());
    if path.starts_with(".vibe/run/")
        || path == evidence
        || path.starts_with(&format!("{evidence}/"))
    {
        return true;
    }
    if path == paths.posix_relative(&paths.warnings_log()) {
        return true;
    }
    path == ".vibe/last-inject"
}

/// Predicate 2 — the one promoted blocking tooth. Returns the block text, or
/// `None` to continue.
fn evidence_receipt_check(
    paths: &FlowPaths,
    state: &str,
    feature: Option<&str>,
    tree: &Tree,
    warn: &mut dyn FnMut(&str),
) -> Option<String> {
    if state != "feature.verify" && state != "quick.verify" {
        return None;
    }
    let name = match (state, feature) {
        ("quick.verify", _) => "quick.md".to_string(),
        (_, Some(feature)) => format!("feature-{feature}.md"),
        (_, None) => {
            warn(
                "in feature.verify but the cursor names no feature — cannot resolve the evidence receipt; skipping the gate. (warn-only)",
            );
            return None;
        }
    };
    let current = paths.evidence_dir().join(&name);
    let legacy = paths.legacy_evidence_dir().join(&name);
    // A present new receipt is authoritative, including invalid or stale ones.
    let legacy_layout = paths.legacy_cursor_path().exists() && !paths.cursor_path().exists();
    let receipt = if !matches!(crate::cursor::regular_file_absent(&current), Ok(true))
        || (!legacy.exists() && !legacy_layout)
    {
        current
    } else {
        legacy
    };
    let shown = diagnostic_value(&shown_path(&receipt));
    let existing = std::fs::metadata(&receipt)
        .ok()
        .filter(std::fs::Metadata::is_file);
    let Some(receipt_stat) = existing else {
        return Some(
            line(&format!(
                "vibe-gate: BLOCKED — {state} needs an evidence receipt before 'done'."
            )) + &line(&format!("  expected: {shown}"))
                + &line(
                    "  it must record the commands you ran and their observed output (per unit ID for a feature).",
                )
                + &line(ABORT_HINT),
        );
    };
    // Fail CLOSED on an undeterminable tree: the receipt exists, so the only
    // question left is whether the tree moved under it, and "unknown" must not
    // read as "nothing changed". `no_git` is deliberately NOT this case — a
    // target without git has always degraded to existence-only, and blocking
    // it would wedge every such install.
    if !tree.determined {
        if tree.no_git {
            return None;
        }
        return Some(
            line(
                "vibe-gate: BLOCKED — the working tree state could not be determined, so the receipt cannot be trusted.",
            ) + &line(&format!("  receipt: {shown}"))
                + &line(&format!(
                    "  git status --porcelain -z -uall did not answer ({}).",
                    tree.why
                ))
                + &line(
                    "  a huge UN-IGNORED untracked tree is the usual cause — gitignore it (or commit it) and re-run the Stop.",
                )
                + &line(
                    "  \"I could not look\" is not \"nothing changed\", so this blocks rather than passing you through.",
                )
                + &line(ABORT_HINT),
        );
    }
    if tree.text.is_empty() {
        return None;
    }
    let Ok(receipt_time) = receipt_stat.modified() else {
        return None;
    };
    for (status, path, _) in &tree.entries {
        if status.contains('D') {
            continue;
        }
        if own_runtime_state(paths, path) {
            continue;
        }
        let checked = std::fs::metadata(paths.root().join(path)).and_then(|stat| stat.modified());
        let modified = match checked {
            Ok(modified) => modified,
            Err(err) => {
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) && !path.contains('\u{fffd}')
                {
                    continue;
                }
                return Some(
                    line(
                        "vibe-gate: BLOCKED — a changed path could not be checked, so the receipt cannot be trusted.",
                    ) + &line(&format!("  receipt: {shown}"))
                        + &line(&format!(
                            "  could not stat: {} ({})",
                            diagnostic_value(path),
                            diagnostic_value(&err.to_string())
                        ))
                        + &line(
                            "  fix or remove that path, then re-run verification and rewrite the receipt.",
                        )
                        + &line(ABORT_HINT),
                );
            }
        };
        if modified > receipt_time {
            return Some(
                line("vibe-gate: BLOCKED — the evidence receipt is stale.")
                    + &line(&format!("  receipt: {shown}"))
                    + &line(&format!(
                        "  changed after it was written: {}",
                        diagnostic_value(path)
                    ))
                    + &line(
                        "  re-run verification and rewrite the receipt with fresh commands + output.",
                    )
                    + &line(ABORT_HINT),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::paths::Env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct NoEnv;

    impl Env for NoEnv {
        fn get(&self, _key: &str) -> Option<String> {
            None
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("vibe-flow-hook-{name}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    /// A root with the legacy data home: the shipped policy and a placeholder
    /// `detect-context.sh` (the guard only stats it; the in-process answer
    /// never spawns).
    fn root_with_policy(name: &str) -> (PathBuf, FlowPaths) {
        let root = scratch(name);
        let vibe = root.join(".agents").join("skills").join("vibe");
        fs::create_dir_all(vibe.join("content")).expect("content dir");
        fs::create_dir_all(vibe.join("scripts")).expect("scripts dir");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../flow/content/policy.json"),
            vibe.join("content").join("policy.json"),
        )
        .expect("copy policy");
        fs::write(vibe.join("scripts").join("detect-context.sh"), "").expect("detect stub");
        let paths = FlowPaths::resolve(Some(&root), None, &root, &NoEnv);
        (root, paths)
    }

    fn set_cursor(root: &Path, state: &str, feature: Option<&str>) {
        let vibe = root.join(".agents").join("skills").join("vibe");
        let (flow, phase) = match state.split_once('.') {
            Some((flow, phase)) => (flow, phase),
            None => (state, state),
        };
        let feature = match feature {
            Some(name) => format!("\"{name}\""),
            None => "null".to_string(),
        };
        let body = format!("{{\"flow\":\"{flow}\",\"phase\":\"{phase}\",\"feature\":{feature}}}");
        fs::write(vibe.join("state.json"), body).expect("write cursor");
    }

    fn stamp(path: &Path, seconds: u64) {
        let file = fs::OpenOptions::new().write(true).open(path).expect("open");
        let time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(seconds);
        file.set_times(fs::FileTimes::new().set_modified(time))
            .expect("set mtime");
    }

    fn payload(tool: &str, input: &str) -> Vec<u8> {
        format!("{{\"tool_name\":\"{tool}\",\"tool_input\":{input}}}").into_bytes()
    }

    // -- guard ---------------------------------------------------------------

    #[test]
    fn a_block_is_two_lines_and_exit_two() {
        let (root, paths) = root_with_policy("guard-block");
        set_cursor(&root, "idle", None);
        let out = guard(
            &paths,
            &payload("Write", r#"{"file_path":"docs/spec/lessons.md"}"#),
        );
        assert_eq!(out.code, 2);
        assert_eq!(out.stdout, "");
        assert_eq!(
            out.stderr,
            "vibe-guard: BLOCKED — docs/spec/lessons.md is writable only during feature.compound, setup.apply, strategy.spec, or quick.verify (current: idle)\n\
             vibe-guard: transition with set-state.sh, or edit within the current state's write rules.\n"
        );
    }

    #[test]
    fn the_cursor_state_decides_so_the_same_edit_is_allowed_while_compounding() {
        let (root, paths) = root_with_policy("guard-state");
        set_cursor(&root, "feature.compound", Some("auth"));
        let out = guard(
            &paths,
            &payload("Write", r#"{"file_path":"./docs/spec/lessons.md"}"#),
        );
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
    }

    #[test]
    fn a_warn_also_queues_a_line_for_the_relay() {
        let (root, paths) = root_with_policy("guard-warn");
        set_cursor(&root, "feature.impl", Some("auth"));
        let out = guard(
            &paths,
            &payload(
                "Write",
                r#"{"file_path":"docs/spec/features/auth/tech.md"}"#,
            ),
        );
        assert_eq!(out.code, 0);
        assert!(
            out.stderr
                .starts_with("vibe-guard: warn — docs/spec/features edits"),
            "{}",
            out.stderr
        );
        let log = fs::read_to_string(paths.warnings_log()).expect("warnings.log");
        assert!(log.starts_with("guard: docs/spec/features edits"), "{log}");
    }

    #[test]
    fn bash_gets_a_sniff_warning_never_a_block() {
        let (root, paths) = root_with_policy("guard-bash");
        set_cursor(&root, "idle", None);
        let out = guard(
            &paths,
            &payload("Bash", r#"{"command":"echo hi >> docs/spec/lessons.md"}"#),
        );
        assert_eq!(out.code, 0);
        assert_eq!(
            out.stderr,
            "vibe-guard: warn — a bash command looks like it writes docs/spec/lessons.md (a guarded path); prefer the flow's write surface over a raw shell redirect. (warn-only)\n"
        );
        let log = fs::read_to_string(paths.warnings_log()).expect("warnings.log");
        assert_eq!(
            log,
            "guard: bash command appears to write docs/spec/lessons.md (bypasses the file-tool guard) (warn-only)\n"
        );
    }

    #[test]
    fn a_read_only_bash_command_does_not_warn() {
        let (root, paths) = root_with_policy("guard-bash-read");
        set_cursor(&root, "idle", None);
        let out = guard(
            &paths,
            &payload(
                "Bash",
                r#"{"command":"grep -n lessons docs/spec/lessons.md"}"#,
            ),
        );
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
    }

    #[test]
    fn a_bash_command_driving_set_state_is_never_warned_about_the_cursor() {
        let (root, paths) = root_with_policy("guard-bash-writer");
        set_cursor(&root, "idle", None);
        let out = guard(
            &paths,
            &payload(
                "Bash",
                r#"{"command":"echo x > .agents/skills/vibe/state.json && bash .agents/skills/vibe/scripts/set-state.sh idle"}"#,
            ),
        );
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
    }

    #[test]
    fn a_missing_detect_script_degrades_to_pass() {
        let (root, paths) = root_with_policy("guard-no-detect");
        fs::remove_file(root.join(".agents/skills/vibe/scripts/detect-context.sh")).expect("rm");
        let out = guard(
            &paths,
            &payload("Write", r#"{"file_path":"docs/spec/lessons.md"}"#),
        );
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
    }

    #[test]
    fn an_empty_or_malformed_payload_degrades_to_pass() {
        let (root, paths) = root_with_policy("guard-empty");
        set_cursor(&root, "idle", None);
        for stdin in [&b""[..], b"not json", b"[]", b"{\"tool_name\":7}"] {
            let out = guard(&paths, stdin);
            assert_eq!(out.code, 0, "{stdin:?}");
            assert!(out.stderr.is_empty(), "{stdin:?}");
        }
    }

    #[test]
    fn a_notebook_edit_uses_its_own_path_field() {
        let (root, paths) = root_with_policy("guard-notebook");
        set_cursor(&root, "idle", None);
        let out = guard(
            &paths,
            &payload("NotebookEdit", r#"{"notebook_path":"docs/spec/plan.md"}"#),
        );
        assert_eq!(out.code, 2);
        assert!(
            out.stderr.starts_with("vibe-guard: BLOCKED — "),
            "{}",
            out.stderr
        );
    }

    // -- the block-vs-crash shape gate --------------------------------------

    #[test]
    fn a_verdict_is_one_reason_bearing_line_and_nothing_else() {
        assert!(is_verdict("allow"));
        assert!(is_verdict("warn:some reason"));
        assert!(is_verdict("block:some reason"));
        assert!(!is_verdict("block:"));
        assert!(!is_verdict("warn:"));
        assert!(!is_verdict(""));
        assert!(!is_verdict("allow\n"));
        assert!(!is_verdict(
            "usage: detect-context.sh decide <path> [state]"
        ));
        assert!(!is_verdict("block:real\nusage banner"));
        assert!(!is_verdict("WARN:shout"));
    }

    #[test]
    fn every_non_proof_maps_to_allow_and_only_a_proof_survives() {
        assert_eq!(shape_gated(None), "allow");
        assert_eq!(shape_gated(Some(b"")), "allow");
        assert_eq!(shape_gated(Some(b"boom\n")), "allow");
        assert_eq!(shape_gated(Some(b"block:\n")), "allow");
        assert_eq!(shape_gated(Some(b"block:a\nblock:b\n")), "allow");
        // Leading whitespace is never part of a verdict: both oracle shapes are
        // anchored on the first byte (the bash `case`, `is_verdict`'s
        // `block:?*`), so a banner that merely contains a block line is a
        // diagnostic and must not become a block.
        assert_eq!(shape_gated(Some(b"  block:real reason\n")), "allow");
        assert_eq!(shape_gated(Some(b"\tallow\n")), "allow");
        // Trailing newlines are dropped exactly as `$(…)` drops them; trailing
        // spaces stay, because `block:?*` counts them as part of the reason.
        assert_eq!(
            shape_gated(Some(b"block:real reason \n\n")),
            "block:real reason "
        );
    }

    // -- the bash sniffer ----------------------------------------------------

    #[test]
    fn the_sniffer_names_the_first_guarded_class() {
        assert_eq!(
            sniff_bash("echo x >> docs/spec/lessons.md").as_deref(),
            Some("docs/spec/lessons.md")
        );
        assert_eq!(
            sniff_bash("cp a .spec/lessons.md").as_deref(),
            Some(".spec/lessons.md")
        );
        assert_eq!(
            sniff_bash("sed -i 's/a/b/' docs/spec/tech.md").as_deref(),
            Some("a root docs/spec/{product,tech,design,plan}.md doc")
        );
        assert_eq!(
            sniff_bash("rm .spec/plan.md").as_deref(),
            Some("a root .spec/{product,tech,design,plan}.md doc")
        );
        assert_eq!(
            sniff_bash("echo x > .agents/skills/vibe/state.json").as_deref(),
            Some(".agents/skills/vibe/state.json (use set-state.sh)")
        );
        assert_eq!(
            sniff_bash("echo x > flow/state.json").as_deref(),
            Some(".agents/skills/vibe/state.json (use set-state.sh)")
        );
    }

    #[test]
    fn the_sniffer_needs_a_write_op_and_a_boundary() {
        assert_eq!(sniff_bash("cat docs/spec/lessons.md"), None);
        assert_eq!(sniff_bash("grep -n x .spec/plan.md"), None);
        assert_eq!(sniff_bash("echo x > notes.md"), None);
        assert_eq!(sniff_bash("echo x > xdocs/spec/lessons.md"), None);
        // Unanchored on purpose: the oracle's `state\.json` has no boundary
        // guard, so a longer name that contains it still counts.
        assert_eq!(
            sniff_bash("echo x > .agents/skills/vibe/state.jsonx").as_deref(),
            Some(".agents/skills/vibe/state.json (use set-state.sh)")
        );
        assert_eq!(sniff_bash("sed docs/spec/plan.md"), None);
        assert_eq!(
            sniff_bash("echo x > docs/spec/lessons.md\n&& tee .agents/skills/vibe/state.json")
                .as_deref(),
            Some("docs/spec/lessons.md")
        );
    }

    // -- stop ----------------------------------------------------------------

    #[test]
    fn a_re_entry_is_a_silent_pass() {
        let (root, paths) = root_with_policy("stop-reentry");
        set_cursor(&root, "feature.verify", Some("auth"));
        for stdin in [
            &b"{\"stop_hook_active\":true}"[..],
            b"{\"stop_hook_active\":\"true\"}",
        ] {
            let out = stop(&paths, stdin);
            assert_eq!(out.code, 0, "{stdin:?}");
            assert!(out.stderr.is_empty(), "{stdin:?}");
        }
        assert!(!stop_hook_active(b"{\"stop_hook_active\":1}"));
        assert!(!stop_hook_active(b"{\"stop_hook_active\":false}"));
        assert!(!stop_hook_active(b"{\"stop_hook_active\":\"TRUE\"}"));
        assert!(!stop_hook_active(b""));
        assert!(!stop_hook_active(b"nope"));
    }

    #[test]
    fn an_idle_stop_does_no_work_and_prints_nothing() {
        let (root, paths) = root_with_policy("stop-idle");
        set_cursor(&root, "idle", None);
        let out = stop(&paths, b"{}");
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
        assert!(out.stdout.is_empty());
    }

    #[test]
    fn a_stop_without_detect_degrades_to_pass() {
        let (root, paths) = root_with_policy("stop-no-detect");
        fs::remove_file(root.join(".agents/skills/vibe/scripts/detect-context.sh")).expect("rm");
        let out = stop(&paths, b"{}");
        assert_eq!(out.code, 0);
        assert!(out.stderr.is_empty(), "{}", out.stderr);
    }

    #[test]
    fn a_missing_receipt_blocks_with_the_expected_path() {
        let (root, paths) = root_with_policy("stop-missing");
        set_cursor(&root, "feature.verify", Some("auth"));
        let receipt = paths.legacy_evidence_dir().join("feature-auth.md");
        let blocked = evidence_receipt_check(
            &paths,
            "feature.verify",
            Some("auth"),
            &Tree::unknown(),
            &mut |_| {},
        )
        .expect("block");
        // The oracle prints `$ROOT/<rel>` — `/`-joined on every platform. Spell
        // the expectation independently of the product helper so this stays a
        // check, not a tautology.
        let expected_path = receipt.display().to_string().replace('\\', "/");
        assert_eq!(
            blocked,
            format!(
                "vibe-gate: BLOCKED — feature.verify needs an evidence receipt before 'done'.\n  expected: {expected_path}\n  it must record the commands you ran and their observed output (per unit ID for a feature).\n{ABORT_HINT}\n"
            )
        );
    }

    #[test]
    fn quick_verify_looks_for_its_own_receipt_name() {
        let (_, paths) = root_with_policy("stop-quick");
        let blocked =
            evidence_receipt_check(&paths, "quick.verify", None, &Tree::unknown(), &mut |_| {})
                .expect("block");
        assert!(
            blocked.contains("quick.verify needs an evidence receipt"),
            "{blocked}"
        );
        assert!(blocked.contains("/evidence/quick.md"), "{blocked}");
    }

    #[test]
    fn a_verify_state_with_no_feature_warns_instead_of_blocking() {
        let (_, paths) = root_with_policy("stop-nofeature");
        let mut warned = Vec::new();
        let blocked =
            evidence_receipt_check(&paths, "feature.verify", None, &Tree::unknown(), &mut |m| {
                warned.push(m.to_string());
            });
        assert!(blocked.is_none());
        assert_eq!(
            warned,
            [
                "in feature.verify but the cursor names no feature — cannot resolve the evidence receipt; skipping the gate. (warn-only)"
            ]
        );
    }

    #[test]
    fn a_numeric_feature_still_names_a_receipt_and_blocks() {
        let (root, paths) = root_with_policy("stop-numeric-feature");
        fs::write(
            root.join(".agents/skills/vibe/state.json"),
            "{\"flow\":\"feature\",\"phase\":\"verify\",\"feature\":123}",
        )
        .expect("write cursor");
        let (state, feature) = cursor_state_feature(&paths);
        assert_eq!(state, "feature.verify");
        // Bash with jq and JS both stringify this nonzero numeric feature.
        assert_eq!(feature.as_deref(), Some("123"));
        let out = stop(&paths, b"{}");
        assert_eq!(out.code, 2, "{}", out.stderr);
        assert!(
            out.stderr
                .starts_with("vibe-gate: BLOCKED — feature.verify needs an evidence receipt"),
            "{}",
            out.stderr
        );
        assert!(
            out.stderr.contains("/evidence/feature-123.md"),
            "{}",
            out.stderr
        );
    }

    #[test]
    fn false_features_warn_but_false_strings_and_zero_name_receipts() {
        for (feature, expected_code, expected_name) in [
            (serde_json::json!(false), 0, None),
            (serde_json::json!("false"), 2, Some("false")),
            (serde_json::json!(0), 2, Some("0")),
            (serde_json::json!(true), 2, Some("true")),
        ] {
            let (root, paths) = root_with_policy("stop-feature-types");
            let cursor = serde_json::json!({
                "flow": "feature", "phase": "verify", "feature": feature
            });
            fs::write(
                root.join(".agents/skills/vibe/state.json"),
                cursor.to_string(),
            )
            .expect("write cursor");
            let out = stop(&paths, b"{}");
            assert_eq!(out.code, expected_code, "{feature}: {}", out.stderr);
            if let Some(name) = expected_name {
                assert!(
                    out.stderr.contains(&format!("feature-{name}.md")),
                    "{}",
                    out.stderr
                );
            } else {
                assert!(
                    out.stderr.contains("cursor names no feature"),
                    "{}",
                    out.stderr
                );
            }
        }
    }

    #[test]
    fn null_false_and_empty_features_are_unnamable() {
        assert_eq!(feature_name(&Value::Null), None);
        assert_eq!(feature_name(&serde_json::json!(false)), None);
        assert_eq!(
            feature_name(&serde_json::json!("false")),
            Some("false".into())
        );
        assert_eq!(feature_name(&serde_json::json!("")), None);
        assert_eq!(
            feature_name(&serde_json::json!("auth")),
            Some("auth".into())
        );
        assert_eq!(feature_name(&serde_json::json!(123)), Some("123".into()));
        assert_eq!(feature_name(&serde_json::json!(true)), Some("true".into()));
        assert_eq!(
            feature_name(&serde_json::json!(["a"])),
            Some("[\"a\"]".into())
        );
    }

    fn receipt_root(name: &str) -> (PathBuf, FlowPaths, PathBuf) {
        let (root, paths) = root_with_policy(name);
        let dir = paths.legacy_evidence_dir();
        fs::create_dir_all(&dir).expect("evidence dir");
        let receipt = dir.join("feature-auth.md");
        fs::write(&receipt, "commands and output\n").expect("receipt");
        stamp(&receipt, 1_600_000_000);
        (root, paths, receipt)
    }

    #[test]
    fn a_fresh_receipt_over_a_quiet_tree_passes() {
        let (_, paths, _) = receipt_root("stop-fresh");
        let tree = tree_from(true, Status::Output(Vec::new()));
        assert!(
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .is_none()
        );
    }

    #[test]
    fn a_file_changed_after_the_receipt_blocks() {
        let (root, paths, _) = receipt_root("stop-stale");
        let changed = root.join("notes.md");
        fs::write(&changed, "later\n").expect("write");
        stamp(&changed, 1_700_000_000);
        let tree = tree_from(true, Status::Output(b"?? notes.md\0".to_vec()));
        let blocked =
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .expect("block");
        assert!(
            blocked.starts_with("vibe-gate: BLOCKED — the evidence receipt is stale."),
            "{blocked}"
        );
        assert!(
            blocked.contains("  changed after it was written: notes.md\n"),
            "{blocked}"
        );
    }

    #[test]
    fn the_receipts_own_directory_and_the_relay_never_stale_it() {
        let (root, paths, _) = receipt_root("stop-own");
        let relay = paths.warnings_log();
        fs::write(&relay, "gate: something\n").expect("relay");
        stamp(&relay, 1_700_000_000);
        let marker = root.join(".vibe").join("last-inject");
        fs::create_dir_all(marker.parent().expect("parent")).expect("vibe dir");
        fs::write(&marker, "x").expect("marker");
        stamp(&marker, 1_700_000_000);
        let tree = tree_from(
            true,
            Status::Output(
                b"?? .agents/skills/vibe/evidence/feature-auth.md\0?? .agents/skills/vibe/warnings.log\0?? .vibe/last-inject\0"
                    .to_vec(),
            ),
        );
        assert!(
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .is_none()
        );
    }

    #[test]
    fn a_deleted_path_is_never_stale() {
        let (_, paths, _) = receipt_root("stop-deleted");
        let tree = tree_from(true, Status::Output(b" D gone.md\0".to_vec()));
        assert!(
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .is_none()
        );
    }

    #[test]
    fn a_rename_is_judged_by_its_destination() {
        let (root, paths, _) = receipt_root("stop-rename");
        let moved = root.join("moved.md");
        fs::write(&moved, "x\n").expect("write");
        stamp(&moved, 1_700_000_000);
        let tree = tree_from(true, Status::Output(b"R  moved.md\0old.md\0".to_vec()));
        let blocked =
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .expect("block");
        assert!(
            blocked.contains("changed after it was written: moved.md"),
            "{blocked}"
        );
    }

    #[test]
    fn an_undeterminable_tree_fails_closed_only_when_a_receipt_exists() {
        let (_, paths, _) = receipt_root("stop-unknown");
        let tree = tree_from(true, Status::Failed("ENOBUFS".to_string()));
        let blocked =
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .expect("block");
        assert!(
            blocked
                .starts_with("vibe-gate: BLOCKED — the working tree state could not be determined"),
            "{blocked}"
        );
        assert!(
            blocked.contains("git status --porcelain -z -uall did not answer (ENOBUFS)."),
            "{blocked}"
        );
    }

    #[test]
    fn no_git_at_all_keeps_the_existence_only_pass() {
        let (_, paths, _) = receipt_root("stop-nogit");
        let tree = tree_from(false, Status::Failed(String::new()));
        assert!(tree.no_git);
        assert!(
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .is_none()
        );
    }

    #[test]
    fn the_three_way_tree_answer_keeps_clean_apart_from_unknown() {
        let clean = tree_from(true, Status::Output(b"".to_vec()));
        assert!(clean.determined);
        assert_eq!(clean.text, "");
        let rows = tree_from(true, Status::Output(b"?? a\0?? b\0".to_vec()));
        assert_eq!(rows.text, "?? a\n?? b");
        let broken = tree_from(true, Status::Failed("git status failed".to_string()));
        assert!(!broken.determined && !broken.no_git);
        assert_eq!(broken.why, "git status failed");
    }

    #[test]
    fn a_src_change_without_tests_warns_only_in_impl_states() {
        let changed = "M  packages/web/src/a.rs";
        assert!(predicate_tdd("feature.impl", changed).is_some());
        assert!(predicate_tdd("quick.fix", changed).is_some());
        assert!(predicate_tdd("feature.verify", changed).is_none());
        assert!(predicate_tdd("idle", changed).is_none());
        assert!(predicate_tdd("feature.impl", "").is_none());
        assert!(
            predicate_tdd(
                "feature.impl",
                "M  packages/web/src/a.rs\nM  packages/web/tests/a.rs"
            )
            .is_none()
        );
        assert!(predicate_tdd("feature.impl", "M  web/src/a.rs\nM  web/test/a.rs").is_none());
        assert!(predicate_tdd("feature.impl", "?? docs/spec/tech.md").is_none());
    }

    // -- routing -------------------------------------------------------------

    #[test]
    fn an_unknown_or_absent_event_is_a_silent_pass() {
        let (_, paths) = root_with_policy("route");
        for name in ["", "nope", "session-start-doctrine"] {
            let out = run(&paths, name, b"junk");
            assert_eq!(out.code, 0, "{name}");
            assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{name}");
        }
    }

    #[test]
    fn only_the_two_ported_events_are_registered() {
        assert_eq!(GUARD, "pre-tool-use-guard");
        assert_eq!(STOP, "stop-gate");
    }

    #[test]
    fn strip_leading_root_only_removes_the_root_prefix() {
        let root = Path::new("/sandbox");
        assert_eq!(strip_leading_root("/sandbox/a/b.md", root), "a/b.md");
        assert_eq!(strip_leading_root("/sandbox", root), "/sandbox");
        assert_eq!(
            strip_leading_root("/elsewhere/a.md", root),
            "/elsewhere/a.md"
        );
        assert_eq!(strip_leading_root("a.md", root), "a.md");
    }
    // flow-v2/1.2: the new receipt wins; the old path is absence-only fallback.
    #[test]
    fn new_runtime_receipts_are_authoritative_with_legacy_fallback() {
        let (root, paths, legacy) = receipt_root("new-receipt");
        let tree = tree_from(true, Status::Output(Vec::new()));
        let new = paths.evidence_dir().join("feature-auth.md");
        fs::create_dir_all(new.parent().unwrap()).unwrap();
        fs::rename(&legacy, &new).unwrap();
        assert!(
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .is_none()
        );
        let source = root.join("src/new.rs");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "changed").unwrap();
        stamp(&new, 1_600_000_000);
        stamp(&source, 1_600_000_001);
        fs::write(&legacy, "fresh old receipt").unwrap();
        stamp(&legacy, 1_600_000_002);
        let tree = tree_from(true, Status::Output(b" M src/new.rs\0".to_vec()));
        assert!(
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .unwrap()
                .contains("receipt is stale")
        );
    }
    fn assert_stale_changed_path(name: &str) {
        let (root, paths, _) = receipt_root("nul-path");
        fs::write(root.join(name), "changed").unwrap();
        stamp(&root.join(name), 1_700_000_000);
        let tree = tree_from(true, Status::Output(format!("?? {name}\0").into_bytes()));
        let block =
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .expect("stale path blocks");
        assert!(block.contains("receipt is stale"));
        assert!(
            block.lines().skip(1).all(|l| l.starts_with("  ")),
            "{block}"
        );
    }

    // flow-v2/F6 review: the preferred producer preserves main's NUL protocol.
    #[cfg(unix)]
    #[test]
    fn newline_changed_path_is_scanned() {
        assert_stale_changed_path("changed\nsecond.txt");
    }

    #[test]
    fn nul_paths_and_every_rename_origin_are_scanned() {
        for name in ["changed file.txt", "å.txt"] {
            assert_stale_changed_path(name);
        }
        for xy in ["R ", "RD", " R", "C "] {
            let (root, paths, _) = receipt_root("origin");
            fs::write(root.join("new.txt"), "changed").unwrap();
            stamp(&root.join("new.txt"), 1_700_000_000);
            let data = format!("{xy} dest\0origin\0?? new.txt\0");
            let tree = tree_from(true, Status::Output(data.into_bytes()));
            let block =
                evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                    .expect("record after origin blocks");
            assert!(
                block.contains("changed after it was written: new.txt"),
                "{xy}: {block}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_changed_symlink_loop_fails_closed_but_vanished_paths_pass() {
        let (root, paths, _) = receipt_root("stat-error");
        std::os::unix::fs::symlink("loop", root.join("loop")).unwrap();
        let tree = tree_from(true, Status::Output(b"?? loop\0".to_vec()));
        let block =
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .expect("unknown stat blocks");
        assert!(block.contains("could not stat: loop"), "{block}");
        fs::remove_file(root.join("loop")).unwrap();
        assert!(
            evidence_receipt_check(&paths, "feature.verify", Some("auth"), &tree, &mut |_| {})
                .is_none()
        );
    }
    // flow-v2/F6 final review: metadata-only evidence uses full ancestry.
    #[cfg(unix)]
    #[test]
    fn receipt_absence_requires_valid_searchable_ancestry() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let mut failures = Vec::new();
        for feature in ["auth", "auth/part"] {
            for kind in [
                "denied-parent",
                "dangling-parent",
                "file-parent",
                "loop-parent",
                "missing-parent",
                "missing-leaf",
                "search-only",
                "search-only-missing",
                "metadata-only",
                "directory-leaf",
                "dangling-leaf",
                "loop-leaf",
                "valid",
            ] {
                let (_, paths) = root_with_policy("receipt-ancestry");
                let name = format!("feature-{feature}.md");
                let legacy = paths.legacy_evidence_dir().join(&name);
                fs::create_dir_all(legacy.parent().unwrap()).unwrap();
                fs::write(&legacy, "legacy").unwrap();
                let current = paths.evidence_dir().join(&name);
                let dir = current.parent().unwrap().to_path_buf();
                fs::create_dir_all(&dir).unwrap();
                fs::write(&current, "current").unwrap();
                match kind {
                    "denied-parent" => {
                        fs::set_permissions(&dir, fs::Permissions::from_mode(0o0)).unwrap()
                    }
                    "search-only" | "search-only-missing" => {
                        if kind == "search-only-missing" {
                            fs::remove_file(&current).unwrap();
                        }
                        fs::set_permissions(&dir, fs::Permissions::from_mode(0o111)).unwrap()
                    }
                    "metadata-only" => {
                        fs::set_permissions(&current, fs::Permissions::from_mode(0o0)).unwrap()
                    }
                    "dangling-parent" | "file-parent" | "loop-parent" | "missing-parent" => {
                        fs::remove_dir_all(&dir).unwrap();
                        match kind {
                            "dangling-parent" => symlink("missing", &dir).unwrap(),
                            "loop-parent" => symlink(dir.file_name().unwrap(), &dir).unwrap(),
                            "file-parent" => fs::write(&dir, "file").unwrap(),
                            _ => (),
                        }
                    }
                    "missing-leaf" | "directory-leaf" | "dangling-leaf" | "loop-leaf" => {
                        fs::remove_file(&current).unwrap();
                        match kind {
                            "directory-leaf" => fs::create_dir(&current).unwrap(),
                            "dangling-leaf" => symlink("missing", &current).unwrap(),
                            "loop-leaf" => symlink(current.file_name().unwrap(), &current).unwrap(),
                            _ => (),
                        }
                    }
                    _ => (),
                }
                let tree = tree_from(false, Status::Failed(String::new()));
                let block = evidence_receipt_check(
                    &paths,
                    "feature.verify",
                    Some(feature),
                    &tree,
                    &mut |_| {},
                );
                if matches!(
                    kind,
                    "denied-parent" | "search-only" | "search-only-missing"
                ) {
                    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
                }
                let expected = matches!(
                    kind,
                    "denied-parent"
                        | "dangling-parent"
                        | "file-parent"
                        | "loop-parent"
                        | "directory-leaf"
                        | "dangling-leaf"
                        | "loop-leaf"
                );
                if block.is_some() != expected {
                    failures.push(format!("{kind}: expected block={expected}; got {block:?}"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
    #[test]
    fn receipt_names_keep_nested_feature_components_and_reject_basename_decoys() {
        for feature in ["auth", "auth/part"] {
            for placement in ["current", "basename", "legacy"] {
                let (_, paths) = root_with_policy("receipt-name");
                let name = format!("feature-{feature}.md");
                let current = paths.evidence_dir().join(&name);
                let receipt = match placement {
                    "legacy" => paths.legacy_evidence_dir().join(&name),
                    "basename" => paths.evidence_dir().join(current.file_name().unwrap()),
                    _ => current.clone(),
                };
                fs::create_dir_all(receipt.parent().unwrap()).unwrap();
                fs::write(&receipt, "observed output").unwrap();
                let tree = tree_from(false, Status::Failed(String::new()));
                let block = evidence_receipt_check(
                    &paths,
                    "feature.verify",
                    Some(feature),
                    &tree,
                    &mut |_| {},
                );
                let expected = placement == "basename" && feature.contains('/');
                assert_eq!(
                    block.is_some(),
                    expected,
                    "{feature}/{placement}: {block:?}"
                );
                if expected {
                    assert!(block.unwrap().contains(&shown_path(&current)));
                }
            }
        }
    }
}
