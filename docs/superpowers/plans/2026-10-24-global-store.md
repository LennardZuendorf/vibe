# Global Store and Harness Projection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `crates/vibe` (bin `vibe`): a global store under `~/.config/vibe/` projected into Claude Code, Codex, OpenCode and Factory droid by `vibe install` / `sync` / `hook session-start`, plus the generic `vibe-core` modules it needs.

**Architecture:** Four vocabulary-free `vibe-core` modules (`dirs`, `lock`, `markers`, `json`). `crates/vibe` loads the store, asks each adapter for a `Projection` (plan items: file write, link create/remove, Claude CLI call), and one executor applies the plan (files in one `vibe_core::fs::commit`, then links, then CLI calls) and records ownership in `state.json`. The hook is the same planner restricted to file and link items.

**Tech Stack:** Rust 2024 / MSRV 1.88, `clap`, `serde`, `serde_json` only; hand-written TOML emitter; Node 18+ for the OpenCode plugin test and `contracts/`.

**Spec:** `docs/superpowers/specs/2026-10-23-global-store-design.md` (section numbers `§n` below refer to it).

## Global Constraints

- No commits. Every "Gate" step runs, for the touched crate `<c>`: `cargo fmt -p <c> --check`, `cargo clippy -p <c> --all-targets --locked -- -D warnings`, `cargo test -p <c> --locked`. Show real output. (Tasks 0 and 4 are the only places `--locked` is dropped, once each: T0 for the `vibe-core` dependency list, T4 for the new `vibe` package. Check `Cargo.lock` after each: only those entries change.)
- Baseline before editing (Task 0): record pre-existing warnings and test counts; judge each Gate by the delta.
- Banned crates (`deny.toml`): `toml`, `regex`, `tokio`, `tempfile`. Direct deps allowed: `clap`, `serde`, `serde_json`, `vibe-core` (path). No new package may enter `Cargo.lock` except the workspace member `vibe`.
- `serde_json` feature `preserve_order` is ruled out (breaks the `vibe-parity` `spec_corpus` test). Ordered JSON is a custom type in `vibe_core::json`.
- Lints: `unsafe_code = forbid`; clippy `unwrap_used`/`expect_used` = deny (test files start with `#![allow(clippy::unwrap_used, clippy::expect_used)]`).
- `crates/vibe-core/tests/contract.rs::core_sources_hold_no_tool_vocabulary` scans every `crates/vibe-core/src/**/*.rs` (comments and doc comments included) for these substrings: `.spec`, `docs/spec`, `docs/superpowers`, `state-machine`, `state.json`, `policy.json`, `AGENTS.md`, `CLAUDE.md`, every dotted state of `flow/state-machine.json` and its `flow.` prefix (`setup.`, `strategy.`, `feature.`, `quick.`), and the channel names `user-prompt.level`, `user-prompt.edge`, `user-prompt.event`, `user-prompt`, `session-start`, `agents-md`. Therefore new core code never spells those, including `session-start`, `agents-md`, `state.json`. All store/state/instruction file names and all harness names live in `crates/vibe`, never in core.
- Every test module (inline `#[cfg(test)] mod tests` or file) starts with `#![allow(clippy::unwrap_used, clippy::expect_used)]`, as `crates/vibe-core/src/fs.rs` does.
- Execution is strictly sequential, one implementer at a time, in the order T0, T1, ..., T13.
- Lessons: append each lesson to `.superpowers/sdd/2026-10-24-global-store/progress.md` the moment a command fails, a change is reverted or a defect is found late. T13 promotes them to `docs/spec/lessons.md`.
- Tests: no `tempfile`. Fixtures live under `std::env::temp_dir().join(format!("vibe-<crate>-test-{name}-{pid}-{counter}"))`. The binary runs via `env!("CARGO_BIN_EXE_vibe")` with `env_clear()`, then `PATH`, `HOME=<tmp>`, `LC_ALL=C`, `TZ=UTC`. Symlink and fake-CLI tests are `#[cfg(unix)]`; Windows is in the CI matrix.
- Files may carry `// vibe-spec: literal-file` as their first line when they must spell legacy root literals; run `bash tests/run.sh` (literals suite) in Task 13 to find any.
- Exit codes: `vibe_core::exit` (0 ok, 1 findings/refused, 2 usage). The hook always exits 0.
- Hash function everywhere (store hash, marker hash, target hash): FNV-1a 64-bit over bytes, rendered as 16 lowercase hex digits. `vibe_core::markers::body_hash` is the one implementation; `crates/vibe` reuses it (`pub fn hash_bytes(&[u8]) -> String` in the same module).
- Marker grammar (D24 v1, owner `store`): `<!-- vibe:store:begin v=1 hash=<h> -->` / `<!-- vibe:store:end -->` (Markdown), `# vibe:store:begin v=1 hash=<h>` / `# vibe:store:end` (TOML).
- Hook entry shape (Claude `settings.json`, Codex and Factory `hooks.json`): append to `hooks.SessionStart` the group `{"matcher":"startup|resume|clear","hooks":[{"type":"command","command":"\"<abs vibe>\" hook session-start --harness <id>","timeout":10}]}`. Harness ids: `claude`, `codex`, `opencode`, `droid`.

## Review Focus

Inputs the spec implies but its sections do not test; each has a test in the named task.

1. Store `AGENTS.md` missing, empty or non-UTF-8: expect the instruction block skipped with one note (never an empty block, never a crash); other projections proceed. (Task 5 load, Task 8/9 adapters)
2. Target file with CRLF line endings or no trailing newline: block insert/replace/remove leaves all other bytes identical. (Task 2)
3. Two sessions start at once, or a crash left a lock: second run is silent and exit 0; a stale lock (> 60 s) is recovered; no half-written target. (Task 1, Task 10)
4. Hook with unset `HOME`, unreadable store, corrupt `state.json`, or read-only target: exit 0, at most one stderr line, no panic, no stdout noise. (Task 10)
5. MCP server names with dots, spaces, unicode, `]` or quotes: valid TOML keys (quoted) for Codex, JSON keys for Factory, and names Claude rejects (outside `[A-Za-z0-9_-]`) skipped with a reason; a valid Claude name reaches `claude mcp add-json` as one argv element. (Task 6, Task 9)

---

## File Structure

```text
crates/vibe-core/src/{dirs,lock,markers,json}.rs     new, generic
crates/vibe-core/Cargo.toml, src/lib.rs               modify (Task 0)
crates/vibe/Cargo.toml
crates/vibe/src/main.rs            clap tree, dispatch, exit mapping
crates/vibe/src/paths.rs           Paths: every harness/store/state path from Env
crates/vibe/src/store/{mod,config,mcp,skills,import,init}.rs
crates/vibe/src/translate/{mod,claude,codex,factory}.rs, toml.rs
crates/vibe/src/{plan,exec,state,links,hooks_json}.rs
crates/vibe/src/adapters/{mod,claude,codex,factory,opencode,skills}.rs
crates/vibe/src/templates/opencode-plugin.js
crates/vibe/src/cmd/{sync,hook,install,uninstall,status,doctor,spec}.rs
crates/vibe/tests/common/mod.rs + one test file per command/adapter
crates/vibe/tests/opencode_plugin.test.mjs
contracts/vibe-store.schema.json, contracts/fixtures/vibe-store/<def>/{valid,invalid}/*.json
contracts/run.mjs, contracts/tests/run.test.mjs   modify
docs/spec/** , README.md, CHANGELOG.md, AGENTS.md  modify (Task 13)
```

## Execution order

Strictly sequential: **T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13**. No parallel groups. Shared files have one owner: T0 owns core `Cargo.toml`/`lib.rs` mod lines; T4 owns `crates/vibe/src/lib.rs` (every `pub mod` line), every module stub file and every `tests/common/mod.rs` helper (`Home`, `real_shaped`, `fake_claude`, `Out`); later tasks fill stubs and only read the helpers. T11 (contracts) needs T7 (state shape) and its last step needs T10.

---

### Task 0: Baseline and core scaffolding

**Files:** Modify `crates/vibe-core/Cargo.toml`, `crates/vibe-core/src/lib.rs`; Create empty `crates/vibe-core/src/{dirs,lock,markers,json}.rs` (each a single `//! <one line>` doc comment).

**Interfaces:** Produces: `vibe_core::{dirs,lock,markers,json}` modules; `serde` and `serde_json` as normal deps of core.

- [ ] **Step 1: Baseline.** Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked --no-fail-fast`, `bash contracts/run.sh`. Save pass/fail counts in your notes. Expected today: all green.
- [ ] **Step 2: Edit `crates/vibe-core/Cargo.toml`.** Add `[dependencies]` with `serde = { workspace = true }` and `serde_json = { workspace = true }`; keep the `[dev-dependencies]` `serde_json` line out (a normal dep covers tests).
- [ ] **Step 3: Edit `lib.rs`.** Add `pub mod dirs; pub mod json; pub mod lock; pub mod markers;` (alphabetical with the existing four). Update the crate doc line only if it states a module list.
- [ ] **Step 4: Lock update.** Run `cargo build -p vibe-core` once WITHOUT `--locked`. Verify `Cargo.lock` changed only in the `vibe-core` `dependencies` list (gains `serde`); no package added or removed.
- [ ] **Step 5: Gate (vibe-core).** Run the `--locked` gate. Expected: green; `contract.rs` still passes.

---

### Task 1: `vibe_core::dirs` and `vibe_core::lock`

**Files:** `crates/vibe-core/src/dirs.rs`, `crates/vibe-core/src/lock.rs`; tests as `#[cfg(test)] mod tests` in each file (core has no `tests/` helpers besides `contract.rs`).

**Interfaces:**
- Produces:
```rust
// dirs.rs
pub struct Env { /* BTreeMap<String,String>, windows: bool */ }
pub enum Kind { Config, State }
impl Env {
    pub fn new(vars: impl IntoIterator<Item = (String, String)>, windows: bool) -> Env;
    pub fn from_process() -> Env;                       // vars_os, drop non-unicode pairs; windows = cfg!(windows)
    pub fn var(&self, key: &str) -> Option<&str>;       // empty value counts as unset
    pub fn is_windows(&self) -> bool;
    pub fn home(&self) -> Result<PathBuf>;              // HOME, or USERPROFILE when windows
    pub fn config_home(&self) -> Result<PathBuf>;       // windows: APPDATA; else absolute XDG_CONFIG_HOME, else <home>/.config
    pub fn state_home(&self) -> Result<PathBuf>;        // windows: LOCALAPPDATA; else absolute XDG_STATE_HOME, else <home>/.local/state
    pub fn app_dir(&self, override_var: &str, kind: Kind, app: &str) -> Result<PathBuf>; // override if set (any non-empty), else <kind home>/<app>
}
// lock.rs
pub const STALE: std::time::Duration = Duration::from_secs(60);
pub enum Acquired { Got(Lock), Busy }
pub struct Lock { /* path */ }                          // Drop removes the file (ignore errors)
impl Lock { pub fn acquire(path: &Path, stale: Duration) -> Result<Acquired>; }
```
Errors: `Error::Invalid("no home directory (HOME is unset)")` (text names USERPROFILE on windows). A relative `XDG_*` value is ignored (XDG rule).

- [ ] **Step 1: Write failing tests** in `dirs.rs`:
  - `home_unix_uses_home` (`HOME=/h` → `/h`); `home_windows_uses_userprofile`; `home_missing_or_empty_is_error`.
  - `config_home_prefers_absolute_xdg` (`XDG_CONFIG_HOME=/x` → `/x`); `config_home_ignores_relative_xdg` (`rel` → `/h/.config`); `config_home_macos_is_dot_config` (no XDG → `/h/.config`); `config_home_windows_uses_appdata` (`APPDATA=C:\A` → `C:\A`, even if `XDG_CONFIG_HOME` set).
  - `state_home_*` mirror: `/h/.local/state`, `XDG_STATE_HOME`, windows `LOCALAPPDATA`.
  - `app_dir_override_wins` (`VIBE_STORE=/s` → `/s`, no `/vibe` appended); `app_dir_default` (`/h/.config/vibe`); `app_dir_state_default` (`/h/.local/state/vibe`).
  Tests in `lock.rs` (dir `std::env::temp_dir().join(format!("vibe-core-test-lock-{name}-{pid}"))`):
  - `second_acquire_is_busy_until_drop`: `Got`, then `Busy`, drop first, then `Got` again.
  - `creates_missing_parent_dir`.
  - `stale_lock_is_taken_over`: write the lock file, `File::set_modified(now - 120 s)` (stable since 1.75), acquire with `STALE` → `Got`.
  - `fresh_foreign_lock_is_busy`: lock file not created by us, mtime now → `Busy` and the file still exists after.
  - `two_stale_takers_one_wins`: stale file, two threads call `acquire`; exactly one gets `Got`, the other `Busy`, and the winner's lock file still exists afterwards.
- [ ] **Step 2:** `cargo test -p vibe-core dirs lock` → FAIL (items undefined).
- [ ] **Step 3: Implement.** `Lock::acquire`: `create_dir_all(parent)`; `OpenOptions::new().write(true).create_new(true)`; on `AlreadyExists` read `metadata.modified()`; if older than `stale`, take over by `fs::rename(lock, <lock>.stale.<pid>)` (a failed rename means another taker won: `Busy`), remove the renamed file, then retry `create_new` exactly once (`AlreadyExists` again: `Busy`); otherwise `Busy`; write the pid into the file. Other IO errors → `Error::Io`. No vocabulary words in comments (see Global Constraints).
- [ ] **Step 4:** `cargo test -p vibe-core` → PASS, including `contract.rs`.
- [ ] **Step 5: Gate (vibe-core).**

---

### Task 2: `vibe_core::markers`

**Files:** `crates/vibe-core/src/markers.rs` (unit tests inline).

**Interfaces:**
```rust
pub enum Style { Html, Hash }
#[derive(Debug, PartialEq, Eq)]
pub struct Block { pub range: std::ops::Range<usize>, pub hash: String, pub body: String }
//   range = byte range of both marker lines including their line terminators
//   body  = text strictly between the two marker lines (empty, or ends with a line terminator)
pub fn begin_line(style: Style, owner: &str, hash: &str) -> String;   // "<!-- vibe:store:begin v=1 hash=ab12 -->"
pub fn end_line(style: Style, owner: &str) -> String;
pub fn body_hash(body: &str) -> String;                                // = hash_bytes(body.as_bytes())
pub fn hash_bytes(bytes: &[u8]) -> String;
pub fn find(text: &str, style: Style, owner: &str) -> Result<Option<Block>>;
pub fn intact(block: &Block) -> bool;                                  // body_hash(&block.body) == block.hash
pub fn upsert(text: &str, style: Style, owner: &str, body: &str) -> Result<String>;
pub fn remove(text: &str, style: Style, owner: &str) -> Result<String>;
```
Rules: a marker is a whole line equal to the grammar (an optional single trailing `\r` is tolerated; leading/trailing spaces are not; other owners and other styles are ignored as plain text). `v=` must be `1` else `Error::Invalid("unsupported marker version")`. Two begins, two ends, end before begin, begin without end, end without begin → `Error::Invalid` naming the line number. `body` handed to `upsert` is normalized to end with `\n` (or be empty). Body lines are converted to the file's EOL (CRLF when the first line of `text` ends in `\r\n`) before hashing; `hash` covers the bytes exactly as written between the markers. Adapters detect change by `upsert(text, ..) == text`, never by comparing `Block.body` with the store text. `upsert` replaces `range` in place; when absent it appends: nothing before if `text` is empty, else ensure `text` ends with a newline, then one blank line, then the block. New lines use `\r\n` when the first line of `text` ends in `\r\n`, else `\n`. `remove` deletes `range` and one adjacent blank line it introduced (the blank line directly before the block, only if the block is at the end or preceded by one). Known limit (recorded in D48 and `cli-store.md`): text that had no trailing newline comes back with one.

- [ ] **Step 1: Write failing tests:**
  - `begin_line_html_and_hash_exact` pins both strings above and `# vibe:store:begin v=1 hash=ab12`.
  - `find_returns_body_hash_and_range` (block in the middle of text; slice `text[range]` equals the two lines + body).
  - `find_none_when_absent`; `find_ignores_other_owner_and_lookalikes` (indented marker, trailing space, `<!-- vibe:storex:begin …`, a marker inside surrounding prose on the same line).
  - Errors: `reversed_markers_refuse`, `duplicate_begin_refuses`, `begin_without_end_refuses`, `end_without_begin_refuses`, `unsupported_version_refuses`; each asserts the message holds the line number.
  - `upsert_into_empty`, `upsert_appends_with_blank_line_after_text_without_trailing_newline`, `upsert_replaces_in_place_keeping_surrounding_bytes` (assert prefix and suffix substrings byte-identical), `upsert_is_idempotent` (second call returns equal string).
  - `crlf_file_gets_crlf_block_and_find_tolerates_cr`, `crlf_upsert_then_intact_is_true`, `crlf_upsert_is_idempotent` and `no_trailing_newline_preserved_outside_block`.
  - `remove_restores_original_text` (`remove(upsert(t)) == t` for empty `t` and for `t` ending in a newline; for `t` without a trailing newline the result is `t + "\n"`); `remove_absent_is_identity`.
  - `intact_detects_hand_edit` (mutate one body byte → `intact == false`).
  - `hash_is_fnv1a64`: `body_hash("")` = `cbf29ce484222325`, `body_hash("a")` = `af63dc4c8601ec8c`.
- [ ] **Step 2:** `cargo test -p vibe-core markers` → FAIL.
- [ ] **Step 3: Implement.** Scan line by line with `split_inclusive('\n')` tracking byte offsets; one pass collects begin/end positions for the owner, then validate counts/order. Tricky part, the line classifier:
```rust
fn classify(line: &str, style: Style, owner: &str) -> Option<Kind /* Begin{hash} | End */> {
    let l = line.strip_suffix('\n').unwrap_or(line);
    let l = l.strip_suffix('\r').unwrap_or(l);
    let (open, close) = match style { Style::Html => ("<!-- ", " -->"), Style::Hash => ("# ", "") };
    let inner = l.strip_prefix(open)?.strip_suffix(close)?;      // exact: no other whitespace
    let rest = inner.strip_prefix("vibe:")?.strip_prefix(owner)?;
    if rest == ":end" { return Some(Kind::End) }
    let rest = rest.strip_prefix(":begin v=")?;                 // then "<ver> hash=<hex>"
    // split once on " hash="; ver must be "1" (else version error); hash must be 1+ lowercase hex
}
```
  `Kind::Begin` with unparsable fields is not a marker (look-alike), except a non-`1` version which is the version error.
- [ ] **Step 4:** `cargo test -p vibe-core markers` → PASS.
- [ ] **Step 5: Gate (vibe-core).**

---

### Task 3: `vibe_core::json` (order-preserving value)

**Files:** `crates/vibe-core/src/json.rs` (unit tests inline).

**Interfaces:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value { Null, Bool(bool), Number(serde_json::Number), String(String), Array(Vec<Value>), Object(Object) }
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Object(Vec<(String, Value)>);
impl Object {
    pub fn new() -> Object;
    pub fn get(&self, k: &str) -> Option<&Value>;
    pub fn get_mut(&mut self, k: &str) -> Option<&mut Value>;
    pub fn insert(&mut self, k: impl Into<String>, v: Value) -> Option<Value>; // replace in place, else append
    pub fn remove(&mut self, k: &str) -> Option<Value>;
    pub fn keys(&self) -> impl Iterator<Item = &str>;
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)>;
    pub fn len(&self) -> usize;  pub fn is_empty(&self) -> bool;
}
impl Value {
    pub fn str(s: &str) -> Value;                        // convenience
    pub fn as_str(&self) -> Option<&str>;  as_bool, as_array, as_array_mut, as_object, as_object_mut
    pub fn path(&self, keys: &[&str]) -> Option<&Value>;
    pub fn set_path(&mut self, keys: &[&str], v: Value) -> Result<()>;   // creates missing objects; Err if an intermediate is not an object
    pub fn remove_path(&mut self, keys: &[&str]) -> Option<Value>;       // does not prune emptied parents
    pub fn equal_ignoring_order(&self, other: &Value) -> bool;           // objects compared as sets of keys; arrays ordered
}
pub fn parse(text: &str) -> Result<Value>;               // duplicate object key anywhere → Error::Invalid("duplicate key \"k\"")
pub fn parse_object_or_empty(text: &str) -> Result<Object>; // whitespace-only → empty; non-object top level → Error::Invalid
pub fn to_pretty(v: &Value) -> String;                   // JSON.stringify(v, null, 2) + "\n"
pub fn to_compact(v: &Value) -> String;                  // no whitespace, no trailing newline
// serde: impl<'de> Deserialize<'de> for Value, impl Serialize for Value (both hand-written)
```
- [ ] **Step 1: Write failing tests:**
  - `roundtrip_preserves_key_order` (`{"b":1,"a":{"z":1,"y":2}}` → same order after parse + `to_pretty`; assert exact string `"{\n  \"b\": 1,\n  \"a\": {\n    \"z\": 1,\n    \"y\": 2\n  }\n}\n"`).
  - `pretty_matches_json_stringify_cases`: `{}` → `"{}\n"`, `[]` → `"[]\n"`, nested empty `{"a":[],"b":{}}` → `"{\n  \"a\": [],\n  \"b\": {}\n}\n"`; strings with `"` `\` `\n` `\u0001` → `\"`, `\\`, `\n`, `\u0001`; non-ASCII unescaped (`"é"` stays).
  - `compact_has_no_spaces`.
  - `duplicate_key_refuses` (top level and nested); `invalid_json_refuses_with_position`; `trailing_garbage_refuses`.
  - `numbers_survive`: `10`, `-3`, `1.5` re-print byte-exact; an integer-valued number at or above 2^64 (`18446744073709551616`) is refused with `Error::Invalid` (fail safe: the file is not rewritten).
  - `parse_object_or_empty` cases: `""` and `"  \n"` → empty; `"[]"` → Err; `"{}"` → empty.
  - `insert_replaces_in_place_and_appends`; `remove_keeps_order_of_rest`.
  - `set_path_creates_and_errors_through_scalar`; `remove_path_returns_value`.
  - `equal_ignoring_order_true_for_reordered_objects_false_for_reordered_arrays`.
  - `unknown_content_survives`: a 30-line settings-shaped fixture with nested arrays/null/bool round-trips with `parse` → `set_path` of one unrelated key → `remove_path` of it → equals original `Value`.
- [ ] **Step 2:** `cargo test -p vibe-core json` → FAIL.
- [ ] **Step 3: Implement.** `Serialize` uses `serialize_map(Some(len))` in stored order so `serde_json::to_string_pretty` (2-space, `{}`/`[]` for empties) gives the required text; `to_compact` uses `serde_json::to_string`. Tricky part, the deserializer:
```rust
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Value, D::Error> { d.deserialize_any(V) }
}
struct V;
impl<'de> Visitor<'de> for V {
    type Value = Value;
    // visit_unit/none → Null, visit_bool, visit_i64/u64 → Number,
    // visit_f64: serde_json (no arbitrary_precision) hands overflowing integers here as f64, so refuse any finite
    // value with fract()==0 and abs() >= 18446744073709551616.0 (Error::Invalid); non-finite → Err; else Number::from_f64,
    // visit_str/visit_string → String, visit_seq → collect Vec<Value>,
    fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Value, A::Error> {
        let mut out = Object::new();
        while let Some((k, v)) = m.next_entry::<String, Value>()? {
            if out.get(&k).is_some() { return Err(A::Error::custom(format!("duplicate key {k:?}"))); }
            out.0.push((k, v));
        }
        Ok(Value::Object(out))
    }
}
```
  Duplicate check is O(n²) per object; objects here are small.
- [ ] **Step 4:** `cargo test -p vibe-core json` → PASS; `contract.rs` still passes (no vocabulary).
- [ ] **Step 5: Gate (vibe-core).**

---

### Task 4: `crates/vibe` scaffold and workspace rules

**Files:** Create `crates/vibe/Cargo.toml`, `src/lib.rs` (declares EVERY `pub mod` the plan creates: `paths`, `store`, `translate`, `plan`, `exec`, `state`, `links`, `hooks_json`, `adapters`, `cmd`), `src/main.rs`, `src/paths.rs`, an empty stub file or `mod.rs` (one `//! stub, filled by Task N` line) for every other module and submodule listed in File Structure, `tests/common/mod.rs`, `tests/cli_surface.rs`. Modify root `Cargo.toml` (members), `crates/vibe-parity/src/workspace.rs` (`MEMBERS`), `crates/vibe-parity/tests/workspace_graph.rs` (count, `REVIEWED_PACKAGES`).

**Interfaces:**
- Consumes: `vibe_core::dirs::{Env, Kind}`, `vibe_core::exit`.
- Produces:
```rust
// paths.rs — every path in the program derives from here
pub struct Paths {
    pub home: PathBuf,
    pub store: PathBuf,         // VIBE_STORE | <config home>/vibe
    pub state_dir: PathBuf,     // VIBE_STATE | <state home>/vibe
    pub claude_dir: PathBuf,    // CLAUDE_CONFIG_DIR | ~/.claude
    pub claude_json: PathBuf,   // <CLAUDE_CONFIG_DIR>/.claude.json if set, else ~/.claude.json
    pub codex_dir: PathBuf,     // CODEX_HOME | ~/.codex
    pub factory_dir: PathBuf,   // ~/.factory
    pub opencode_dir: PathBuf,  // <XDG_CONFIG_HOME | ~/.config>/opencode  (also on macOS)
    pub agents_skills: PathBuf, // ~/.agents/skills
}
impl Paths { pub fn from_env(env: &Env) -> vibe_core::Result<Paths>; }
// derived helpers: store_json(), store_agents(), store_mcp(), store_skills(), state_file(), lock_file()
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Harness { Claude, Codex, Opencode, Droid }   // ids "claude","codex","opencode","droid"; Harness::ALL in that order; parse(&str)
```
Commands (clap derive, §5 table): `store {init [--instructions FILE] | import-mcp FILE [--keep-literals] | status [--json]}`, `sync [--harness a,b] [--dry-run] [--json]`, `hook session-start [--harness H]`, `install [--harness a,b] [--toolkit DIR] [--dry-run]`, `uninstall [--dry-run]`, `doctor`, `spec <args…>` (trailing var arg). Every command except `hook` and `spec` prints `vibe <cmd>: not yet implemented` to stderr and returns `exit::USAGE` until its task lands; `hook` returns 0 silently.

- [ ] **Step 1: Write failing tests.** `tests/cli_surface.rs`: `help_lists_every_command` (run `vibe --help`, assert each of `store`, `sync`, `hook`, `install`, `uninstall`, `doctor`, `spec` appears); `unknown_command_exits_2`; `hook_stub_exits_0_silently`. Unit tests in `paths.rs`: `paths_default_layout` (Env with only `HOME=/h` → `store=/h/.config/vibe`, `state_dir=/h/.local/state/vibe`, `claude_json=/h/.claude.json`, `codex_dir=/h/.codex`, `opencode_dir=/h/.config/opencode`, `agents_skills=/h/.agents/skills`); `paths_honour_overrides` (`VIBE_STORE`, `VIBE_STATE`, `CODEX_HOME`, `CLAUDE_CONFIG_DIR=/c` → `claude_dir=/c`, `claude_json=/c/.claude.json`, `XDG_CONFIG_HOME=/x` → `opencode_dir=/x/opencode`); `harness_parse_roundtrip`. `tests/common/mod.rs` (defined once here; later tasks only read it): `Out{code,stdout,stderr}`; `Home::new(name)`; `Home::vibe(&self, args, env) -> Out` (clean env as in Global Constraints, plus `HOME=<tmp>`, `current_dir=<tmp>`); `Home::with_store()` (valid `store.json`, `mcp.json` with a stdio server with env, an http server with headers, one `enabled:false`, one `harnesses:["codex"]`, `AGENTS.md`, `skills/a/SKILL.md`); `Home::real_shaped(name)` (everything in §1 "Observed": `~/.claude/AGENTS.md` symlink to `~/.config/opencode/AGENTS.md`, `~/.claude/skills` with real dirs and foreign symlinks, Codex `config.toml` with `[projects."/x"]` plus user `[mcp_servers.github]` and `hooks.json` with an Orca group, Factory `mcp.json` with user `mesh` and `hooks.json` with a Superset group and no `AGENTS.md`, Claude `settings.json` with a third-party hook group, user MCP in `~/.claude.json`, `~/.config/agent-stack/mcp.json` with literal keys; every JSON file in canonical 2-space form ending in a newline); `fake_claude(&Home) -> PathBuf` (unix script dir to prepend to `PATH`: appends argv, one line per call, to `<home>/claude-argv.log`, exits with `$FAKE_EXIT` when the add/remove name equals `$FAKE_FAIL_NAME`, else 0). `main.rs` stubs: every command except `hook` prints `vibe <cmd>: not yet implemented` and returns `exit::USAGE`; `vibe spec` stays a stub too (Task 10).
- [ ] **Step 2:** `cargo test -p vibe` → FAIL (package missing). First add the member and manifest (step 3), then confirm tests fail on assertions.
- [ ] **Step 3: Implement.** `Cargo.toml` (package `vibe`, `[[bin]] name = "vibe"`, deps `vibe-core`, `clap`, `serde_json`, `[lints] workspace = true`; no `serde` unless used). Root `members` gains `crates/vibe`. `MEMBERS` becomes `["vibe", "vibe-core", "vibe-parity", "vibe-spec"]` (sorted). In `workspace.rs` unit tests add the `vibe` package to the `meta()` fixture (`path("vibe-core")` + `reg("clap")`) and change the expected count in `the_intended_graph_passes` to `4`. In `workspace_graph.rs`: `assert_eq!(n, 4)`; edge filter includes `"vibe"` and `edges >= 3`; `REVIEWED_PACKAGES` becomes `[&str; 21]` with `"vibe"` inserted between `"unicode-ident"` and `"vibe-core"`.
- [ ] **Step 4:** Run once WITHOUT `--locked`: `cargo build -p vibe` (updates `Cargo.lock`: only the `vibe` package is added; verify with `git diff --stat Cargo.lock` equivalent `diff` against a saved copy, no other package changes). Then `cargo test -p vibe -p vibe-parity -p vibe-core --locked` → PASS.
- [ ] **Step 5: Planted-defect check.** Save copies of `crates/vibe/Cargo.toml` and `Cargo.lock`. Add `unicode-ident = "1"` (already locked, offline) to `crates/vibe/Cargo.toml` `[dependencies]`, run `cargo build -p vibe` unlocked, then `cargo test -p vibe-parity --locked`: it must FAIL with a message containing `outside the allowlist`. Restore both files from the copies and re-run `cargo test -p vibe-parity --locked` green. Record the failure output.
- [ ] **Step 6: Gate (vibe, vibe-parity).** Also run `cargo deny check` if installed locally (CI runs it); otherwise note it as CI-only.

---

### Task 5: Store model, load/validate, `store init`, `store import-mcp`

**Files:** `crates/vibe/src/store/{mod,config,mcp,skills,import,init}.rs`, `src/main.rs` (wire two commands), `tests/store.rs`.

**Interfaces:**
- Consumes: `Paths`, `vibe_core::json::*`, `vibe_core::fs::write_atomic`/`commit`, `markers::hash_bytes`.
- Produces:
```rust
// config.rs
pub struct StoreConfig { pub enabled: BTreeMap<Harness, bool>, pub skill_sources: Vec<PathBuf> }   // unknown keys ignored; `~` expanded via Paths.home
// mcp.rs
#[derive(Clone, Debug, PartialEq)]
pub enum Transport { Stdio, Http }
pub struct McpServer {
    pub name: String, pub transport: Transport,
    pub command: Option<String>, pub args: Vec<String>, pub env: Vec<(String, String)>,   // order preserved
    pub url: Option<String>, pub headers: Vec<(String, String)>,
    pub enabled: bool, pub harnesses: Option<Vec<Harness>>,
}
impl McpServer { pub fn applies_to(&self, h: Harness) -> bool; }
pub fn placeholders(s: &str) -> Vec<&str>;       // names of every ${NAME}, NAME = [A-Z_][A-Z0-9_]*
// skills.rs
pub struct Skill { pub name: String, pub dir: PathBuf, pub source: PathBuf }
pub struct Shadowed { pub name: String, pub kept: PathBuf, pub dropped: PathBuf }
pub fn discover(sources: &[PathBuf]) -> (Vec<Skill>, Vec<Shadowed>);   // first wins; child dir counts iff it has SKILL.md; sorted by name within a source; unreadable source skipped
// mod.rs
pub struct Store { pub config: StoreConfig, pub instructions: Option<String>, pub servers: Vec<McpServer>,
                   pub skills: Vec<Skill>, pub shadowed: Vec<Shadowed>, pub notes: Vec<String> }
pub fn load(paths: &Paths) -> Result<Store, String>;       // Err = refusal line (store.json/mcp.json invalid JSON or wrong type); missing files are fine
pub fn store_hash(paths: &Paths, store: &Store) -> String; // §8.4: bytes of store.json, AGENTS.md, mcp.json (absent = empty) + sorted (name, source dir) skill list + env!("CARGO_PKG_VERSION")
```
Sources order: `<store>/skills`, then `store.json` `skills.sources` in file order (toolkit last because `install` appends it last).

Secret placeholder rules (§3), implemented in `import.rs`: `pub fn import(file: &Path, paths: &Paths, keep_literals: bool) -> Result<ImportReport, String>`; `ImportReport { added: Vec<String>, kept_existing: Vec<String>, vars: Vec<String> /* names only */, warnings: Vec<String> }`.
- every header value → `${VIBE_<SERVER>_<FIELD>}`, unless the value is already a pure placeholder (`${…}` only), which is kept like env values; `<FIELD>` = header name upper-cased, non-alphanumerics → `_`;
- env value whose key matches (case-insensitive substring) `KEY|TOKEN|SECRET|PASSWORD|AUTH` → `${VIBE_<SERVER>_<ENVKEY>}`, unless the value is already a pure placeholder (`${…}` only);
- URL query value whose key matches the same → `${VIBE_<SERVER>_<QUERYKEY>}`; `<SERVER>` = name upper-cased, non-alphanumerics → `_`; collisions get `_2`, `_3`;
- `--keep-literals` keeps values, adds the warning `keeping literal values (--keep-literals)`;
- a server already in the store is kept (`kept_existing`), never overwritten; unknown fields are carried through unchanged; `type` defaults to stdio; `enabled` carried.
`init`: pure `pub fn init_files(paths: &Paths, instructions: Option<&Path>) -> Result<Vec<InitFile>, String>` (`InitFile { path, bytes: Option<Vec<u8>> /* None = directory */ }`, only entries still missing; symlink refusal) and `pub fn init(paths: &Paths, instructions: Option<&Path>) -> Result<InitReport, String>` which applies it (Task 10 maps `init_files` to plan items for `install`); creates `store.json` (`{"v": 1}` plus `"harnesses": {}`, `"skills": {"sources": []}`), `AGENTS.md` (copied from `--instructions`, else a one-line placeholder comment `<!-- vibe store: edit this file; every harness reads it -->` + newline), `mcp.json` (`{"mcpServers": {}}`), `skills/`; never overwrites an existing file (reported as `exists`). A symlinked `store.json` / `mcp.json` / `AGENTS.md` is refused (`lstat`).

- [ ] **Step 1: Write failing tests** (`tests/store.rs`, using `Home`; plus unit tests in the modules):
  - Valid-fixture-first (lessons rule): `Home::with_store()` (Task 4) writes a valid `store.json`, `mcp.json` (stdio server with env, http server with headers, one `enabled:false`, one with `harnesses:["codex"]`), `AGENTS.md`, `skills/a/SKILL.md`; `load` returns them. Each validation test then mutates ONE field: `type:"sse"` → refusal naming the server; `args` not an array; `env` value not a string; `enabled:"yes"`; `harnesses:["nope"]`; unknown field `x` → tolerated and absent from `McpServer`; invalid JSON → refusal naming the file; `mcp.json` missing → empty list, no error; `AGENTS.md` empty or invalid UTF-8 → `instructions = None` plus one note.
  - `placeholders_extracts_names`: `"Bearer ${A_B}x${c}"` → `["A_B"]` (lower-case `c` is not a placeholder).
  - `skills_first_wins_and_reports_shadowed`: two sources both holding `dup/SKILL.md` → one `Skill` from the first, one `Shadowed`; dir without `SKILL.md` ignored; a symlinked skill dir counts; sorted.
  - `store_hash_changes_on_each_input`: change each of the three files, one skill dir name, one source dir → five distinct hashes; unchanged inputs → equal hash; `AGENTS.md` absent equals empty (stable).
  - `init_creates_skeleton_and_is_idempotent` (second run: all `exists`, bytes unchanged); `init_copies_instructions`; `init_refuses_symlinked_store_file` (`#[cfg(unix)]`).
  - `import_replaces_secrets`: input holds header `Authorization: "Bearer sk-1"`, env `API_KEY: "sk-2"`, `PATH_X: "/bin"`, url `https://x/mcp?exaApiKey=k3&tools=a` → stored header `${VIBE_EXA_AUTHORIZATION}`, env `${VIBE_EXA_API_KEY}` (and `PATH_X` untouched), url `https://x/mcp?exaApiKey=${VIBE_EXA_EXAAPIKEY}&tools=a`; `vars` lists those three names; assert NO literal `sk-1`, `sk-2`, `k3` anywhere in stdout, stderr or the written `mcp.json`.
  - `import_keep_literals_warns`; `import_keeps_existing_name`; `import_keeps_already_placeholdered_value` (env value and header value both); `import_accepts_agent_stack_shape` (`type, command, args, env, url, headers, enabled, timeout` keys; `timeout` carried but not projected).
  - CLI: `vibe store init` twice → exit 0 both, second prints only `exists` lines; `vibe store import-mcp missing.json` → exit 1 one line.
- [ ] **Step 2:** `cargo test -p vibe store` → FAIL.
- [ ] **Step 3: Implement** the modules and wire `store init` / `store import-mcp` in `main.rs` (print `created <path>` / `exists <path>`; import prints `added <name>`, `kept <name> (already in store)`, `export <VAR>` lines — names only). Secret-key matching and placeholder-name derivation are small pure functions (`fn is_secret_key(&str) -> bool`, `fn var_name(server: &str, field: &str) -> String`) with their own unit tests.
- [ ] **Step 4:** `cargo test -p vibe store` → PASS.
- [ ] **Step 5: Gate (vibe).**

---

### Task 6: MCP translation and Codex TOML

**Files:** `crates/vibe/src/translate/{mod,claude,codex,factory}.rs`, `src/translate/toml.rs`, `tests/translate.rs` (or inline unit tests).

**Interfaces:**
- Consumes: `McpServer`, `placeholders`, `Harness`.
- Produces:
```rust
pub enum Translated<T> { Ok(T), Skip(String /* one-line reason, no trailing period */) }
pub fn claude(s: &McpServer) -> Translated<Value>;     // canonical JSON object, key order: type, command, args, env, url, headers
pub fn factory(s: &McpServer) -> Translated<Value>;    // {"type","command","args","env"} | {"type","url","headers"} (+ "disabled": true when !enabled)
pub fn codex(s: &McpServer) -> Translated<CodexServer>;
pub struct CodexServer { pub name: String, pub body: CodexBody, pub enabled: bool }
pub enum CodexBody {
    Stdio { command: String, args: Vec<String>, env: Vec<(String,String)>, env_vars: Vec<String> },
    Http  { url: String, bearer_token_env_var: Option<String>, http_headers: Vec<(String,String)>, env_http_headers: Vec<(String,String)> },
}
// toml.rs
pub fn basic_string(s: &str) -> String;                         // quoted + escaped
pub fn key(s: &str) -> String;                                  // bare iff [A-Za-z0-9_-]+ and non-empty, else basic_string
pub fn codex_tables(servers: &[CodexServer]) -> String;         // the body between the markers, no markers
pub fn scan_mcp_tables(text: &str) -> Vec<String>;              // server names that already own a table or dotted key in `text`
```
Rules (§4 table; OpenCode has no translator — the plugin reads `mcp.json` itself and the node tests own its shape): `claude`/`factory` pass `command,args,env,url,headers` as-is, omitting empty `args`, `env` and `headers`; placeholders in `${N}` form are kept (Claude expands at runtime; Factory expands in `env` and `headers` values only). `factory` and `codex` return `Skip("placeholder in command")`, `Skip("placeholder in args")`, `Skip("placeholder in url")` when `placeholders()` finds any in `command`/`args`/`url`. `claude` skips: `enabled:false` (`Skip("disabled")`), names outside `[A-Za-z0-9_-]+` (`Skip("name not accepted by claude")`). Codex env: value exactly `${K}` where `K` equals the env key → `env_vars` entry `K`; value with no placeholder → `env`; any other placeholder → `Skip("env <K> uses another variable")`. Codex headers: `Bearer ${N}` for header `Authorization` (case-insensitive) → `bearer_token_env_var = "N"` (only once; a second such header → `Skip`); exact `${N}` → `env_http_headers[name]="N"`; no placeholder → `http_headers`; otherwise `Skip("header <H> mixes text and a variable")`. `enabled:false` → `enabled = false` in the table (Codex) / `disabled: true` (Factory, OpenCode).
TOML field order per table: stdio `command, args, env, env_vars, enabled`; http `url, bearer_token_env_var, http_headers, env_http_headers, enabled`; `env`, `http_headers`, `env_http_headers` are inline tables `{ K = "v", "odd key" = "v" }`; `enabled` is emitted only when false. Table header: `[mcp_servers.<key(name)>]`; tables separated by one blank line; output ends with a newline.

Escaping (tricky, pinned):
```rust
pub fn basic_string(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"), '\r' => o.push_str("\\r"), '\t' => o.push_str("\\t"),
            '\u{8}' => o.push_str("\\b"), '\u{c}' => o.push_str("\\f"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => o.push_str(&format!("\\u{:04X}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"'); o
}
```
`scan_mcp_tables` matches, per line (trimmed of leading spaces): `[mcp_servers.<name>]`, `[mcp_servers.<name>.<sub>]`, `[mcp_servers."<quoted>"]` (unescape `\"` and `\\`), `[ mcp_servers . name ]` (whitespace around dots), and dotted keys `mcp_servers.<name>.<k> = …`; comments and lines inside a `#`-comment are ignored; `[[mcp_servers…]]` array-of-tables also counted.

- [ ] **Step 1: Write failing tests:**
  - `toml_basic_string_escapes`: `a"b\c` → `"a\"b\\c"`; newline/tab/CR; `\u{1}` → `"\u0001"`; `\u{7f}` → `"\u007F"`; `é` and `日本` unchanged; empty → `""`.
  - `toml_key_bare_or_quoted`: `exa` bare; `my.server`, `a b`, `ü`, `x]y`, `` (empty) → quoted.
  - `codex_tables_stdio_and_http_exact`: assert complete expected text for one stdio server (`command = "npx"`, `args = ["-y", "pkg"]`, `env = { MODE = "x" }`, `env_vars = ["API_KEY"]`) and one http server (`url = "https://x/mcp"`, `bearer_token_env_var = "TOK"`, `env_http_headers = { X-Y = "VAR" }` → key `X-Y` is bare since `-` is allowed; `http_headers = { "X Odd" = "v" }`), and `enabled = false` last.
  - `codex_translation_rules`: table-driven cases for each §4 cell: `K: "${K}"` → env_vars; `K: "lit"` → env; `K: "${OTHER}"` → Skip; `Authorization: "Bearer ${N}"` → bearer; `Authorization: "bearer ${N}"` (lower-case scheme) → Skip("header Authorization mixes text and a variable") because only the exact prefix `Bearer ` is recognised; `X: "${N}"` → env_http_headers; `X: "lit"` → http_headers; `X: "a${N}"` → Skip; placeholder in command/args/url → Skip with the pinned reason.
  - `claude_translation`: stdio keeps `${N}` in args/env; http keeps headers; `enabled:false` → Skip("disabled"); name `a.b` → Skip("name not accepted by claude"); output key order `type, command, args, env` asserted through `json::to_compact` exact string; a server with empty `args`/`env` has neither key (`{"type":"stdio","command":"x"}`).
  - `factory_translation`: `type` `stdio`/`http`, placeholders in `env`/`headers` kept, in `command`/`url` → Skip, `enabled:false` → `"disabled":true`.
  - `applies_to_filters_by_harnesses_allow_list`: `McpServer::applies_to` is false for claude when `harnesses:["codex"]`, true when `harnesses` is absent; adapters filter with it (no other helper).
  - `scan_mcp_tables_finds_all_forms`: fixture with each form above plus a commented-out header and a `[mcp_servers_other.x]` look-alike (not matched) → exact name set.
  - Review Focus 5: names `a.b`, `a b`, `ü`, `x]y`, `q"uote` each produce parseable, round-trippable TOML (re-scan the emitted text with `scan_mcp_tables` and get the name back).
- [ ] **Step 2:** `cargo test -p vibe translate` → FAIL.
- [ ] **Step 3: Implement** as specified. There is no OpenCode translator.
- [ ] **Step 4:** `cargo test -p vibe translate` → PASS.
- [ ] **Step 5: Gate (vibe).**

---

### Task 7: Plan items, executor, state, links, adapter dispatch

**Files:** `crates/vibe/src/{plan,exec,state,links,hooks_json}.rs`, `src/adapters/mod.rs`, `tests/exec.rs`, `tests/hooks_json.rs` (unit tests may stay inline). The stub files for `adapters/{claude,codex,factory,opencode,skills}.rs` exist from Task 4; this task makes each define `pub fn project(ctx: &Ctx, h: Harness) -> Projection`, `pub fn plan_hook_install(ctx: &Ctx, h: Harness) -> Projection`, `pub fn plan_uninstall(ctx: &Ctx, h: Harness) -> Projection` (and `skills::project(ctx: &Ctx, dirs: &[PathBuf]) -> Projection`) returning `Projection::default()`, so the dispatcher compiles; Tasks 8 and 9 fill them.

**Interfaces:**
- Consumes: `vibe_core::fs::{Write, commit, restore}`, `vibe_core::lock`, `vibe_core::json`, `Paths`, `Harness`, `Store`.
- Produces:
```rust
// plan.rs
pub enum PlanItem {
    FileWrite { harness: Harness, show: String, path: PathBuf, bytes: Vec<u8> },   // `original` is read by the executor at apply time
    FileRemove { harness: Harness, show: String, path: PathBuf },                    // used by uninstall
    LinkCreate { path: PathBuf, target: PathBuf },
    LinkRemove { path: PathBuf, target: PathBuf },
    ClaudeCli { op: CliOp },
}
pub enum CliOp { Add { name: String, json: String /* compact */ }, Remove { name: String } }
pub enum Note { Kept(String), Skipped(String), Refused(String /* full line body, e.g. "config.toml has content after the vibe block — fix by hand" */), Info(String) }
pub enum OwnedEdit {                                                                // applied to state only when its item succeeded
    McpClaude { name: String, canonical: Option<Value> },      // None = no longer owned
    McpNames { harness: Harness, names: Vec<String> },         // codex / droid owned names
    Hook { harness: Harness, command: Option<String> },
    Plugin { harness: Harness, path: Option<String> },
    Link { path: String, target: String, present: bool },
    Created { path: String },                                  // vibe created this file/dir; uninstall deletes it when empty / `{}`
    Target { key: String, hash: Option<String> },   // keys: claude:instructions, codex:instructions, codex:mcp, droid:instructions, droid:mcp
}
#[derive(Default)]
pub struct Projection { pub items: Vec<PlanItem>, pub notes: Vec<Note>, pub edits: Vec<(usize /* item index or usize::MAX = with file stage */, OwnedEdit)> }
// state.rs — shape pinned (§5, §8.3)
pub struct State { pub store_hash: String, pub mcp_hash: String, pub targets: BTreeMap<String,String> /* key (see OwnedEdit::Target) → hash */, pub owned: Owned }
pub struct Owned { pub mcp_claude: Vec<(String, Value)>, pub mcp_names: BTreeMap<String, Vec<String>>, pub hooks: BTreeMap<String,String>,
                   pub plugins: BTreeMap<String,String>, pub links: Vec<(String,String)>, pub created: Vec<String> }
pub fn load(paths: &Paths) -> Result<State, String>;          // missing → State::default(); corrupt/unknown v → Err(refusal)
pub fn to_value(&State) -> Value;  pub fn save_plan(paths: &Paths, &State) -> PlanItem;
```
`state.json` (on disk): `{"v":1,"store_hash":"<h>","mcp_hash":"<h>","targets":{"codex:mcp":{"hash":"<h>"}},"owned":{"mcp":{"claude":{"<name>":<canonical>},"codex":["n"],"droid":["n"]},"hooks":{"claude":"<cmd>"},"plugins":{"opencode":"<path>"},"links":[{"path":"…","target":"…"}],"created":["<path>"]}}`. `mcp_hash` = hash of `mcp.json` bytes (separate stamp because the hook never projects Claude MCP; design gap, see report).
```rust
// exec.rs
pub struct Outcome { pub applied: Vec<String>, pub failed: Vec<String>, pub notes: Vec<Note> }   // lines for output
pub fn apply(paths: &Paths, proj: Projection, state: State, dry_run: bool, claude_bin: &str) -> (Outcome, State);
// links.rs
pub fn create_link(path: &Path, target: &Path) -> Result<(), String>;   // create_dir_all(parent) first; unix symlink; non-unix → Err("links unsupported on this OS")
pub fn is_link_to(path: &Path, target: &Path) -> bool;                  // lstat + read_link equality
// adapters/mod.rs
pub struct Ctx<'a> { pub paths: &'a Paths, pub store: &'a Store, pub state: &'a State, pub vibe_bin: &'a Path, pub mode: Mode }
pub enum Mode { Hook, Sync }                     // Hook = files and links only
pub fn project(ctx: &Ctx, h: Harness) -> Projection;       // dispatches to adapters::{claude,codex,factory,opencode}::project, then skills
pub fn enabled_harnesses(paths: &Paths, store: &Store, filter: Option<&[Harness]>, require_dir: bool) -> Vec<Harness>;
//   enabled = store.json flag (default true) AND (when require_dir) the harness config dir exists. Only `install` passes require_dir = false.
```
Shared adapter helpers (`plan.rs`, used by Tasks 8 and 9 so no adapter repeats read/guard/compare/hash):
```rust
pub fn guard_target(path: &Path) -> Result<Option<String>, Note>;   // lstat: symlink → Refused("<path> is a symlink — not written"); invalid UTF-8 → Refused; missing → Ok(None)
pub fn block_item(ctx: &Ctx, harness: Harness, key: &str, path: &Path, style: Style, body: Option<&str>) -> Projection;
//   body Some → markers::upsert; None → remove block if present. No item when result == current text. Emits FileWrite, Target{key, hash_bytes(body as written)}, Created{path} when the file is new,
//   Info("<harness>: <show> edited inside the vibe block — replaced") when a block exists, !intact and body differs; marker errors → Refused(markers message).
pub fn json_file_item(ctx: &Ctx, harness: Harness, path: &Path, edit: impl FnOnce(&mut Object)) -> Projection;
//   parse_object_or_empty (invalid → Refused), apply edit, no item when the Value is unchanged (!=), else FileWrite(to_pretty) and Created when new.
```
`hooks_json.rs` (shared by Claude, Codex and Factory; moved here from Task 8):
```rust
pub fn hook_command(vibe_bin: &Path, h: Harness) -> Result<String, String>;   // "\"<abs>\" hook session-start --harness <id>"; path containing '"' → Err
pub fn add_hook(doc: &mut Value, command: &str);        // idempotent: no new group when any hook has this exact command
pub fn remove_hook(doc: &mut Value, command: &str);     // removes that hook; drops its group when `hooks` empties, drops `SessionStart`/`hooks` keys only when they become empty
pub fn has_hook(doc: &Value, command: &str) -> bool;
```
Executor contract (§8.2): (a) all `FileWrite` in one `vibe_core::fs::commit` after reading each file's `original`; (a2) `FileRemove` items run after `commit` with `std::fs::remove_file`, remembering each file's bytes; (b) link items in order; on a link failure, undo the links already done in reverse (`LinkCreate` → remove link and any parent dir recorded `Created` for it; `LinkRemove` → recreate), then restore removed files and written files via `vibe_core::fs::restore` (removed files as `Write{bytes: original, original: Some(original)}`); (c) CLI items last: `Command::new(claude_bin).args(["mcp","add-json","--scope","user",name,json])` / `["mcp","remove","--scope","user",name]`, stdin null, output captured; a missing binary or non-zero exit is a `failed` line `claude mcp <op> <name>: <stderr first line>`; each success applies its `OwnedEdit`; failure does not roll back (a)/(b). Dry run applies nothing and returns the item descriptions as `applied` lines prefixed `would `. The executor takes the lock (`lock_file()`, `STALE`) before reading `State` for the plan: provide `pub fn with_lock<T>(paths, f: impl FnOnce() -> T) -> Result<Option<T>, String>` (`None` = busy). State: `apply` returns the new `State` with every `OwnedEdit` of a succeeded item applied, and writes `state.json` after stage (c) whenever stages (a)-(b) succeeded (so successful CLI edits are recorded even when another CLI item failed); `store_hash`/`mcp_hash` are stamped only when `failed` is empty; nothing is saved when (a) or (b) fails; the file is written only when `to_value(new) != to_value(loaded)` (an unchanged run touches no mtime).

- [ ] **Step 1: Write failing tests** (`tests/exec.rs`, library-level via `#[path]` or a `pub` lib target — make `crates/vibe` a bin + `src/lib.rs` re-exporting modules so integration tests import `vibe::exec` directly):
  - `files_are_all_or_nothing`: three `FileWrite`s where the third's parent is a regular file → first two restored byte-identical, `failed` has one line.
  - `link_failure_undoes_links_then_files` (`#[cfg(unix)]`): FileWrite ok, LinkCreate ok, LinkCreate fails (parent is a file) → first link removed, file restored, `failed` non-empty, state unchanged.
  - `link_remove_only_if_still_link_to_target`: path replaced by a real dir → not removed; replaced by a link to elsewhere → not removed; each yields `Note::Kept`.
  - `link_create_refuses_real_dir_and_foreign_link`: note `Skipped`, nothing changed.
  - `cli_items_run_last_and_record_success_only` (`#[cfg(unix)]`, fake `claude` script appending argv to a log and exiting with `$FAKE_EXIT` for one name): argv lines exactly `mcp add-json --scope user alpha {"type":"stdio","command":"x"}`; failing name not in state; others are; exit status of the outcome reflects failure; files/links stay applied.
  - `dry_run_changes_nothing`: file tree snapshot before == after, outcome lists `would write …`.
  - `state_roundtrip_and_corrupt`: save → load equal; `{"v":2}` → Err; invalid JSON → Err; missing → default; unknown extra fields tolerated and dropped.
  - `lock_busy_returns_none` (hold a `Lock`, call `with_lock`).
  - `state_saved_after_cli_stage`: when a CLI item fails the state still records the successful edits and `store_hash` is NOT stamped (next sync retries); when stage (a) fails no state file is written.
  - `file_remove_restored_on_link_failure`: a `FileRemove` followed by a failing `LinkCreate` → removed file back, byte-identical.
  - `link_create_makes_parent_and_rollback_removes_created_parent`.
  - `unchanged_run_does_not_touch_state_mtime` (state saved only when `to_value` differs; assert `state_roundtrip_and_corrupt` too).
  - `block_item_*`: `creates_file_with_target_hash`, `idempotent_returns_no_item`, `symlink_refused`, `invalid_utf8_refused`, `hand_edit_note`, `crlf_file_idempotent`, `remove_when_body_none`; `json_file_item_*`: `unchanged_value_no_item`, `invalid_json_refused`, `missing_file_created`, `keeps_foreign_keys`.
  - `hooks_json` tests: `add_hook_idempotent`, `remove_hook_keeps_foreign_hooks_in_shared_group`, `remove_hook_prunes_empty_containers_only`, `hook_command_rejects_quote_in_path`.
- [ ] **Step 2:** `cargo test -p vibe --test exec` → FAIL.
- [ ] **Step 3: Implement.** `lib.rs` exists from Task 4; keep `main.rs` thin. Adapters reach files only through `guard_target`, `block_item` and `json_file_item`; the executor trusts the plan.
- [ ] **Step 4:** `cargo test -p vibe` → PASS.
- [ ] **Step 5: Gate (vibe).**

---

### Task 8: Codex and Factory adapters

**Files:** `crates/vibe/src/adapters/{codex,factory}.rs`, `tests/adapter_codex.rs`, `tests/adapter_factory.rs`.

**Interfaces:**
- Consumes: Task 6 translators, Task 7 `Projection`/`Ctx`/`block_item`/`json_file_item`/`hooks_json`, `markers`, `json`.
- Produces:
```rust
// hooks_json (Task 7) and block_item / json_file_item (Task 7) are consumed; nothing new is exported except:
pub fn project(ctx: &Ctx, h: Harness /* Codex | Droid */) -> Projection;       // in codex.rs / factory.rs
pub fn plan_hook_install(ctx: &Ctx, h: Harness) -> Projection;                 // `install` only: hooks file edit
pub fn plan_uninstall(ctx: &Ctx, h: Harness) -> Projection;
```
Codex (§4): instructions: `<codex_dir>/AGENTS.md` managed block (`Style::Html`, owner `store`) holding the store `AGENTS.md` text; no block when `instructions` is `None`; note `Info("codex: AGENTS.override.md exists and shadows AGENTS.md")` when `<codex_dir>/AGENTS.override.md` exists; note `Info("codex: instructions exceed 32 KiB (<n> bytes)")` when the block body > 32768 bytes (still written). MCP: block (`Style::Hash`, owner `store`) at the end of `<codex_dir>/config.toml`: servers = store servers for codex that translate OK, minus names found by `scan_mcp_tables` in the text OUTSIDE the existing block (→ `Kept("codex's own <name>")`) and minus `Skip`s (→ `Skipped("codex: <name> (<reason>)")`). Block placement/refusal: after the end marker only blank lines, `#` comments and lines starting with `[` are allowed, else `Refused("config.toml has content after the vibe block — fix by hand")`; any marker error → `Refused` with the markers message; file untouched. No servers and no block → no item; block present and no servers → block removed. Hook: `<codex_dir>/hooks.json`. Mode `Hook`: instructions + config only (MCP file items count as file items) — Codex MCP is file-based so the hook projects it too.
Factory: instructions `<factory_dir>/AGENTS.md` block; MCP: owned keys merged into `<factory_dir>/mcp.json` under `mcpServers` (translated via `factory()`); an existing key not in `state.owned.mcp_names["droid"]` is `Kept("droid's own <name>")`; previously owned names no longer projected (removed from store, disabled, or untranslatable) are deleted from the file; if a previously owned key's value was hand-edited we still overwrite (owned = by name, §4). Hook: `<factory_dir>/hooks.json`.
Both go through `block_item` / `json_file_item` (JSON re-serialised only when the value changed; invalid JSON → `Refused` for that target; missing file created with only vibe content and recorded `Created`). Target keys: `codex:instructions`, `codex:mcp`, `droid:instructions`, `droid:mcp` (hash of the block body, or of the compact JSON of owned keys). `plan_hook_install` removes `state.owned.hooks[h]` when it differs from the new command, then adds the new command (a moved binary is repaired). The Codex adapter alone emits the trust note `Info("codex: trust the new hook with /hooks")`, on hook install only.

- [ ] **Step 1: Write failing tests.** Fixtures: `Home::real_shaped` (defined in Task 4) already holds (Codex) `config.toml` holding `[projects."/x"]\ntrust_level = "trusted"` and `[mcp_servers.github]` (user-owned), `hooks.json` holding an Orca group `{"matcher":"","hooks":[{"type":"command","command":"/orca/hook.sh"}]}`; (Factory) `mcp.json` with user `mesh`, `hooks.json` with a Superset group, no `AGENTS.md`.
  - Codex: `codex_instructions_block_created_and_idempotent` (second `project` → zero items); `codex_instructions_keep_user_text_outside_block` (bytes before/after identical); `codex_hand_edit_inside_block_is_reported_and_replaced` (note says `edited inside the block`; replaced); `codex_no_instructions_no_block`; `codex_override_note`; `codex_32kib_note`; `codex_reversed_markers_refuse_and_file_untouched`; `codex_toml_block_appended_at_end_after_user_tables`; `codex_duplicate_table_is_skipped_not_duplicated` (user `[mcp_servers.github]` + store `github` → `Kept`, emitted block lacks it, final file passes `scan_mcp_tables` with `github` exactly once outside the block); `codex_content_after_block_refuses` (a bare `key = 1` after end marker) while table header after the block is accepted; `codex_translation_skips_reported` (placeholder in url → `Skipped` with reason); `codex_symlinked_config_refused` (`#[cfg(unix)]`); `codex_invalid_utf8_refused`; `codex_hooks_json_keeps_orca_group_and_adds_one_entry`, `codex_hook_second_run_no_item`; `hook_binary_moved_install_replaces_entry` (state records old command; new command installed, old group gone, Orca group kept); `codex_trust_notice` (install projection holds exactly one note containing `/hooks`; `project` holds none).
  - Factory: `factory_mcp_merge_keeps_mesh_and_adds_owned`; `factory_key_order_preserved` (byte-compare unchanged keys' text); `factory_same_name_unowned_is_kept`; `factory_owned_key_removed_when_store_drops_it`; `factory_disabled_server_gets_disabled_true`; `factory_invalid_json_refused_other_targets_unaffected`; `factory_missing_file_created_with_only_mcp_servers` (`{"mcpServers": {…}}` exact text); `factory_instructions_block_when_agents_file_absent` (file created with only the block); `factory_hooks_merge_keeps_superset_group`.
- [ ] **Step 2:** `cargo test -p vibe --test adapter_codex --test adapter_factory` → FAIL.
- [ ] **Step 3: Implement** both adapters.
- [ ] **Step 4:** the two test files + `cargo test -p vibe` → PASS.
- [ ] **Step 5: Gate (vibe).**

---

### Task 9: Claude and OpenCode adapters

**Files:** `crates/vibe/src/adapters/{claude,opencode,skills}.rs`, `crates/vibe/src/templates/opencode-plugin.js`, `tests/adapter_claude.rs`, `tests/adapter_opencode.rs`, `tests/opencode_plugin.test.mjs`.

**Interfaces:** same functions as Task 8 for `Harness::Claude` and `Harness::Opencode` (Claude uses `block_item`, `json_file_item`, `hooks_json` from Task 7; target key `claude:instructions`; `plan_hook_install` repairs a moved binary as in Task 8); plus
```rust
// skills.rs — shared "links into a skills dir" adapter
pub fn project(ctx: &Ctx, dirs: &[PathBuf /* ~/.agents/skills, claude_dir/skills */]) -> Projection;
pub const PLUGIN_MARKER: &str = "// vibe:store-plugin v1";
pub const PLUGIN_TEMPLATE: &str = include_str!("../templates/opencode-plugin.js");
```
Claude (§4, §8.3): instructions: `<claude_dir>/CLAUDE.md` managed block (`Html`, `store`) whose body is exactly the single line `@<abs store>/AGENTS.md` (+ `\n`); no block when `instructions` is `None` or the store `AGENTS.md` path is not valid UTF-8; Windows path uses `/` separators in the import line. A `CLAUDE.md` that is a symlink → `Refused`. Hook: `<claude_dir>/settings.json` via `hooks_json`. MCP (Mode `Sync` only), computed with `.claude.json` read-only (`<claude_json>`, top-level `mcpServers`; missing/invalid file → treat as empty plus `Info`):
  - owned (in `state.owned.mcp_claude`) + current entry `equal_ignoring_order` to the record + store canonical differs → `Remove` then `Add`, edit records the new canonical;
  - owned, entry equals record, store canonical equals record → nothing;
  - owned but entry differs from record → `Kept("claude: <name> edited outside vibe — kept")`, no item, still owned;
  - owned but absent from the file → `Add` again + `Info("claude: re-added <name>")`;
  - owned but no longer projected (dropped/disabled/untranslatable/filtered) and entry equals record → `Remove`; if it differs → `Kept("… edited outside vibe — kept")` and ownership dropped;
  - not owned, same name present → `Kept("claude's own <name>")`;
  - not owned, absent → `Add`.
  Skips from `translate::claude` become `Skipped("claude: <name> (<reason>)")`. CLI JSON = `json::to_compact(canonical)`. Missing `claude` binary: the executor reports it; the adapter emits nothing special.
Skills (§8.8): for each store `Skill` and each target dir: link `<dir>/<name>` → `skill.dir` (absolute). Existing entry that is a link to the same target → nothing; a link to another target, or a real dir/file → `Skipped("skills: <dir>/<name> exists, not a vibe link")`; previously owned link (in `state.owned.links`) whose skill vanished → `LinkRemove`. Targets: Claude dir `<claude_dir>/skills`; shared dir `~/.agents/skills` (Codex, OpenCode, Factory read it natively) — planned once by the dispatcher (`adapters::project`) for the first enabled harness; `skills::project(ctx, dirs)` is called with `[~/.agents/skills]` there and with `[<claude_dir>/skills]` additionally when Claude is enabled. Shadowed skills (`store.shadowed`) → `Info("skill <name>: kept <kept>, shadowed <dropped>")`. Parent dir missing → `OwnedEdit::Created{path}` for each parent absent at plan time (`create_link` creates them). Windows: `LinkCreate` returns `Err` → reported as failure line `skills: links unsupported on this OS` once, not per skill.
OpenCode: `plan_*` writes the plugin file `<opencode_dir>/plugins/vibe-store.js` = template with the first line `PLUGIN_MARKER`; rewrite when bytes differ; an existing file whose first line is not `PLUGIN_MARKER` → `Refused("opencode plugin: <path> exists and is not vibe's — not overwritten")`. The plugin reads store files at runtime; the adapter writes nothing else. Hook mode projects the plugin file only (cheap), Sync/install the same.
`opencode-plugin.js` contract (plain ES module, no imports, API from `@opencode/plugin` 2.0.29):
```js
// vibe:store-plugin v1
export default {
  id: "vibe-store",
  setup(ctx) { /* see tests */ },
};
```
Behaviour: resolve store dir (`process.env.VIBE_STORE` || `$XDG_CONFIG_HOME/vibe` || `~/.config/vibe`; `process.env.HOME`/`USERPROFILE`); `ctx.mcp.transform(editor => …)`: read `<store>/mcp.json` (missing/invalid → no-op), for each enabled server whose `harnesses` is absent or includes `"opencode"`: skip if `editor.get(name)` exists; resolve `${NAME}` from `process.env` in command/args/env/url/headers values, skip the server if any variable is unset; `editor.set(name, {type:"local", command:[cmd,...args], environment?, disabled?} | {type:"remote", url, headers?, disabled?})`; `ctx.session.hook("context", e => e.system.push({type:"text", text}))` with the store `AGENTS.md` text when non-empty (read per event so edits are live); spawn `vibe hook session-start --harness opencode` detached (`child_process.spawn(<vibe path>, [...], {detached:true, stdio:"ignore"}).unref()`, errors swallowed) where the vibe path is injected as a constant on the line `const VIBE_BIN = "__VIBE_BIN__";` replaced by the adapter with the JSON-escaped absolute path of the installing binary (a plugin without a replaced constant skips the spawn). Because the installed bytes contain the binary path, `bytes differ` also fires when the binary moves.

- [ ] **Step 1: Write failing tests.**
  - Claude instructions: `claude_block_is_single_import_line` (body equals `@/h/.config/vibe/AGENTS.md\n`); `claude_symlinked_claude_md_refused` (`#[cfg(unix)]`; real-shaped `~/.claude/AGENTS.md → opencode/AGENTS.md` is NOT touched either — assert `~/.claude/AGENTS.md` bytes and link target unchanged after a full `project`); `claude_md_user_text_survives`; `claude_hook_added_once_and_third_party_group_survives`; `claude_settings_invalid_json_refused`.
  - Claude MCP, one test per bullet above (table-driven with a fake `.claude.json`): `claude_mcp_add_new`, `claude_mcp_unowned_same_name_kept`, `claude_mcp_changed_store_remove_then_add` (items order exactly `[Remove, Add]`), `claude_mcp_edited_outside_kept`, `claude_mcp_absent_owned_readded`, `claude_mcp_dropped_removed_when_equal`, `claude_mcp_dropped_kept_when_edited`, `claude_mcp_equal_ignores_key_order`, `claude_mcp_hook_mode_emits_no_cli_items`, `claude_mcp_missing_claude_json_is_empty`, `claude_mcp_bad_name_skipped`, `claude_mcp_disabled_not_added`, `claude_mcp_placeholders_pass_through` (argv JSON contains the literal `${EXA_KEY}`).
  - Skills: `skills_link_created_for_both_dirs`; `skills_real_dir_collision_skipped` (real `~/.claude/skills/foo` dir and a foreign symlink `bar` as on the owner's machine → both untouched, two `Skipped` notes, other skills still linked); `skills_idempotent_second_run_empty`; `skills_stale_owned_link_removed`; `skills_shadowed_reported`; `skills_unix_only_failure_message_once` (`#[cfg(not(unix))]`).
  - OpenCode Rust: `opencode_plugin_written_with_marker_and_bin`; `opencode_plugin_foreign_file_refused`; `opencode_plugin_rewritten_when_template_differs`; `opencode_plugin_idempotent`.
  - `tests/opencode_plugin.test.mjs` (Node 18, no deps, `node:test` + `node:assert`): imports the generated file (the Rust test `opencode_plugin_node_test` writes `__VIBE_BIN__`-replaced plugin to a temp dir and runs `node --test` on the mjs with `VIBE_PLUGIN=<path>`; skipped with a printed reason when `node` is absent unless `VIBE_REQUIRE_NODE=1`). Fake `ctx` records `mcp.transform` and `session.hook` callbacks. Cases: `registers_servers_set_if_absent` (existing `exa` untouched, new `serena` set to `{type:"local", command:["uvx","serena"], environment:{…}}`), `remote_server_shape`, `unset_variable_skips_server`, `resolved_variables_in_headers_and_env`, `disabled_maps_to_disabled_true`, `harness_allow_list_respected`, `context_hook_pushes_agents_text_per_event` (edit file between two events → second event sees new text), `missing_or_invalid_mcp_json_is_noop`, `missing_agents_file_pushes_nothing`, `spawn_skipped_without_bin_constant`, `module_default_export_has_id_and_setup`.
- [ ] **Step 2:** the three Rust test files → FAIL; node test → FAIL (no plugin).
- [ ] **Step 3: Implement** adapters, `skills.rs`, and the plugin template.
- [ ] **Step 4:** `cargo test -p vibe` → PASS including `opencode_plugin_node_test`.
- [ ] **Step 5: Gate (vibe).**

---

### Task 10: Commands end-to-end

**Files:** `crates/vibe/src/cmd/{sync,hook,install,uninstall,status,doctor,spec}.rs`, `src/main.rs` (wire), `tests/{cmd_sync,cmd_hook,cmd_install,cmd_uninstall,cmd_status,cmd_spec}.rs`.

**Interfaces:**
- Consumes: everything above.
- Produces (library level, each returns `u8` exit code and writes through a `&mut dyn Write` pair so tests can also assert output):
```rust
pub fn sync(paths: &Paths, harnesses: Option<Vec<Harness>>, dry_run: bool, json: bool, claude_bin: &str, out: &mut dyn Write, err: &mut dyn Write) -> u8;
pub fn session_start(paths: &Paths, _harness: Option<Harness>, out: &mut dyn Write, err: &mut dyn Write) -> u8;  // always 0; `--harness` is informational
pub fn install(paths: &Paths, opts: InstallOpts /* harnesses, toolkit, dry_run */, claude_bin: &str, out, err) -> u8;
pub fn uninstall(paths: &Paths, dry_run: bool, claude_bin: &str, out, err) -> u8;
pub fn status(paths: &Paths, json: bool, out, err) -> u8;
pub fn doctor(paths: &Paths, out, err) -> u8;
pub fn spec(args: &[OsString], env: &Env) -> u8;   // exec $VIBE_SPEC_BIN, else `vibe-spec` on PATH; missing → stdout "vibe: vibe-spec is not installed — build it with `cargo build -p vibe-spec` or install the vibe-spec binary" exit 0; child's exit code passes through
```
Output grammar (one line per event, stable, asserted in tests): `wrote <show>`, `linked <path>`, `unlinked <path>`, `claude mcp add <name>` / `claude mcp remove <name>`, `kept <harness>'s own <name>`, `skip <harness>: <name> (<reason>)`, `refused <text>` (`<text>` = the full `Note::Refused` string), `note <text>`; dry run prefixes `would ` and writes nothing. `sync --json` prints `{"v":1,"dry_run":b,"changes":[strings],"notes":[strings],"failed":[strings]}` via `json::to_pretty`. Exit: 1 when `failed` or any `refused` is non-empty, else 0.
Behaviour pins:
- `sync`: lock → load state → load store → `project(Mode::Sync)` per enabled harness (+ skills) → `exec::apply` → stamp `store_hash`, `mcp_hash`, per-target hashes only when nothing failed. Detects external drift: a recorded target hash that differs from the current block hash yields `note <harness>: <show> edited inside the vibe block — replaced`. Lock busy → `note another vibe run holds the lock` exit 1 (sync is explicit; only the hook is silent).
- `hook session-start`: never panics (wrap in `std::panic::catch_unwind`), every error path: at most one stderr line `vibe hook: <cause> — skipped`, exit 0, empty stdout. Steps: build `Env::from_process` (unset `HOME` → error path); try lock → `Busy` silent; load state (corrupt → skip with the stderr line); load store; compute `store_hash`; equal to the stamp → silent exit 0 (no projection, no state write). Else `project(Mode::Hook)` for all enabled harnesses → apply (files + links only) → if all succeeded stamp `store_hash` (leave `mcp_hash`); print on stdout exactly one line only when at least one item applied or Claude MCP is out of date: `vibe: store changed — refreshed <h1, h2>; run \`vibe sync\` for Claude MCP` (the clause after `;` only when `mcp_hash` differs from the current `mcp.json` hash and `claude` is enabled); when no item applied but Claude MCP is out of date print `vibe: Claude MCP changed; run \`vibe sync\``; else nothing.
- `install`: take lock; map `store::init_files` to `FileWrite` items plus `Created`; locate toolkit (`--toolkit <dir>`, else `$VIBE_TOOLKIT`, else walk up ≤4 ancestors of `current_exe()` testing `<a>`, `<a>/skills`, `<a>/.agents/skills` for child dirs `spec`, `vibe`, `using-spec` each holding `SKILL.md`); when found and not already listed, append it to `store.json` `skills.sources` (a `json_file_item` edit in the same plan) and patch `store.config.skill_sources` in memory, re-running `skills::discover`, before projecting; the final `store_hash` is computed by reloading the store after apply; build one plan = store init writes + per selected harness (`--harness` or all four via `enabled_harnesses(.., require_dir = false)`; created dirs recorded `Created`) `plan_hook_install` + opencode plugin + the full sync projection (Mode::Sync) → apply once. Notes are printed unchanged (the `/hooks` trust note comes from the Codex adapter only). Second run → zero items (`nothing to do`), exit 0, no file mtime changes.
- `uninstall`: lock; load state (missing → `note nothing installed`, exit 0); build removal plan from the state only: marker blocks removed (`markers::remove`; file deleted when it was `Created` and is now empty/`{}`), hook groups with the recorded command removed, plugin removed when its first line is `PLUGIN_MARKER`, links removed when still a link to the recorded target, Factory owned keys removed, Codex block removed, Claude MCP removed via CLI only when the live entry equals the record (else `kept … edited outside vibe`), finally `state.json` deleted; the store dir is never touched.
- `status [--json]`: store validity (`store ok` / refusal), per harness: enabled, block present/intact, hook present, drift (`ok`|`drifted`|`absent`), skips and conflicts counted; exit 1 only when the store is invalid. JSON `{"v":1,"store":{"valid":b,…},"harnesses":{…}}`.
- `doctor`: for `claude`, `codex`, `opencode`, `droid`, `vibe-spec` look the binary up on `PATH` (print `found <path>` / `missing <name>`); store parse; state parse; lock (`free`/`held by <pid>`/`stale`); each recorded hook command whose binary path no longer exists → `problem <harness>: hook binary missing — run \`vibe install\``; exit 1 when any `problem` line.

- [ ] **Step 1: Write failing tests** (all via the real binary with `Home::real_shaped`, plus `fake_claude(&Home) -> PathBuf` placed first on `PATH` for the CLI):
  - Install/sync (plus `hook_binary_moved_install_replaces_entry` end to end: move the binary, re-run `install`, one entry remains): `install_fresh_home_wires_all_four` (assert each of: store skeleton; `CLAUDE.md` block; `settings.json` hook; Codex `AGENTS.md` block + `config.toml` block + `hooks.json`; Factory `AGENTS.md` + `mcp.json` + `hooks.json`; OpenCode plugin; skill links; fake-claude argv log has the `add-json` lines); `install_second_run_changes_nothing` (snapshot of every file's bytes AND mtime under HOME before/after, `nothing to do`); `install_real_shape_preserves_everything` (symlink `~/.claude/AGENTS.md` target + bytes, foreign skill symlinks, real skill dirs, Orca/Superset groups, user MCP in all four, agent-stack file untouched; each compared byte-for-byte or link-target); `install_dry_run_writes_nothing`; `install_refused_target_does_not_block_others` (invalid `~/.factory/mcp.json` → exit 1, other harnesses done); `install_finds_toolkit_via_flag_and_env` and adds it as the last source once; `install_no_literal_secrets_written`: fixture store whose `mcp.json` holds `${VIBE_X}` placeholders and env set to a sentinel `SECRET-SENTINEL-123` → grep every file under HOME for the sentinel → none.
  - `sync_after_store_edit_updates_files_and_claude_cli` (edit `mcp.json` → exactly `remove` then `add-json` for the changed name); `sync_json_shape`; `sync_reports_edit_inside_block`; `sync_lock_busy_exits_1`; `sync_filter_harness_only_touches_it`.
  - Hook: `hook_unchanged_store_is_silent_and_writes_nothing`; `hook_after_store_edit_reprojects_files_and_prints_one_notice` (stdout exactly one line; claude MCP clause present only when `mcp.json` changed; no `claude` invocation — fake-claude log empty); `hook_fail_open_cases` (unset HOME, corrupt `state.json`, missing store, read-only target dir `#[cfg(unix)]`, held lock → each exit 0, stdout empty, stderr ≤ 1 line, no `panicked`); `hook_second_concurrent_run_silent` (hold lock via core `Lock`); `hook_after_crash_stale_lock_recovered` (lock file mtime 2 min old); `hook_does_not_spawn_subprocesses` (PATH empty dir: still exit 0, files projected).
  - Uninstall: `uninstall_restores_pre_install_bytes` (`real_shaped` fixture, whose text files end in a newline and JSON is canonical 2-space: every pre-existing file byte-identical to the pre-install snapshot, files vibe created gone, store dir intact, `state.json` gone; the no-trailing-newline limit is documented, not tested here); `uninstall_keeps_edited_claude_mcp`; `uninstall_keeps_replaced_links`; `uninstall_without_state_is_noop`; `uninstall_dry_run_changes_nothing`.
  - `status_reports_drift_and_exits_1_on_invalid_store`; `doctor_flags_missing_hook_binary` (move the binary path in state); `doctor_lists_found_and_missing_clis`.
  - `spec_dispatch_runs_vibe_spec_bin_and_passes_exit_code` (`VIBE_SPEC_BIN` = tiny fake script `exit 7` → 7, `#[cfg(unix)]`); `spec_missing_prints_one_advice_line_exit_0`.
  - Planted-defect checks (record each failure output before reverting): (1) make the executor skip the link-undo → `link_failure_undoes_links_then_files` fails; (2) make `markers::find` accept indented markers → look-alike test fails; (3) make the hook stamp `store_hash` before applying → `hook_after_store_edit…` retries test fails.
- [ ] **Step 2:** `cargo test -p vibe` → FAIL for the new tests.
- [ ] **Step 3: Implement** commands and wiring. Hook command string inside `settings.json` etc. uses `std::env::current_exe()` canonicalised.
- [ ] **Step 4:** `cargo test -p vibe` → PASS.
- [ ] **Step 5: Gate (vibe, vibe-core).** Also `cargo test --workspace --locked` once.

---

### Task 11: Contracts for store, mcp and state

**Files:** Create `contracts/vibe-store.schema.json`, `contracts/fixtures/vibe-store/{store,mcp,state,sync}/{valid,invalid}/*.json`; Modify `contracts/run.mjs`, `contracts/tests/run.test.mjs`; Create `crates/vibe/tests/contract_state.rs`.

**Interfaces:** schema with `$defs` `store`, `mcp`, `state`, `sync`, `x-vibe-compat: "additive-within-major; readers-ignore-unknown-fields"` (the string `run.mjs` requires), each def `required: ["v"]` with `properties.v.const = 1` except `mcp` (the `{"mcpServers": …}` file has no `v`; its required set is `["mcpServers"]`). Required sets pinned: `store: ["v"]`; `state: ["owned","store_hash","targets","v"]`; `sync: ["changes","dry_run","failed","notes","v"]`. Use only keywords supported by `contracts/lib/schema.mjs` (read it first).

- [ ] **Step 1: Write failing contract changes.** In `run.mjs` add a `STORE_DEFS = ['store','mcp','state','sync']` loop mirroring the spec-json loop (floors: 2 valid incl. `unknown-field.json`, 2 invalid per def) and pin `REQUIRED_STORE` sets as above; update the `FLOOR.schemas` expectation test. In `contracts/tests/run.test.mjs` add discriminating cases: a copy of the contracts dir with `vibe-store.schema.json` missing `x-vibe-compat` fails; a copy with an `invalid/` golden that validates fails; a copy with `valid/unknown-field.json` holding no unknown field fails. The validator subset has no `additionalProperties` schema or `patternProperties`, so dynamic-key maps (`targets.<key>`, `mcpServers.<name>`) are typed only as `object`. Fixtures: valid — minimal; full (as written by `install` on the real-shaped home; `targets` keys are free strings such as `codex:mcp`); `unknown-field.json`; invalid goldens use top-level violations only — `v:2`, `store_hash` failing `pattern ^[0-9a-f]{16}$`, missing `store_hash`, `targets` as an array, `mcpServers` as an array, `sync.changes` not an array.
- [ ] **Step 2:** `bash contracts/run.sh` → FAIL (schema missing).
- [ ] **Step 3: Write the schema and fixtures.** `contracts/vibe-store.schema.json` describes exactly the shapes of Task 5 (`store.json`, `mcp.json`) and Task 7 (`state.json`: `v`, `store_hash`, `mcp_hash`, `targets`, `owned.{mcp,hooks,plugins,links,created}`), and Task 10 (`sync --json`).
- [ ] **Step 4:** `bash contracts/run.sh` → PASS.
- [ ] **Step 5 (after Task 10): emitted-shape test.** `crates/vibe/tests/contract_state.rs`: run `vibe install` on `Home::real_shaped`, parse the produced `state.json` and `vibe sync --json` output with `vibe_core::json`, and assert the pinned shape in Rust: `v == 1`, keys of `owned` ⊆ {`mcp`,`hooks`,`plugins`,`links`,`created`}, every `targets.<id>.hash` is 16 lowercase hex, `hooks.claude` starts with `"` and contains `hook session-start --harness claude`, sync JSON has exactly the five required keys. Plus one test that copies `contracts/fixtures/vibe-store/state/valid/full.json` to `Paths::state_file()` under a temp `Paths` and calls `vibe::state::load(&paths)` (the contract fixture and the reader agree).
- [ ] **Step 6: Gate (vibe)** and `bash contracts/run.sh`.

---

### Task 12: Real-CLI acceptance (`VIBE_E2E=1`)

**Files:** `crates/vibe/tests/e2e.rs`.

Each test starts with `if std::env::var_os("VIBE_E2E").is_none() { eprintln!("skipped: set VIBE_E2E=1"); return; }` and a `which`-style PATH lookup that skips (with a printed reason) when the CLI is missing, unless `VIBE_E2E_REQUIRE=1` (then it fails). Scenario: `Home::real_shaped`, a store with one stdio server (`echo-srv`, command `node`, args `["-e","0"]`), one env-placeholder server, one http server with `Authorization: "Bearer ${VIBE_HTTP_TOKEN}"`; `vibe install`; then run the real CLI under the temp `HOME` with `env_clear()` + `PATH`.

- [ ] **Step 1: Write the tests.**
  - `e2e_codex_accepts_generated_config`: `CODEX_HOME=<home>/.codex codex mcp list --json` exit 0, JSON contains `echo-srv` and the user's own `github`; `codex mcp get echo-srv --json` shows `command == "node"`; the http entry shows `bearer_token_env_var == "VIBE_HTTP_TOKEN"`.
  - `e2e_codex_rejects_duplicate_table_control`: append a duplicate `[mcp_servers.echo-srv]` outside the block by hand → real `codex mcp list` FAILS (proves the scan guards a real failure; skip assertion text, only non-zero exit).
  - `e2e_droid_lists_generated_mcp`: `droid mcp list` exit 0, output contains `echo-srv` and `mesh`.
  - `e2e_claude_user_scope_has_servers`: real `claude` (not the fake) on PATH; `vibe sync`; `claude mcp get echo-srv` exit 0 and shows `node`; `.claude.json` placeholders stored literally (`${VIBE_…}` text present); `claude mcp get` for a server owned by the user (pre-seeded via `claude mcp add-json`) is unchanged after `sync`; `vibe uninstall` → `claude mcp get echo-srv` non-zero while the user's server remains.
  - `e2e_claude_mcp_list_works_with_block`: with the vibe block present in `CLAUDE.md`, `claude mcp list` still exits 0 (the `@import` itself is not testable offline).
  - `e2e_opencode_loads_plugin` (optional, runs only with `VIBE_E2E_OPENCODE=1`): `opencode debug config` under temp `HOME` + `XDG_CONFIG_HOME` exit 0 and mentions `echo-srv`; documented fragile (needs a running service).
- [ ] **Step 2:** `VIBE_E2E=1 cargo test -p vibe --test e2e -- --nocapture` on the owner's machine. Expected: PASS for each CLI found; record outputs. Any mismatch between a CLI's real behaviour and the plan's pinned facts is a design finding → fix in the owning task and add a lesson (Task 13).
- [ ] **Step 3:** without `VIBE_E2E`: `cargo test -p vibe --locked` shows the e2e tests as skipped (printed reasons), exit 0. Windows legs skip via `#[cfg(unix)]`.
- [ ] **Step 4: Gate (vibe).**

---

### Task 13: CI fit, docs and compound

**Files:** `.github/workflows/ci.yml` (verify only; edit if needed), `docs/spec/decisions/d48-global-store-and-projection.md`, `docs/spec/decisions/README.md`, `docs/spec/tech/cli/cli-store.md` (≤ 60 lines), `docs/spec/tech/cli/cli-binary.md` (hook-ownership sentence: replace "carries the front-door command surface only — no hooks of its own" with the D48 amendment), `docs/spec/parts/cli/concepts/cli-feature-sequence.md` (row C0, `updated:` bumped), `docs/spec/plan.md` (v0.5 arc mention), `docs/spec/lessons.md`, root `AGENTS.md` (dogfood table row for `crates/vibe` and the active-rules digest), `README.md`, `CHANGELOG.md`. Check each path first: `bash .agents/skills/vibe/scripts/detect-context.sh decide <path>`. Root specs, lessons and `AGENTS.md` are writable only in compound states; when `decide` says block/warn, run `bash .agents/skills/vibe/scripts/set-state.sh feature.compound global-store` (only after the owner's verify→ship gate allows it), then write, never edit `state.json` by hand.

- [ ] **Step 1: CI fit.** Confirm `rust-check` and `rust-test` already cover the new crate (`--workspace`), cargo-deny passes on the lock (run `cargo deny check` if available), and the contracts leg runs `contracts/run.sh` (Task 11). Edit `.github/workflows/ci.yml`: add an `actions/setup-node@v4` step (`node-version: '20.19'`, same as the other jobs) before `Test` in the `rust-test` matrix job, so `opencode_plugin_node_test` runs on all three OSes. This is the only CI edit.
- [ ] **Step 2: D48.** One decision note matching the format of `d47-router-tooth.md`: front door is a fourth, user-level hook owner for SessionStart projection only (D25 amended); amends D40's "plugin carries no hooks" (vibe registers no plugin hook; one user-level entry per harness); state/ownership model; hook never injects instruction content; known limit: `markers::remove` returns text that lacked a trailing newline with one. Add the README index line.
- [ ] **Step 3: `cli-store.md`** (≤ 60 lines, frontmatter `type: tech`, `applies: ["crates/vibe/**"]`, `updated: 2026-10-24`): store layout, `store.json`/`mcp.json` keys, projection table (compressed), commands, state shape (incl. `mcp_hash`), marker grammar, ownership rules, links to the design and D48.
- [ ] **Step 4: Plan row C0** `global-store` (Deliverable: `crates/vibe` store/projection/install; Test: idempotent install, real-shaped preserve, fail-open hook, E2E gated; Status DONE; Starts when: —) and the v0.5 summary line; README section "Global store" (how to install/sync/uninstall, store layout, secrets rule, env vars `VIBE_STORE`, `VIBE_STATE`, `VIBE_TOOLKIT`, `VIBE_SPEC_BIN`); CHANGELOG entry under the next version heading.
- [ ] **Step 5: Spec amendments and lessons.** Amend the design spec §5 state shape (`mcp_hash`, `targets` keyed `claude:instructions`, `codex:instructions`, `codex:mcp`, `droid:instructions`, `droid:mcp`) and bump its `updated:` in this compound cycle (check `decide` first). Then move every lesson from `.superpowers/sdd/2026-10-24-global-store/progress.md` into `docs/spec/lessons.md`, and update the `AGENTS.md` active-rules digest in the same cycle (no edits inside `<!-- vibe:active-rules:* -->` by hand: run the regen script the repo documents — find it with `ls .agents/skills/vibe/scripts | grep regen`).
- [ ] **Step 6: Full verification (verification-before-completion skill).** Run and show real output: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked --no-fail-fast`, `cargo deny check` (if installed), `bash contracts/run.sh`, `bash tests/run.sh`, `cargo run -q --locked -p vibe-spec -- validate --strict`, `cargo run -q --locked -p vibe-spec -- check-drift`, and `VIBE_E2E=1 cargo test -p vibe --test e2e`.
- [ ] **Step 7: Gate (workspace).** Delta against the Task 0 baseline: no new warnings, test count up, no skipped-by-accident suite.

---

## Self-review (spec coverage)

| Spec | Task |
|---|---|
| §1 criteria 1,2 (idempotent install, session-start re-projection, notice) | T10 `install_second_run…`, `hook_after_store_edit…` |
| §1 criterion 3 (real CLIs accept files) | T12 |
| §1 criterion 4 (no literal secret) | T5 `import_replaces_secrets`, T10 `install_no_literal_secrets_written` |
| §1 criteria 5,6 (preserve, uninstall exact) | T8, T9, T10 `install_real_shape_preserves_everything`, `uninstall_restores…` |
| §1 criterion 7 (all gates) | T13 step 6 |
| §3 store files, `store.json`, `mcp.json`, secrets | T5 |
| §4 per-harness projection, ownership, markers, symlink refusal, atomic, lock, formatting | T1, T2, T7 (helpers), T8, T9 |
| §4 MCP translation table, Codex scan/placement | T6, T8 |
| §5 commands, state | T4 (surface), T5, T7, T10 |
| §6 architecture, core modules, deps | T0-T3, T4 |
| §7 verification, planted defects | T10 step 1, T4 step 5 |
| §8.1 hook ownership (docs) | T13 D48 |
| §8.2 executor order and rollback | T7 |
| §8.3 Claude MCP rules | T9 |
| §8.4 hook cache key | T5 `store_hash`, T10 hook |
| §8.5 hook entries | T8 `hooks_json`, T9 Claude, T10 doctor |
| §8.6 OpenCode plugin | T9 |
| §8.7 workspace rules | T4 |
| §8.8 skills | T5 discovery, T9 links |
| §8.9 docs | T13 |

Type consistency checked: `Projection`/`OwnedEdit` (T7) are the only adapter outputs used by T8-T10; `McpServer`/`Translated` (T5/T6) feed T8/T9; `Paths`/`Harness` (T4) are used everywhere; hash function is `markers::hash_bytes` throughout.
