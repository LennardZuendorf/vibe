# Spec v0.4 P4 — Hooks, Tooth, Agents, Migrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver F19 (router hook, blocking tooth, validate hooks, git hook, two registered agents) and F20 (`init`, `migrate`, this repo on a layered `docs/spec/` with `validate --strict` green and zero `.spec` literals).

**Architecture:** Every hook is a `vibe-spec hook <event>` subcommand. A bash shim (`spec/hooks/run-hook.sh`) wraps each one so a missing binary fails open. The tooth reads the session transcript the hook payload names. Two real payloads fix its rules: one main session and one subagent, captured from Claude Code 2.1.303 on 2026-10-19. `migrate` plans every move, rewrite and new file in memory, checks a population assertion, writes all-or-nothing with restore, and verifies the result. This repo then migrates with that command in one commit. An agent-led restructure follows, and `validate --strict` gates it.

**Tech Stack:** Rust 1.88 (`vibe-core`, `vibe-spec`; deps stay `clap`/`serde`/`serde_json`), bash 3.2+ (shim, git hook, adapters test), Markdown/JSON (skills, agents, hooks.json), Node ≥20.19 (OpenCode plugin shim, contracts).

**Spec:** `docs/superpowers/specs/2026-10-13-spec-v04-design.md` (§4, §6.2, §6.3, §7, §8 F20, §9). Issues: `docs/roadmap/issues/19-F19-spec-hooks-agents.md`, `docs/roadmap/issues/20-F20-spec-migrate.md`. Index row 4: `docs/superpowers/plans/2026-10-13-spec-v04.md`.

## Rulings (P4-R1..R20)

These settle what the spec and issues leave open. They are binding for every task.

- **P4-R1** One plan for F19 and F20 (owner, 2026-10-19).
- **P4-R2 Skill detection.** These facts come from the captured payloads in `crates/vibe-spec/tests/fixtures/hooks/`:
  - Hook stdin always carries `session_id`, `transcript_path`, `cwd`, `hook_event_name`, and `tool_name`/`tool_input` on tool events.
  - A subagent's payload adds `agent_id` and `agent_type`, and its `transcript_path` is the **main** session's transcript.
  - The subagent's own transcript is `<transcript_path without .jsonl>/subagents/agent-<agent_id>.jsonl`.

  Skill `N` counts as loaded when one of these appears in the main transcript, or in the subagent transcript when `agent_id` is present:
  - (a) an `assistant` record with a `tool_use` content item named `Skill` whose `input.skill` is `N` or ends in `:N`;
  - (b) a `user` record whose string content contains `<command-name>/N</command-name>` or `<command-name>/<ns>:N</command-name>`.

  Text anywhere else never counts: tool results, prose, quoted names. The design's Task-prompt fallback (§6.3) is dropped, because the capture shows that the parent's load is visible to its subagents.
- **P4-R3 Tooth targets and messages.** Root-relative targets:
  - spec root `**` and `.vibe/spec.json` need `spec`;
  - `docs/superpowers/specs/**` needs `superpowers:brainstorming` or `spec`;
  - `docs/superpowers/plans/**` needs `superpowers:writing-plans` or `spec`.

  A block exits 2 with exactly one stderr line, for example:
  - `blocked: invoke Skill(spec) before editing docs/spec/areas/checkout.md`
  - `blocked: invoke Skill(superpowers:brainstorming) or Skill(spec) before editing docs/superpowers/specs/x.md`

  Paths in messages are project-relative and `/`-joined. A `Bash` command that names a target together with a write operator (`>`, `>>`, `tee`, `sed -i`, `cp`, `mv`) gets one stderr warning and exit 0: `vibe-spec: warning: a shell write to <rel> bypasses the spec tooth`.
- **P4-R4 Fail open.** Each of these causes prints one stderr line, `vibe-spec hook guard: allowing — <cause>`, and exits 0:
  - malformed or empty input;
  - an unreadable transcript;
  - no spec root;
  - a target outside the project.

  If `.vibe/spec.json` has `"tooth": "warn"`, a would-be block prints `vibe-spec hook guard: warn — <block line>` and exits 0. Any other `tooth` value is a usage error: one stderr line, exit 0 (fail open). `.vibe/spec.json` is itself a `spec` target.
- **P4-R5 Validate hooks.**
  - `validate --file <path>` runs the full validation and keeps only the findings whose `file` equals the path, relative to the project.
  - `validate --changed` keeps only the findings for spec files that changed: `git status --porcelain=v1 -z --untracked-files=all` covers staged, unstaged and untracked files under the root. Without git, it uses files whose mtime is newer than `.vibe/spec-session`. With no marker, it uses every file. With none changed, it prints `validate: no spec changes` and exits 0.
  - Tree-level findings (`file == ""`) are kept by `--changed` and dropped by `--file`.
  - `--format hook` prints hook JSON; see Task 3.
  - `hook validate` dispatches on `hook_event_name`. `PostToolUse` → `--file` with a 500 ms cap. `Stop` → `--changed` with a 2 000 ms cap. Over the cap it prints `vibe-spec hook validate: skipped — over <n> ms` on stderr. It always exits 0.
- **P4-R6 Router.** `hook session-start` prints the using-spec body inside `<EXTREMELY_IMPORTANT>`. The JSON shape depends on the environment:
  - Claude (`CLAUDE_PLUGIN_ROOT` or `CLAUDE_PROJECT_DIR` set): `{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":…}}`.
  - Cursor (`CURSOR_PLUGIN_ROOT`): `{"additional_context":…}`.
  - Otherwise: `{"additionalContext":…}`.

  It touches `<project>/.vibe/spec-session` (creating `.vibe/` if absent) when a project is found, and always exits 0. The skill body is compiled in with `include_str!`.
- **P4-R7 Shim.** `spec/hooks/run-hook.sh <event>` resolves the binary in this order: `$VIBE_SPEC_BIN`, then `vibe-spec` on `PATH`, then `$CLAUDE_PROJECT_DIR/target/release/vibe-spec`, then `$CLAUDE_PROJECT_DIR/target/debug/vibe-spec`.
  - If none is found, it prints one stderr line, `vibe-spec: binary missing — spec hooks skipped (build with: cargo build -p vibe-spec)`, and exits 0.
  - It passes exit codes 0 and 2 through. Any other code prints `vibe-spec: hook <event> failed (exit <n>) — allowing` and exits 0.
- **P4-R8 Wiring.** Both places wire the same four events:
  - `plugin/hooks/hooks.json`, generated by `build-plugin.sh`, with the command `bash "${CLAUDE_PLUGIN_ROOT}/skills/spec/hooks/run-hook.sh" <event>`;
  - this repo's `.claude/settings.json`, with `bash "$CLAUDE_PROJECT_DIR/spec/hooks/run-hook.sh" <event>`.

  The four events:
  - SessionStart (matcher `startup|clear|compact`);
  - PreToolUse (matcher `Edit|Write|MultiEdit|NotebookEdit|Bash`, event `guard`);
  - PostToolUse (matcher `Edit|Write|MultiEdit`, event `validate`);
  - Stop (event `validate`).

  The spec tooth and flow's guard are independent. Both must allow.
- **P4-R9 Agents.** Agents live at `spec/agents/spec-promoter.md` (model `sonnet`) and `spec/agents/spec-health.md` (model `haiku`). They use Claude Code agent frontmatter (`name`, `description`, `tools`, `model`) and each file is ≤40 lines.
  - The promoter body contains the exact line `Write only under the directory printed by \`vibe-spec root\`.`
  - The health body contains `Read-only: never write files.`
  - Both contain `If \`vibe-spec\` is not on PATH, reply "vibe-spec binary missing" and stop.`

  Registration:
  - `plugin/agents -> ../spec/agents`, a symlink that `build-plugin.sh` maintains;
  - this repo: `.claude/agents/spec-promoter.md` and `.claude/agents/spec-health.md`, symlinks to `../../spec/agents/…`;
  - OpenCode: through `setup --opencode`.

  `spec-interviewer` and `spec-tracer` are deleted.
- **P4-R10 setup.** `vibe-spec setup [--git-hooks] [--opencode] [--agents-md] [--json]` requires at least one flag, otherwise exit 2. Every file it writes carries a marker. It refuses (exit 1, writes nothing) to replace a file without that marker, and it replaces its own files idempotently.
  - **Pre-commit hook.** It goes at `git rev-parse --git-path hooks` + `/pre-commit`, with the marker `# vibe-spec pre-commit v1`. It runs `vibe-spec validate --changed --strict` and exits with its code. A missing binary prints one stderr line and exits 0.
  - **OpenCode.** The marker is `<!-- vibe-spec:opencode v1 -->`, or `// vibe-spec:opencode v1` in JS. Files: `.opencode/agent/spec-promoter.md`, `.opencode/agent/spec-health.md`, and `.opencode/plugin/vibe-spec.js`, which is a `tool.execute.after` shim that runs `vibe-spec hook validate` with a PostToolUse-shaped payload.
  - **AGENTS.md.** The block is `<!-- vibe-spec:router:start -->` … `<!-- vibe-spec:router:end -->`, with ≤6 lines between the markers. It is appended when absent and replaced in place when present. No other text changes.
- **P4-R11 init.** `vibe-spec init [--to <dir>] [--json]` targets `--to`, else the discovered root, else `<project>/docs/spec`.
  - It seeds `README.md`, `AGENTS.md`, `product.md`, `tech.md`, `design.md`, `plan.md` and `lessons.md`.
  - The four hubs get `type: hub` and `updated: <today_utc>`. `README.md` has `type: hub` too, which makes `layered::detect` return `Layered`.
  - It never overwrites: a file that exists gets `SKIP: <rel>` instead.
  - It exits 0. Templates are compiled in.
  - `setup.sh` stays oracle-only. `init` is a deliberate redesign, not a port, and the `tests/parity/spec.json` `setup` entry keeps `candidate: null`.
- **P4-R12 migrate.** `vibe-spec migrate --to <dir> [--layout layered] [--dry-run] [--check] [--json]`.
  - **Literal.** The regex `(^|[^A-Za-z0-9_-])\.spec([^A-Za-z0-9_-]|$)`, applied per line. A line that contains `<!-- literal -->`, `// literal` or `# literal` is exempt.
  - **File set.** The default globs are `<to>/**`, `.vibe/*.json`, `content/policy.json`, `.claude/**`, `.agents/**`, `tests/**`, `*.md` and `flow/reference/templates/**`. Add `.vibe/spec.json` `migrate.include` (an array of globs), then subtract `migrate.exclude`. Only regular files count: symlinked files and directories are never followed, so each real file is scanned once. Paths are matched with `vibe_core::glob::glob_match`.
  - **Population.** `before ≥ 1` is checked only when the root must move. `scanned ≥ migrate.floor` (default 1). After the write, `after == 0`. A miss before writing exits 1. A miss after writing reverts, then exits 1.
  - **Idempotence.** If the root is already at `<to>` and the literal count is 0, it prints `migrate: already migrated` and exits 0. If the root is already at `<to>` and literals remain, it rewrites them without a move.
  - **`--check`.** It scans the configured set without writing. It exits 0 when there are 0 literals; otherwise it lists each one and exits 1. This is the zero-literal sweep.
- **P4-R13 Execute and revert.** Order of operations:
  1. Plan everything.
  2. Move the root: `git mv` when it is tracked, otherwise rename.
  3. Do the layered moves.
  4. Write every rewritten or new file through one commit helper that keeps the original bytes or records that the file was absent.
  5. Write `.vibe/spec.json` `root`.
  6. Verify.

  Any failure restores the originals, removes the files it created, reverses the moves in reverse order, prints `migrate: ERROR: <cause> — reverted`, and exits 1.
- **P4-R14 Allow-markers.** After the rewrite, `migrate` validates the new root. For each file with a warning from SF22, SF30, SF31, SF33 or SF34, it inserts `<!-- vibe-spec: allow SF-nn -->`, one marker per check id, after the frontmatter. It lists every marker it inserted. The agent-led restructure removes them.
- **P4-R15 Layered mechanics.**
  - `type: entrypoint` becomes `type: hub`.
  - `tech-<t>.md` becomes `tech/<t>.md` with `type: tech` and `applies: "**"`. Its `parent:` and `covers:`/`scope:` keys are dropped.
  - A `product-<t>.md` or `plan-<t>.md` refuses with `migrate: ERROR: <rel>: place product/plan topics by hand` and exits 1. No such file exists here.
  - D-records: each `plan.md` bullet matching `^- \*\*D(\d+) — (.+?)\.\*\*` runs to the next bullet, blank line or heading. It becomes `decisions/d<NN>-<slug>.md`, where `slug` is the kebab-case title, capped at 6 words. The frontmatter is `type: decision`, `updated:` today, and the body is the bullet text.
  - In `plan.md`, each bullet is replaced by `- [D<NN> — <title>](decisions/d<NN>-<slug>.md)`.
  - Relative markdown links inside the root are re-resolved after the moves: links in moved files and links pointing at moved files are both updated.
- **P4-R16 Content is rewritten, shipped logic is not.** `migrate` text-rewrites prose and config only. Code that ships to other users is changed by hand in Task 12, with tests, so that it recognises **both** roots. D33 keeps `.spec` as a discovery fallback, and a blind rewrite would drop legacy support.
  - **Hand-made dual-root changes (Task 12):**
    - `flow/engine/root.mjs` `MARKERS`;
    - the `flow/content/policy.json` rules, which get both `docs/spec/…` and `.spec/…` globs;
    - the `flow/scripts/detect-context.sh` arms and `flow/hooks-fallback/pre-tool-use-guard.sh`;
    - `flow/scripts/{regen-active-rules,orders,doctor}.sh`;
    - the session-start detector in the `build-plugin.sh` heredoc;
    - `install.sh`;
    - `crates/vibe-spec/src` legacy messages.

    Each kept legacy line carries the `literal` tag. JSON has no comments, so `policy.json` is exempt by path, which is recorded in the sweep script.
  - **This repo's `migrate` set** (`.vibe/spec.json` `migrate`):
    - `include`: `spec/**/*.md`, `flow/**/*.md`, `flow/reference/templates/**`, `docs/roadmap/**`, `.github/**`, `README.md`, `AGENTS.md`.
    - `exclude`: `oracles/**`, `spec/scripts/**`, `docs/superpowers/**`, `CHANGELOG.md`, `target/**`.
  - **`updated:` bump.** `migrate` sets `updated:` to today on every spec doc it moves or rewrites, so SF29 holds right after the migration. Task 11 tests this.
- **P4-R16a Zero-literal sweep (§1.2).** `tests/literals/run.sh` scans every tracked file outside `oracles/` with the P4-R12 regex. A line is exempt when it carries the line tag, or when its file holds the file-level marker `vibe-spec: literal-file` in its first 5 lines; `literal::find` honours both. Path exemptions are listed in the script, each with its reason:
  - `contracts/fixtures/**` and `tests/parity/corpus/**`: legacy-tree fixtures, and JSON cannot carry a comment;
  - `flow/content/policy.json`: JSON, holding the dual-root rules;
  - `docs/superpowers/**`: these close in P5, once promoted and deleted per D46.

  Legacy-support test files (`crates/*/tests/*.rs`) and `CHANGELOG.md` get the file-level marker. The sweep runs in `tests/run.sh` and CI. Criterion §1.2 is fully met only in P5, when `docs/superpowers/**` is gone.
- **P4-R17 Callers.** The bash spec scripts hard-code `.spec`, so the migration commit switches this repo's callers to `vibe-spec`:
  - CI `validate`/`check-drift` steps;
  - `tests/run.sh` live checks;
  - `spec/SKILL.md` and `spec/README.md` commands;
  - `AGENTS.md` commands, via the template.

  The parity **live** leg is retired at migration, because oracle bash cannot read `docs/spec`. Corpus legs stay. **This changes the P3a exit gate** ("live on this repo"). From the migration on, this repo's tree is checked only by `vibe-spec` itself. Surface this at plan review.
- **P4-R17a SF29 under squash merges (owner decision, asked at plan review).** SF29 errors when a doc's `updated:` is older than its last commit. After a squash merge, the merge date becomes every touched doc's last-commit date, so `validate --strict` on `main` would fail. The plan assumes option (a) unless the owner picks otherwise:
  - (a) This repo's `.vibe/spec.json` sets SF29 `off` for CI, and the pre-commit hook (`validate --changed --strict` with SF29 on) keeps freshness at authoring time.
  - (b) Keep SF29 under strict and bump `updated:` in the squash commit.
  - (c) SF29 compares against the last commit that changed the doc's content outside its frontmatter.
- **P4-R18 Restructure.** It is agent-led and preserves facts:
  - parts are `parts/{spec,flow,instruct,cli}/`;
  - hubs are ≤80 lines, notes ≤60, and `lessons.md` is unbounded;
  - every allow-marker inserted by migrate is removed.

  Evidence: `tests/literals/facts.sh` extracts every `D<nn>`, `F<nn>`, `SF<nn>` and every backticked token from the tree. The restructure must keep the set: before ⊆ after.
- **P4-R19 Ratchet.** After the restructure, this repo's `.vibe/spec.json` `strictness` sets SF22 and SF30–SF34 to `error`, and SF29 per P4-R17a. CI runs `vibe-spec validate --strict`. The defaults for other users stay warn.
- **P4-R20 Dogfood tooth.** Once Task 5 wires the hooks here, any agent editing `.spec/**`, `docs/spec/**`, `.vibe/spec.json` or `docs/superpowers/**` must invoke `Skill(spec)`, or the matching superpowers skill, first. Every later dispatch says so.

## Global Constraints

- MSRV 1.88. No new crates. Use `std::thread`/`std::sync::mpsc` for time caps, `serde_json` for payloads, and `text::today_utc` for dates.
- Exit codes: 0 ok, 1 findings or refusal, 2 usage, 3 no root. Hooks exit only 0, or 2 for a tooth block.
- `--json` on every new command prints one Spec JSON v1 object on exit 0. Without a root it exits 3 with empty stdout.
- Every read that feeds a write or a gate fails closed. Multi-file writes are all-or-nothing with restore. Never write or delete through a symlink.
- Printed paths are project-relative and `/`-joined.
- Commits: Conventional Commits, ≤50-char subject, no body, signed. If signing fails, stage the changes and report BLOCKED.
- Run `cargo test`, `bash tests/parity/run.sh`, `bash tests/run.sh`, `node flow/engine/tests/run.mjs` and `bash tests/openspec/run.sh` one at a time, never concurrently.
- Probes of root-discovering commands run in a scratch tree that has its own spec root.
- Edits to `.spec/**` (later `docs/spec/**`) happen only inside `bash .agents/skills/vibe/scripts/set-state.sh strategy.spec` … `idle`.

## Review Focus

1. **A 20 MB transcript with the skill load on its last line.** The guard must still find it, and finish in under 1 s. Task 2 owns the test `guard_scans_a_large_transcript_in_time`.
2. **Skill text in prose or a tool result.** A user message saying "use Skill(spec)", or a tool result that contains `"name":"Skill"`, must not count. Task 1 owns the test `prose_and_tool_results_do_not_count`.
3. **A payload path that names the target through a symlinked cwd or `..`.** For example, `/tmp/p` against `/private/tmp/p`, or `docs/x/../spec/a.md`. Classification must still hit the target. Task 2 owns the test `targets_survive_symlinked_cwd_and_dotdot`.
4. **`migrate` run twice.** The second run must exit 0 with no writes. Task 10 owns the test `migrate_twice_is_a_no_op`.
5. **Windows-style payload paths (`C:\\p\\docs\\spec\\a.md`).** They must classify like `/`-paths. Task 2 owns the test `backslash_paths_classify`.

---

### Task 1: Hook payloads and transcript scan

**Files:**
- Create: `crates/vibe-spec/src/hook.rs` (payload type), `crates/vibe-spec/src/transcript.rs`
- Modify: `crates/vibe-spec/src/main.rs` (`mod hook; mod transcript;`)
- Modify: `crates/vibe-spec/tests/common/mod.rs` (stdin helper)
- Commit: `crates/vibe-spec/tests/fixtures/hooks/*`. These are already in the working tree. They are sanitized: project dir → `/work/project`, transcript dir → `/home/u/.claude/projects/p`.
- Create: `tests/hooks/capture.sh`. It re-runs the capture: a temp project with dumping hooks, `claude -p --model haiku --setting-sources project`, and `CLAUDE_CODE_OAUTH_TOKEN` from the environment, never printed. It is not in CI.
- Test: `crates/vibe-spec/tests/hook_payload.rs`

**Interfaces:**
- Produces:
  - `pub struct hook::Payload { pub event: String, pub cwd: PathBuf, pub transcript: Option<PathBuf>, pub agent_id: Option<String>, pub tool: Option<String>, pub file: Option<String>, pub command: Option<String> }`. `file` comes from `tool_input.file_path`, else `tool_input.notebook_path`. `command` comes from `tool_input.command`.
  - `pub fn hook::parse(stdin: &[u8]) -> Result<Payload, String>`.
  - `pub fn transcript::loaded(main: &Path, agent_id: Option<&str>, names: &[&str]) -> std::io::Result<bool>`. It returns true if any name is loaded per P4-R2. When `agent_id` is set, it also scans `<main stem>/subagents/agent-<id>.jsonl`, and a missing subagent file is treated as not loaded. It streams line by line and does a substring prefilter (`"Skill"` or `<command-name>`) before parsing any JSON.
  - `pub fn common::spec_stdin(cwd: &Path, args: &[&str], env: &[(&str, &str)], stdin: &[u8]) -> Out`.

- [ ] **Step 1: Write the failing tests** in `tests/hook_payload.rs`:
  - `parses_every_captured_payload`: every fixture `*.json` except `subagent-meta.json` parses. `subagent-write-parent-skill.json` gives `agent_id == Some("a315ef01f296b121d")`, `tool == Some("Write")`, and `file` ending in `docs/spec/b.md`.
  - `skill_loads_are_found`. Point `main` at a fixture copy laid out as `<tmp>/<sid>.jsonl` plus `<tmp>/<sid>/subagents/agent-<id>.jsonl`, then assert:
    - `transcript-main-skill.jsonl` gives `loaded(..., None, &["spec"]) == true`;
    - `transcript-slash-command.jsonl` gives true;
    - `transcript-subagent-no-skill.jsonl` used as main gives false;
    - parent `transcript-parent-of-subagents.jsonl` with `agent_id` `a6d9f9d932ee550ba` gives true;
    - a parent with no load plus an agent file with `transcript-subagent-own-skill.jsonl` gives true;
    - `&["brainstorming"]` against the main-skill fixture gives false;
    - `plugin:spec` in `input.skill` counts for `spec`.
  - `prose_and_tool_results_do_not_count` (Review Focus 2): a user text `please use Skill(spec)` and a `tool_result` containing `{"name":"Skill","input":{"skill":"spec"}}` both give false.
  - `malformed_lines_are_skipped`: a non-JSON line between valid lines does not abort the scan.
- [ ] **Step 2:** Run `cargo test -p vibe-spec --test hook_payload`. Expected: FAIL to compile, because the modules do not exist.
- [ ] **Step 3:** Implement `hook::parse`, `transcript::loaded` and `common::spec_stdin`. `spec_stdin` spawns with `Stdio::piped()`, writes stdin, then waits.
- [ ] **Step 4:** Run the test again. Expected: PASS. Then run the full pre-commit command: `cargo fmt --all && cargo clippy --workspace --all-targets -q -- -D warnings && cargo test -p vibe-spec -q`.
- [ ] **Step 5: Commit** `feat(spec): parse hook payloads and transcripts`.

### Task 2: The tooth — `vibe-spec hook guard`

**Files:**
- Modify: `crates/vibe-spec/src/hook.rs` (`guard`), `crates/vibe-spec/src/main.rs` (`Hook { event: HookEvent }` with `HookEvent::{SessionStart, Guard, Validate}`), `crates/vibe-spec/src/root.rs` (read `tooth` from `.vibe/spec.json`)
- Test: `crates/vibe-spec/tests/hook_guard.rs`

**Interfaces:**
- Consumes: `hook::parse`, `transcript::loaded`, `root::discover`.
- Produces:
  - `pub fn hook::guard(stdin: &[u8]) -> (u8, Vec<u8>)`, which returns exit 0 or 2. Stderr is printed inside the function.
  - `pub enum hook::Target { Spec, Brainstorm, Plans }`.
  - `pub fn hook::classify(project: &Path, root: &Path, file: &str) -> Option<(Target, String)>`, where the `String` is the `/`-joined project-relative path.
  - `pub fn root::tooth(project: &Path) -> Result<Tooth, String>` with `enum Tooth { Block, Warn }`. Absent config or key gives `Block`.

- [ ] **Step 1: Write the failing tests.** Build each payload from a fixture with `cwd` and `transcript_path` rewritten to a temp project that has its own `docs/spec/` root.
  - `unskilled_edit_is_blocked`: no skill load, Write to `docs/spec/areas/a.md`. Expect exit 2 and stderr exactly `blocked: invoke Skill(spec) before editing docs/spec/areas/a.md\n`. **This is the discriminating test:** with the guard replaced by `return (0, vec![])`, it fails. Record that RED in the report.
  - `skilled_edit_is_allowed`: main-skill transcript gives exit 0 and empty stderr.
  - `subagent_inherits_parent_load`: the parent transcript has the load and the payload has an `agent_id`. Expect exit 0.
  - `superpowers_targets`:
    - `docs/superpowers/specs/2026-01-01-x-design.md` without a load gives exit 2 and `blocked: invoke Skill(superpowers:brainstorming) or Skill(spec) before editing docs/superpowers/specs/2026-01-01-x-design.md`;
    - with a `superpowers:brainstorming` load it gives exit 0;
    - with `spec` loaded it gives exit 0;
    - `plans/` with `superpowers:writing-plans` gives exit 0.
  - `spec_json_is_a_target`: `.vibe/spec.json` without a load gives exit 2.
  - `other_paths_pass`: `src/main.rs` gives exit 0 and empty stderr.
  - `fail_open_causes`. Each case exits 0 with one stderr line starting `vibe-spec hook guard: allowing — `:
    - empty stdin;
    - `{` alone;
    - a missing transcript file;
    - a project without a spec root.
  - `tooth_warn_override`: `.vibe/spec.json` `{"tooth":"warn"}` gives exit 0 and stderr `vibe-spec hook guard: warn — blocked: invoke Skill(spec) before editing docs/spec/a.md\n`.
  - `bash_writes_warn`: `tool_name: Bash` with `echo x > docs/spec/a.md` gives exit 0 and stderr `vibe-spec: warning: a shell write to docs/spec/a.md bypasses the spec tooth\n`. `cat docs/spec/a.md` gives empty stderr.
  - `targets_survive_symlinked_cwd_and_dotdot` (Review Focus 3): `cwd` passes through a symlink to the project, and the file is given as `docs/x/../spec/a.md`. Both give exit 2.
  - `backslash_paths_classify` (Review Focus 5): a unit test of `classify` with `C:\\p\\docs\\spec\\a.md` and project `C:\\p`.
  - `guard_scans_a_large_transcript_in_time` (Review Focus 1): write a 20 MB transcript of filler assistant records with the Skill load last. Expect exit 0 and an elapsed time under 1 s.
    - Mark it `#[ignore]`, because wall-clock asserts flake on loaded CI runners.
    - The phase gate runs it with `cargo test --release -p vibe-spec --test hook_guard -- --ignored`.
    - A non-ignored twin asserts only exit 0 on the same file.
- [ ] **Step 2:** Run `cargo test -p vibe-spec --test hook_guard`. Expected: FAIL.
- [ ] **Step 3: Implement `hook guard`.**
  - Resolve the project and root from the payload's `cwd`.
  - Compare file identity, not spelling: canonicalize the existing parent directory of the target, and fall back to lexical normalization.
  - Required names: `Target::Spec` → `["spec"]`, `Brainstorm` → `["superpowers:brainstorming","spec"]`, `Plans` → `["superpowers:writing-plans","spec"]`. Match on the name after the last `:` only where the requirement has no namespace. `superpowers:brainstorming` must match exactly.
- [ ] **Step 4:** Run the tests and the pre-commit command. Expected: PASS. Then revert the guard to `return (0, vec![])` and confirm `unskilled_edit_is_blocked` fails. Restore the guard.
- [ ] **Step 5: Commit** `feat(spec): add the spec path tooth`.

### Task 3: Validate hooks — `--changed`, `--file`, `--format hook`, `hook validate`

**Files:**
- Modify: `crates/vibe-spec/src/validate.rs` (`Options { strict, format, changed: bool, file: Option<String> }`, `Format::Hook`), `crates/vibe-spec/src/git.rs` (`pub fn changed_files(top: &Path) -> Option<Vec<String>>`), `crates/vibe-spec/src/hook.rs` (`validate`), `crates/vibe-spec/src/main.rs`
- Test: `crates/vibe-spec/tests/validate_hooks.rs`

**Interfaces:**
- Produces:
  - `pub fn hook::validate(stdin: &[u8], cwd: &Path) -> (u8, Vec<u8>)`. It always returns exit 0.
  - `validate --format hook --event <PostToolUse|Stop>` output:
    - For PostToolUse with findings: `{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"<text findings, one per line>"}}`.
    - For Stop with findings: `{"systemMessage":"<text findings>"}`.
    - No findings: nothing.

- [ ] **Step 1:** Check the PostToolUse `additionalContext` and Stop `systemMessage` output shapes against the current Claude Code hooks reference, using context7 or the docs site. Record the source URL in the report. If either shape differs, use the documented one and note the change.
- [ ] **Step 2: Write the failing tests:**
  - `file_filters_to_one_file`: a tree with findings in `a.md` and `b.md`. `validate --file docs/spec/a.md --json` lists only `a.md` findings. Exit 1 if any are errors.
  - `changed_uses_git_status`: commit a clean tree, then edit `b.md`. `--changed` reports only `b.md` findings. A clean tree gives stdout `validate: no spec changes\n` and exit 0.
  - `changed_without_git_uses_the_session_marker`: with no `.git`, the `.vibe/spec-session` mtime falls between `a.md` and `b.md`, so only `b.md` is reported.
  - `hook_validate_post_tool_use`: the `post-write.json` fixture, rewritten to a temp tree whose `a.md` has an SF error, gives exit 0 and stdout parsing as JSON with `hookSpecificOutput.hookEventName == "PostToolUse"` and `additionalContext` containing `a.md`.
  - `hook_validate_stop`: the `stop.json` fixture plus a changed file with a finding gives exit 0 and stdout `{"systemMessage":…}`.
  - `hook_validate_is_capped`: env `VIBE_SPEC_HOOK_CAP_MS=1` with a 2 000-file tree gives exit 0, empty stdout, and stderr starting `vibe-spec hook validate: skipped — over 1 ms`. The env var overrides the cap and exists for tests only.
  - `hook_validate_never_fails`: malformed stdin, no root, and an unknown event all give exit 0.
- [ ] **Step 3:** Run the tests. Expected: FAIL.
- [ ] **Step 4: Implement.**
  - Filter at the `Report` level: keep findings by `file`, and recompute the counts and text from the kept findings only.
  - The cap: run the validation on a worker thread and wait with `mpsc::Receiver::recv_timeout`.
- [ ] **Step 5:** Run the tests, `bash tests/parity/run.sh` (alone; `validate` parity must stay `0 failed`), `node contracts/run.mjs`, and the pre-commit command. Expected: all PASS.
- [ ] **Step 6: Commit** `feat(spec): add validate hooks`.

### Task 4: Router — `using-spec` skill and `hook session-start`

**Files:**
- Create: `spec/using-spec/SKILL.md`, from `spec/reference/using-spec.md`, which is then deleted. Point its references at the new path.
- Create symlinks: `.claude/skills/using-spec`, `.agents/skills/using-spec` → `../../spec/using-spec`. `plugin/skills/using-spec` → `../../spec/using-spec` is created by `build-plugin.sh` in Task 5.
- Modify: `crates/vibe-spec/src/hook.rs` (`session_start`), `crates/vibe-spec/src/main.rs`
- Test: `crates/vibe-spec/tests/hook_session.rs`

**Interfaces:**
- Produces: `pub fn hook::session_start(stdin: &[u8], cwd: &Path, harness: Harness) -> (u8, Vec<u8>)`. `Harness::{Claude, Cursor, Other}` is detected from the environment per P4-R6. Also `const USING_SPEC: &str = include_str!("../../../spec/using-spec/SKILL.md")`; the body after the frontmatter is injected.

- [ ] **Step 1: Write `spec/using-spec/SKILL.md`.**
  - Frontmatter: `name: using-spec` and a trigger-only description.
  - Body, ≤150 words: a `<SUBAGENT-STOP>` block, the 1 % rule, a three-row routing table, and a Red Flags table with three rows taken from the spec skill's table.
  - The coexistence sentence, verbatim from §6.2: spec decides *where* work is recorded; superpowers process skills decide *how* feature work is designed; if superpowers is absent, the spec fallback applies.
- [ ] **Step 2: Write the failing tests:**
  - `claude_shape`: env `CLAUDE_PROJECT_DIR` gives exit 0, and stdout is JSON whose `hookSpecificOutput.hookEventName == "SessionStart"`. Its `additionalContext` starts with `<EXTREMELY_IMPORTANT>`, contains `using-spec`, and ends with `</EXTREMELY_IMPORTANT>`.
  - `cursor_and_other_shapes`: `CURSOR_PLUGIN_ROOT` gives an `additional_context` key; an empty env gives an `additionalContext` key.
  - `writes_the_session_marker`: a project with a root gets `.vibe/spec-session` created.
  - `no_root_still_routes`: a project without a root still gets exit 0 and the injection; no marker is written outside a project.
  - `skill_is_within_budget`: the `using-spec` body is ≤150 words and its frontmatter `name` is `using-spec`.
- [ ] **Step 3:** Run the tests: FAIL. Implement, run them again: PASS. Then run the pre-commit command and `bash tests/run.sh` alone, because the spec suite references the router file.
- [ ] **Step 4: Commit** `feat(spec): add the using-spec router hook`.

### Task 5: Shim and wiring

**Files:**
- Create: `spec/hooks/run-hook.sh` (mode 755)
- Modify: `build-plugin.sh`. The generated `plugin/hooks/hooks.json` gets the four spec events next to flow's SessionStart. The generator also creates `plugin/agents -> ../spec/agents` and `plugin/skills/using-spec`. Regenerate `plugin/`.
- Modify: `.claude/settings.json`: add the four spec hooks per P4-R8, keeping flow's hooks unchanged.
- Test: `spec/tests/hooks.sh` (new, called from `spec/tests/run.sh`), feeding the committed fixtures through the shim.

- [ ] **Step 1: Write the failing shell tests:**
  - **Binary missing.** `PATH` has no `vibe-spec`, `VIBE_SPEC_BIN` is unset, and `CLAUDE_PROJECT_DIR` points at an empty dir. Each event gives exit 0 and exactly one stderr line, `vibe-spec: binary missing — spec hooks skipped (build with: cargo build -p vibe-spec)`.
  - **Pass-through.** `VIBE_SPEC_BIN=target/debug/vibe-spec` with an unskilled Write fixture gives exit 2 and the block line.
  - **Crash.** `VIBE_SPEC_BIN` points at a script that exits 5. The result is exit 0 and `vibe-spec: hook guard failed (exit 5) — allowing`.
  - **Generated hooks.** `jq` over `plugin/hooks/hooks.json` shows the SessionStart matcher `startup|clear|compact`, the PreToolUse matcher `Edit|Write|MultiEdit|NotebookEdit|Bash`, the PostToolUse matcher `Edit|Write|MultiEdit`, and a Stop entry.
  - **Drift check.** `bash build-plugin.sh --check` exits 0.
- [ ] **Step 2:** Run `bash spec/tests/hooks.sh`: FAIL. Implement, run it again: PASS. Then run `shellcheck spec/hooks/run-hook.sh spec/tests/hooks.sh` (it must print nothing) and `bash tests/run.sh` (alone).
- [ ] **Step 3: Commit** `feat(spec): wire spec hooks through a shim`. From this commit on, P4-R20 applies in this repo.

### Task 6: Agents — register two, retire two

**Files:**
- Create: `spec/agents/spec-promoter.md`, `spec/agents/spec-health.md`
- Delete: `spec/agents/{spec-interviewer,spec-tracer,spec-promoter,spec-health}/`, the old SKILL.md-format dirs
- Create symlinks: `.claude/agents/spec-promoter.md`, `.claude/agents/spec-health.md`
- Modify:
  - `spec/SKILL.md`: the agents map, roles and commands; the interview and trace commands route to the spec skill itself.
  - `spec/README.md:129`, `spec/AGENTS.md:18`.
  - `spec/tests/run.sh`, the lines that assert the four agent names.
- Create: `tests/hooks/capture-agent.sh` and the fixture `crates/vibe-spec/tests/fixtures/hooks/agent-spec-health.json`
- Test: `spec/tests/agents.sh` (new, called from `spec/tests/run.sh`)

- [ ] **Step 1: Write the agents per P4-R9.**
  - The promoter runs `vibe-spec promote <feature>`, shows the checklist, and asks before `--done`.
  - The health agent runs `vibe-spec validate --json` and `vibe-spec check-drift --json` and returns findings.
- [ ] **Step 2: Write `spec/tests/agents.sh`** and run it. It must FAIL before the files exist and PASS after. For each agent it asserts:
  - ≤40 lines;
  - frontmatter keys `name`, `description`, `tools` and `model`, with values `sonnet` and `haiku`;
  - the exact write-path or read-only line, and the binary-missing line;
  - no `spec-interviewer` or `spec-tracer` string anywhere under `spec/`, `.claude/`, `plugin/` or `flow/`.
- [ ] **Step 3: Capture.**
  - `tests/hooks/capture-agent.sh` copies `spec/agents/spec-health.md` into a temp project's `.claude/agents/` and adds a dumping PreToolUse hook for `Read|Bash`.
  - It runs `claude -p "Use the spec-health agent to check this project." --model haiku --setting-sources project`.
  - It writes the first dumped payload with `agent_type == "spec-health"`, sanitized, to the fixture. Run it once and commit the fixture.
  - Add a test to `spec/tests/agents.sh` that asserts the fixture has `agent_type: "spec-health"`.
- [ ] **Step 4: Plugin validation.** Assemble a resolved copy (`cp -RL plugin "$tmp/vibe"`) and run `claude plugin validate --strict "$tmp/vibe"`. Record the output; it must print `Validation passed`. Wrap this as `spec/tests/plugin-validate.sh`. It skips with one line when `claude` is not installed, so CI skips it, and is called from `spec/tests/run.sh`.
- [ ] **Step 5:** Run `bash tests/run.sh` (alone) and `shellcheck` on the new scripts. Expected: PASS.
- [ ] **Step 6: Commit** `feat(spec): register promoter and health agents`.

### Task 7: `vibe-spec setup`

**Files:**
- Create: `crates/vibe-spec/src/setup.rs`; templates under `crates/vibe-spec/src/templates/` (`pre-commit.sh`, `opencode-plugin.js`, `agents-fragment.md`)
- Modify: `crates/vibe-spec/src/main.rs`, `contracts/spec-json.schema.json` (def `setup`: `v`, `written`, `skipped`), `contracts/run.mjs`, plus goldens (3 valid, 2 invalid)
- Test: `crates/vibe-spec/tests/setup.rs`

**Interfaces:**
- Produces: `pub fn setup::run(found: &Result<Root, Miss>, cwd: &Path, opts: &setup::Options) -> (u8, Vec<u8>)`, with `Options { git_hooks: bool, opencode: bool, agents_md: bool, json: bool }`.

- [ ] **Step 1:** Check the OpenCode plugin API (`tool.execute.after` hook signature, plugin file location) and the agent frontmatter (`description`, `mode: subagent`, `model`, `tools`) against current OpenCode docs, using context7 or Exa. Record the URLs and the exact shapes in the report. The templates follow the docs.
- [ ] **Step 2: Write the failing tests:**
  - `no_flag_is_usage`: exit 2.
  - `git_hook_installs_and_is_idempotent`:
    - Two runs give exit 0, and the file starts with `#!/usr/bin/env bash` and contains `# vibe-spec pre-commit v1`.
    - Committing a spec file that has an SF error makes the hook block: `git commit` exits non-zero.
    - With `vibe-spec` absent from the hook's PATH, the commit succeeds and prints one stderr line.
  - `git_hook_refuses_a_foreign_hook`: an existing `pre-commit` without the marker gives exit 1 and leaves the file unchanged.
  - `opencode_files`: writes the three files with markers. Re-running gives exit 0. A foreign `.opencode/agent/spec-health.md` gives exit 1 with nothing written (all-or-nothing).
  - `agents_md_block`:
    - The block is appended once and replaced on re-run.
    - It is ≤6 lines between the markers.
    - Text outside the markers is byte-identical, including flow's `<!-- vibe:… -->` blocks.
  - `json_shape`: `--git-hooks --json` validates against the `setup` def.
- [ ] **Step 3:** Run the tests: FAIL. Implement, run them again: PASS. Then run the pre-commit command and `node contracts/run.mjs`.
- [ ] **Step 4: Dogfood.** Run `vibe-spec setup --agents-md` in this repo. The router block lands in `AGENTS.md` outside the managed blocks. Run `bash tests/run.sh` (alone).
- [ ] **Step 5: Commit** `feat(spec): add setup for git hooks and opencode`.

### Task 8: F19 docs and gate

**Files:**
- Modify (inside `strategy.spec`; load `Skill(spec)` first, per P4-R20):
  - `.spec/tech-spec.md`: the hooks, tooth, agents and setup sections are rewritten to today's truth, and the false "registered via the plugin manifest" claim is fixed;
  - `.spec/plan.md`: F18 set to `DONE`, because CI run 39242955610 was green; F19 set to `IN PROGRESS — local gate green; CI pending push`;
  - `.spec/lessons.md`: one lesson per late-found defect, if any;
  - `CHANGELOG.md`, `README.md` (spec section);
  - bump `updated:` in every changed spec doc.
- Then run `bash flow/scripts/regen-active-rules.sh`.

- [ ] **Step 1:** Write the docs. Run `bash .agents/skills/spec/scripts/validate.sh` and `./target/debug/vibe-spec validate`; both must report `Errors: 0`.
- [ ] **Step 2: F19 gate.** Run each command alone and paste the tail:
  - `cargo fmt --all --check && cargo clippy --workspace --all-targets -q -- -D warnings`
  - `cargo test --workspace --locked`
  - `bash contracts/run.sh`
  - `bash tests/parity/run.sh`
  - `bash tests/run.sh`
  - `bash spec/tests/plugin-validate.sh`
  - shellcheck over the tracked `*.sh`
- [ ] **Step 3: Commit** `docs(spec): record f19 hooks and agents`.

### Task 9: `vibe-spec init`

**Files:**
- Create: `crates/vibe-spec/src/init.rs`, `crates/vibe-spec/src/templates/init/{README,AGENTS,product,tech,design,plan,lessons}.md`
- Modify: `crates/vibe-spec/src/main.rs`, contracts (def `init`: `v`, `root`, `created`, `skipped`; plus goldens), `crates/vibe-spec/src/validate.rs`. The no-root message already names `vibe-spec init`; keep it.
- Test: `crates/vibe-spec/tests/init.rs`

**Interfaces:**
- Produces: `pub fn init::run(found: &Result<Root, Miss>, cwd: &Path, to: Option<&Path>, json: bool) -> (u8, Vec<u8>)`.

- [ ] **Step 1: Write the failing tests:**
  - `seeds_a_layered_root`: an empty git project gives `docs/spec/` with the seven files, then `vibe-spec root --json` reports `layout: layered`, then `vibe-spec validate --strict` exits 0. The seeded hubs must pass strict.
  - `never_overwrites`: a pre-existing `docs/spec/product.md` is byte-identical afterwards, and stdout has `SKIP: docs/spec/product.md`.
  - `to_flag_and_discovered_root`: `--to x/spec` seeds there, and a discovered `.spec` root seeds into `.spec`.
  - `refuses_a_symlinked_target`: a symlinked `docs/spec` gives exit 1 with nothing written.
  - `json_shape`.
- [ ] **Step 2:** Run the tests: FAIL. Implement, run them again: PASS. Then run the pre-commit command and `node contracts/run.mjs`.
- [ ] **Step 3: Commit** `feat(spec): add vibe-spec init`.

### Task 10: migrate — scan, plan, dry-run, check

**Files:**
- Create: `crates/vibe-spec/src/migrate.rs`, `crates/vibe-spec/src/literal.rs`
- Modify: `crates/vibe-spec/src/main.rs`, `crates/vibe-spec/src/root.rs` (read the `migrate` config: `include`, `exclude`, `floor`), contracts (def `migrate`: `v`, `from`, `to`, `moves`, `rewrites`, `created`, `markers`, `dry_run`; plus goldens)
- Test: `crates/vibe-spec/tests/migrate.rs`, which uses a planted corpus built in the test

**Interfaces:**
- Produces:
  - `pub fn literal::find(line: &[u8]) -> Vec<usize>`, the byte offsets of each literal, empty when the line is tagged.
  - `pub fn literal::rewrite(line: &[u8], from: &str, to: &str, renames: &[(String, String)]) -> Vec<u8>`.
  - `pub struct migrate::Plan { pub moves: Vec<(String, String)>, pub rewrites: Vec<Rewrite>, pub created: Vec<(String, Vec<u8>)>, pub scanned: usize, pub before: usize }`, where `Rewrite { file, line, old, new }`.
  - `pub fn migrate::plan(project: &Path, root: &Root, to: &str, layered: bool) -> Result<Plan, String>`.

- [ ] **Step 1: Write the failing tests:**
  - `literal_regex`. The function must match `.spec/`, `".spec"`, `$ROOT/.spec` and `./.spec`. It must not match `strategy.spec`, `x.spec.ts`, `tech-spec.md`, `.vibe/spec.json`, or a line ending `<!-- literal -->`.
  - `dry_run_prints_the_table_and_writes_nothing`.
    - On the planted corpus, stdout begins `migrate: dry run — <n> literal(s) in <m> file(s)` and lists each line as `<file>:<line>  <old> → <new>`.
    - Every file hash is unchanged afterwards.
  - `floor_miss_aborts`: `migrate.floor: 999` gives exit 1, `migrate: ERROR: scanned <m> file(s), floor 999`, and nothing written.
  - `nothing_to_migrate`: a corpus with 0 literals and the root still at `.spec` gives exit 1 and `migrate: ERROR: no .spec literal found` (tagged `// literal`).
  - `migrate_twice_is_a_no_op` (Review Focus 4): the root is already at `docs/spec` and there are 0 literals. Expect exit 0, `migrate: already migrated`, and no writes.
  - `symlinks_are_not_followed`: a symlink to a file that contains literals is scanned once, through its real path, and never rewritten through the link.
  - `check_mode`: `--check` gives exit 1 with a listing when literals remain, and exit 0 on a clean corpus.
  - `layered_plan`: `tech-cli.md` moves to `tech/cli.md`; two `plan.md` D-bullets give `decisions/d12-orders-in-the-skill.md` and `decisions/d13-carrier-split.md`, and `plan.md` lists links to them; a `product-x.md` refuses.
- [ ] **Step 2:** Run the tests: FAIL. Implement `literal` and `plan`, then the dry-run and `--check` output. Run them again: PASS. Then run the pre-commit command and `node contracts/run.mjs`.
- [ ] **Step 3: Commit** `feat(spec): plan migrations with a population check`.

### Task 11: migrate — execute, markers, revert

**Files:**
- Modify: `crates/vibe-spec/src/migrate.rs` (`execute`). Reuse the restore pattern from `promote::commit`; if you can share it without changing promote's behaviour, move it to `vibe_core::fs`.
- Test: `crates/vibe-spec/tests/migrate.rs`

**Interfaces:**
- Produces: `pub fn migrate::run(found: &Result<Root, Miss>, cwd: &Path, opts: &migrate::Options) -> (u8, Vec<u8>)`.

- [ ] **Step 1: Write the failing tests:**
  - `migrates_a_flat_repo`. On a committed planted repo:
    - `git mv` moves `.spec` → `docs/spec`;
    - `.vibe/spec.json` gains `"root": "docs/spec"`, and other keys are kept;
    - literals in the file set are rewritten and `--check` exits 0;
    - `git status` shows renames, not delete plus add.
  - `layered_execute`:
    - the root hubs end up with `type: hub`;
    - the moved topic is at `tech/cli.md`;
    - the decision files exist;
    - relative links inside and into moved files resolve, so `validate` reports no SF22;
    - the allow-markers from P4-R14 are inserted and listed in stdout.
  - `revert_restores_the_tree` (the **discriminating test**). Hash the whole tree, then make one rewrite target read-only so the write fails mid-way. Expect exit 1, `— reverted`, an identical tree hash, and `git status --porcelain` identical to before.
    - Mutation check: with the restore step removed, the test fails. Record that RED.
  - `non_git_project_renames`: without `.git`, a plain rename takes the place of `git mv`.
  - `moved_and_rewritten_docs_get_today`: every moved or rewritten spec doc ends with `updated: <today_utc>`, and `validate --strict` reports no SF29.
  - `legacy_root_still_found`: after migrating one fixture, a second untouched `.spec` fixture is still discovered (`Source::DotSpec`). Migrate only moves the root it was pointed at.
- [ ] **Step 2:** Run the tests: FAIL. Implement, run them again: PASS. Then run the pre-commit command and `bash tests/parity/run.sh` (alone).
- [ ] **Step 3: Commit** `feat(spec): execute migrations all-or-nothing`.

### Task 12: Prepare this repo's migration

**Files:**
- Create: `.vibe/spec.json`, holding the `migrate` set from P4-R16 only. There is no `root` key yet, and `tooth` is not set.
- Dual-root shipped logic (P4-R16), each change with a test in its existing suite:
  - `flow/engine/root.mjs` `MARKERS` = `['docs/spec', '.spec', '.git']`. Add an engine test: a project with only `docs/spec/` resolves.
  - `flow/content/policy.json`: every `.spec/…` rule gains its `docs/spec/…` twin. In `flow/engine/tests/policy.test.mjs`, each protected path is blocked under both roots.
  - `flow/scripts/detect-context.sh` arms and `flow/hooks-fallback/pre-tool-use-guard.sh`: both roots, with `flow/tests/run.sh` cases for `docs/spec/lessons.md`.
  - `flow/scripts/{regen-active-rules,orders,doctor}.sh`: their marker walks accept both roots. `regen-active-rules.sh` reads the `lessons.md` that `vibe-spec root` names when the binary is on PATH, then `docs/spec/lessons.md`, then `.spec/lessons.md`.
  - `build-plugin.sh` session-start heredoc: the detection accepts both roots. Regenerate `plugin/`.
  - `install.sh`: the seed and preserve text names both roots, and the `setup.sh` call (until F22) is tagged `# literal`.
  - Tag kept legacy lines in `crates/vibe-spec/src/` with `// literal`. Add the file-level marker to `crates/*/tests/*.rs` files and `CHANGELOG.md` (P4-R16a).
- Create: `tests/literals/run.sh`, the full-repo sweep (P4-R16a). Wire it into `tests/run.sh`. It must pass once the migration commit lands; until then it runs in report-only mode, controlled by `LITERALS_REPORT_ONLY=1`, which `tests/run.sh` sets until Task 13.
- Tag the prose of issue 20 with `<!-- literal -->`.
- Prepare the callers for P4-R17 so they work on either root:
  - the CI steps call `cargo run -q -p vibe-spec -- validate` and `-- check-drift` instead of the bash scripts;
  - `tests/run.sh` live checks use the same;
  - the parity live leg is removed from `tests/parity/run.sh` and `.github/workflows/ci.yml`, and the ledger records why;
  - `spec/SKILL.md`, `spec/README.md` and `flow/reference/templates/AGENTS.md` command lines become `vibe-spec …`;
  - `AGENTS.md` is re-rendered from the template.
- Create: `tests/literals/facts.sh` (P4-R18). It prints the sorted set of `D\d+`, `F\d+`, `SF\d+` and backticked tokens under the spec root.

- [ ] **Step 1:** Run `./target/debug/vibe-spec migrate --to docs/spec --layout layered --dry-run`. Save the full table to the report. Check that every listed file belongs to the set, and that no oracle, corpus or history file appears.
- [ ] **Step 2:** Run `bash tests/literals/facts.sh > .superpowers/p4/facts-before.txt` and keep the file for Task 14.
- [ ] **Step 3:** Run `bash tests/run.sh`, `node flow/engine/tests/run.mjs` and `cargo test --workspace`, each alone. All must stay green on the still-`.spec` tree.
- [ ] **Step 4: Commit** `chore(spec): prepare the docs/spec migration`.

### Task 13: Migrate this repo (one commit)

- [ ] **Step 1:** Load `Skill(spec)` and run `set-state.sh strategy.spec`.
  - Run `git rm -r -q .spec/archive` first, so migrate never rewrites, counts or marks frozen files.
  - Then run `./target/debug/vibe-spec migrate --to docs/spec --layout layered`.
  - Update `docs/roadmap/README.md` (L4, 71, 101, 102) if migrate did not already rewrite it.
  - Drop `LITERALS_REPORT_ONLY` from `tests/run.sh`.
  - Run `set-state.sh idle`.
- [ ] **Step 2:** Run each check alone. All must pass:
  - `vibe-spec validate` reports `Errors: 0`. Warnings are allowed only where markers were inserted.
  - `vibe-spec migrate --to docs/spec --check` exits 0.
  - `bash tests/literals/run.sh` exits 0.
  - `bash tests/run.sh` passes.
  - `node flow/engine/tests/run.mjs` passes. Only the known working-copy perf budget may fail; if it does, confirm with a clean `git archive` run.
  - `cargo test --workspace` passes.
  - `bash tests/parity/run.sh` passes on the corpus legs.
  - `bash build-plugin.sh --check` passes.
  - `bash flow/scripts/regen-active-rules.sh` leaves `git diff` empty.
- [ ] **Step 3: Commit** everything as one commit: `refactor(spec): migrate to layered docs/spec`.

### Task 14: Restructure the layered tree (agent-led)

**Files:** `docs/spec/**` only, inside `strategy.spec`, with `Skill(spec)` loaded and its restructure mode applied.

- [ ] **Step 1: Hubs.**
  - Rewrite `product.md`, `tech.md`, `design.md` and `plan.md` as hubs of ≤80 lines that link down.
  - Move their facts into `areas/`, `flows/`, `concepts/`, `platform/`, `tech/` and `decisions/` notes, or into `parts/{spec,flow,instruct,cli}/`. Each part gets its own hub, `parts/<p>/<p>.md`, with the same kind folders inside it.
  - Every note is ≤60 lines, has typed frontmatter (`code:` on areas, `applies:` on tech), and is reachable from a hub.
- [ ] **Step 2: Tech topics.** Split each `tech/<t>.md` that runs over 60 lines into a folder with its own hub. The topics today are cli, contracts, instruct, rust and spec. Replace `applies: "**"` with the real glob.
- [ ] **Step 3: Allow-markers.** Remove every marker that migrate inserted. Then set the strictness ratchet in `.vibe/spec.json` (P4-R19).
- [ ] **Step 4: Gate.**
  - `vibe-spec validate --strict` exits 0.
  - `bash tests/literals/facts.sh` output is a superset of `facts-before.txt`, checked with `comm -23 before after`, which must print nothing.
  - `vibe-spec migrate --to docs/spec --check` exits 0.
  - `bash tests/run.sh` passes.
- [ ] **Step 5: Commit** `docs(spec): restructure into layered notes`.

### Task 15: F20 docs, rows and phase gate

**Files:**
- Inside `strategy.spec`:
  - `docs/spec/plan.md` (or wherever the restructure put the sequence): F19 `DONE` if CI is green, else `IN PROGRESS — CI pending push`; F20 `IN PROGRESS — local gate green; CI pending push`;
  - the tech notes for `init`, `migrate` and `setup`;
  - `docs/spec/lessons.md`, one lesson per late-found defect.
- Also: `CHANGELOG.md`, `README.md`, and the regenerated `AGENTS.md` active rules.
- CI (`.github/workflows/ci.yml`): `vibe-spec validate --strict` and `vibe-spec migrate --to docs/spec --check` steps.

- [ ] **Step 1: Phase gate.** Run each command alone and paste the tail:
  - `cargo fmt --all --check && cargo clippy --workspace --all-targets -q -- -D warnings`
  - `cargo test --workspace --locked`
  - `bash contracts/run.sh`
  - `bash tests/parity/run.sh`
  - `bash tests/openspec/run.sh`
  - `bash tests/run.sh`
  - `./target/debug/vibe-spec validate --strict`
  - `./target/debug/vibe-spec migrate --to docs/spec --check`
  - `bash tests/literals/run.sh`
  - `cargo test --release -p vibe-spec --test hook_guard -- --ignored`
  - `./target/debug/vibe-spec check-drift`
  - `bash spec/tests/plugin-validate.sh`
  - `git ls-files '*.sh' | grep -Ev '^oracles/' | xargs shellcheck`
- [ ] **Step 2: Commit** `docs(spec): record f20 migration`.

---

## Self-Review

1. **Spec coverage.**

| Requirement | Task |
|---|---|
| Router `using-spec` ≤150 words, SessionStart `startup\|clear\|compact`, `<EXTREMELY_IMPORTANT>` (§6.2) | 4, 5 |
| Tooth: targets, exit 2 one line, Bash warn, fail-open causes, `tooth: warn`, subagents (§6.3) | 1, 2 |
| Discriminating test: tooth vs `exit 0` (§1.5, §9) | 2 |
| Validate hooks exit 0, 2 s / 500 ms caps, `--changed` (git, mtime fallback), `--file` (F19) | 3 |
| Shim: binary missing → exit 0 (§6.3, §10) | 5 |
| `setup --git-hooks` pre-commit `validate --changed --strict` (F19) | 7 |
| Agents promoter + health, ≤40 lines, write path, binary-missing, both platforms; interviewer + tracer retired | 6, 7 |
| Captured subagent payload fixture (index row 4, F19 acceptance) | 1, 6 |
| `claude plugin validate --strict` | 6, 8 |
| OpenCode `tool.execute.after` shim + AGENTS.md fragment ≤6 lines | 7 |
| `init` seeds hubs, README, AGENTS, lessons only (§4) | 9 |
| `migrate --dry-run` table; population assertion; floor-miss revert; allow-markers (F20) | 10, 11 |
| Discriminating test: migrate revert vs half-migrated tree (§9) | 11 |
| `--layout layered`: topics → kind folders, D-records → `decisions/`, hubs | 10, 11, 14 |
| This repo → layered `docs/spec/` in one commit; `.spec/archive/` deleted; roadmap points at new plan | 12, 13 |
| `validate --strict` green; zero `.spec` literals outside `oracles/` and tagged lines (§1.1, §1.2) | 14, 15 |
| `--json` on every new command | 3, 7, 9, 10 |

Wiring `check-drift --pre-merge` into flow compound and `finishing-a-development-branch` stays with F21 (P3c-R2). F21s owns the plugin dir split (`spec/.claude-plugin`); until then, F19's plugin check validates the resolved `plugin/` copy.

2. **Step scan.** No step says TBD. Each code step names a signature and file. Each test step names its assertions with exact strings. Two steps (Task 3 Step 1, Task 7 Step 1) verify external API shapes against docs, because the shapes are outside this repo.
3. **Type consistency.**
   - `hook::Payload`, `hook::parse` and `transcript::loaded` (Task 1) are used unchanged in Tasks 2–4.
   - `validate::Options` gains fields in Task 3, and `main.rs` changes in the same task.
   - `migrate::plan` (Task 10) feeds `migrate::run` (Task 11).
   - `root.rs` gains `tooth` (Task 2) and the `migrate` config (Task 10). Both read the same `.vibe/spec.json`, and a config without `root` still falls through (P3a).
4. **Review Focus.** Each of the five lines names its test in the owning task (Tasks 1, 2, 10).
5. **Proportion.** Signatures, test names and exact strings only, with no code bodies. Task 14 is agent-led writing, gated by strict validation and the facts superset check.
