# Global store and harness projection — Design

**Date:** 2026-10-23 · **Branch:** `villain/vibe-config` · **Arc:** v0.5 front door, pulled
ahead (new row C0) · **Decision:** D48 (new)

## 1. Intent

**Outcome.** One global store under `~/.config/vibe/` holds the user's root instructions, shared
MCP servers and shared skills. One command (`vibe install`) wires the store into Claude Code,
Codex, OpenCode and Factory droid. After that, every session start in any harness, repo or
worktree brings each harness up to date with the store. The `vibe` binary that does this is the
first piece of the v0.5 CLI and its shared Rust infrastructure (`vibe-core`).

**Stated by the owner.**
- vibe must be harness-flexible: Claude, Codex, OpenCode, Factory — "one big plugin".
- A global store for root instructions, shared MCP and shared skills.
- One command installs the toolkit into a harness. A change to the global root reaches all.
- Instructions everywhere — all repos and worktrees — and MCP kept up to date.
- How a harness is fed may differ per harness (OpenCode already gets MCP through a plugin).
- Hooks may trigger a check against the root config at every startup.
- Rust; this becomes the core backend of the `vibe` CLI that other features build on.

**Observed on the owner's machine (2026-10-23).**
- MCP lives in four unrelated places: `~/.claude.json` (grafana, yab-browser),
  `~/.codex/config.toml` (github, outline_mcp, computer-use, node_repl), `~/.factory/mcp.json`
  (mesh), `~/.config/agent-stack/mcp.json` read by the OpenCode plugin `shared-mcp.ts`
  (indexed, context7, exa, serena). The rules tell agents to use serena/context7/exa/indexed;
  only OpenCode loads them.
- Two drifted copies of the global rules: `~/.codex/AGENTS.md` and
  `~/.config/opencode/AGENTS.md` (which `~/.claude/AGENTS.md` symlinks to). Factory has none.
- `~/.agents/skills` is the de facto skills hub; Codex, OpenCode and Factory read it natively,
  Claude does not. `~/.claude/skills` mixes symlinks and real directories.
- Every harness carries third-party hooks (Orca, Superset). They must survive.
- `~/.config/agent-stack/mcp.json` holds literal API keys, one inside a URL.

**Success criteria.**
1. `vibe install` on a fresh `HOME` wires all four harnesses; a second run changes nothing.
2. After a store edit, the next session start of any harness re-projects every file-based target
   (one-line notice); Claude MCP changes are projected by `vibe sync` (the notice says so).
3. The real CLIs accept the generated files under a temporary `HOME`: `codex mcp list --json`,
   `droid mcp list`, `claude mcp get <name>`, `opencode debug config` (where available).
4. No literal secret is written by vibe into any harness file; placeholders only.
5. Nothing vibe did not create is changed or removed: user MCP entries, hooks, skills and
   instructions outside vibe's markers survive install, sync and uninstall byte-for-byte.
6. `vibe uninstall` removes exactly what vibe created and leaves the store in place.
7. `cargo test --workspace`, clippy `-D warnings`, fmt, cargo-deny, `bash tests/run.sh`,
   `bash contracts/run.sh`, `vibe-spec validate --strict` all green.

## 2. Scope

| In | Out |
|---|---|
| `crates/vibe` (bin `vibe`): store, projection, sync, hook, install, uninstall, status, doctor | flow / instruct Rust ports (F5–F15) |
| Generic infra in `vibe-core`: dirs, lock, markers, ordered JSON | `repos.json` registry, worktree lifecycle (C1–C3, C6) |
| Adapters: Claude, Codex, OpenCode, Factory | Network fetch of the store (D43 git source stays instruct's) |
| `vibe spec …` dispatch to `vibe-spec` (thin) | Windows symlinked skills (copy fallback is future) |
| Contracts: store + state schemas and fixtures | Auto-migrating the owner's existing configs |
| Docs: D48, tech note, plan row C0, README, AGENTS dogfood | Codex hook trust automation |

## 3. The store

Root: `$VIBE_STORE`, else `$XDG_CONFIG_HOME/vibe`, else `~/.config/vibe` (also on macOS);
Windows `%APPDATA%\vibe`. It sits beside the planned `instruct/`, `spec/lessons.md`,
`repos.json` (D42, D43) and does not touch them.

```text
~/.config/vibe/
  store.json     # {"v":1,"harnesses":{...},"skills":{"sources":[...]}}
  AGENTS.md      # root instructions (plain Markdown, the single source)
  mcp.json       # {"mcpServers":{...}} — canonical MCP list
  skills/        # shared skills: one dir per skill, each with SKILL.md
```

`store.json` (all keys optional; unknown keys tolerated):
- `harnesses.<id>.enabled` (bool, default true when the harness is detected), ids `claude`,
  `codex`, `opencode`, `droid`.
- `skills.sources`: extra directories whose child dirs with a `SKILL.md` are skills
  (`~` expanded). `skills/` is always a source. `vibe install` adds the toolkit source (the
  directory holding vibe's own `spec`, `vibe`, `using-spec` skills) when it can locate it.

`mcp.json` server (Claude/Factory-compatible shape, which the owner's agent-stack file already
uses): `type` `stdio`|`http` (default `stdio`), `command`, `args`, `env`, `url`, `headers`,
`enabled` (default true), optional `harnesses` (allow-list of ids). Unknown fields tolerated and
not projected. Values may hold placeholders `${NAME}` (NAME = `[A-Z_][A-Z0-9_]*`).

**Secrets.** vibe never writes a literal secret it detects. `vibe store import-mcp <file>`
replaces likely secrets — every header value, env values whose key matches
`KEY|TOKEN|SECRET|PASSWORD|AUTH` (case-insensitive), URL query values whose key matches the
same — with `${VIBE_<SERVER>_<FIELD>}` placeholders and prints the variable names (never the
values) the user must export. `--keep-literals` opts out, with a warning.

## 4. Projection per harness

| Harness | Instructions | MCP | Skills | Trigger |
|---|---|---|---|---|
| Claude | managed block in `$CLAUDE_CONFIG_DIR\|~/.claude`/`CLAUDE.md` holding `@<abs store>/AGENTS.md` (live import) | `claude mcp add-json --scope user` / `remove` during `vibe sync` only | links in `~/.claude/skills` | SessionStart hook entry merged into `~/.claude/settings.json` |
| Codex | managed block in `$CODEX_HOME\|~/.codex`/`AGENTS.md` (copied text; warn > 32 KiB; notice if `AGENTS.override.md` exists) | marker block at the end of `config.toml` | links in `~/.agents/skills` | SessionStart entry merged into `~/.codex/hooks.json` (user must trust it via `/hooks`) |
| OpenCode | vibe plugin pushes the store `AGENTS.md` text through `ctx.session.hook("context")` (live) | vibe plugin reads `mcp.json` at startup, `ctx.mcp.transform`, set-if-absent (live) | `~/.agents/skills` (native) | plugin `setup` spawns `vibe hook session-start --harness opencode` detached |
| Factory | managed block in `~/.factory/AGENTS.md` (copied text) | owned keys merged into `~/.factory/mcp.json` | `~/.agents/skills` (native) | SessionStart entry merged into `~/.factory/hooks.json` |

Rules every adapter obeys:
- **Ownership.** vibe owns only what its state file records (MCP names, hook entries, links,
  the plugin file) or what sits between its markers. A same-named entry vibe does not own wins:
  vibe skips it and reports `kept <harness>'s own <name>` (matches `shared-mcp.ts`).
- **Markers.** D24 grammar, owner `store`: `<!-- vibe:store:begin v=1 hash=<h> -->` …
  `<!-- vibe:store:end -->` in Markdown, `# vibe:store:begin v=1 hash=<h>` … `# vibe:store:end`
  in TOML. Exact-line match; start before end; reversed/duplicate markers → refuse, file
  untouched. The hash covers the block body so hand edits inside it are reported, not lost
  silently.
- **No writes through links.** A target file that is a symlink (e.g. the owner's
  `~/.claude/AGENTS.md`) is refused with one line. Skills links never replace a real directory
  or a link vibe did not create.
- **Atomic, all-or-nothing.** All file writes of one run go through `vibe_core::fs::commit`
  (rollback on failure). Plan in memory first; `--dry-run` runs the same plan and prints it.
- **Lock.** An exclusive lock file (`create_new`, stale after 60 s) is taken before reading the
  state used to build the plan. The hook skips silently when the lock is held.
- **Formatting.** JSON targets are re-serialised with 2-space indentation, key order preserved,
  unknown content preserved; untouched files are never rewritten.

**MCP translation** (placeholder `${N}`; `skip` = server not projected to that harness, with a
one-line reason):

| Field | Claude | Codex | OpenCode (plugin) | Factory |
|---|---|---|---|---|
| `command`, `args` | as is (`${N}` expands) | as is; placeholder → skip | `command: [cmd, ...args]`, resolved from env | as is; placeholder → skip |
| `env` value | as is | `K: "${K}"` → `env_vars=["K"]`; literal → `env`; other placeholder → skip | `environment`, resolved | as is |
| `url` | as is | placeholder → skip | resolved | placeholder → skip |
| `headers` value | as is | `Authorization: "Bearer ${N}"` → `bearer_token_env_var`; `"${N}"` → `env_http_headers`; literal → `http_headers`; else skip | resolved | as is |
| `enabled:false` | not added | `enabled = false` | `disabled: true` | `disabled: true` |
| type | `stdio`/`http` | stdio / streamable http | `local` / `remote` | `stdio` / `http` |

Codex block safety: before writing, scan `config.toml` for `[mcp_servers.<name>]` headers
outside the block (duplicate tables break all of Codex) → skip those names. The block is kept
at the end of the file; content after the end marker that is not a table header or comment →
refuse.

## 5. Commands (`crates/vibe`, bin `vibe`)

| Command | Does |
|---|---|
| `vibe store init [--instructions <file>]` | create the store skeleton; idempotent |
| `vibe store import-mcp <file> [--keep-literals]` | merge servers from any `{"mcpServers":…}` file |
| `vibe store status [--json]` | store validity, per-harness drift, skips, conflicts |
| `vibe sync [--harness a,b] [--dry-run] [--json]` | full projection, including Claude MCP via its CLI |
| `vibe hook session-start [--harness h]` | fail-open fast path: hash check, file projection, one notice line; no subprocess, no network, exit 0 |
| `vibe install [--harness a,b] [--toolkit <dir>] [--dry-run]` | store init, hook registration, OpenCode plugin, then `sync` |
| `vibe uninstall [--dry-run]` | reverse everything the state file records; store kept |
| `vibe doctor` | harness CLIs found, store parse, state, lock |
| `vibe spec …` | exec `vibe-spec` (`$VIBE_SPEC_BIN`, then `PATH`); missing → one advice line, exit 0 |

Exit codes from `vibe_core::exit` and `contracts/exit-codes.json`: 0 ok, 1 findings/refused,
2 usage. The hook always exits 0.

State: `$VIBE_STATE`, else `$XDG_STATE_HOME/vibe`, else `~/.local/state/vibe`; Windows
`%LOCALAPPDATA%\vibe`. `state.json` = `{"v":1,"store_hash":…,"mcp_hash":…,
"targets":{<key>:{"hash":…}},"owned":{"mcp":…,"hooks":…,"plugins":…,"links":…,"created":…}}`,
schema in `contracts/`. `store_hash` may be empty (written after a partial failure, so the next
hook run re-plans). `mcp_hash` is the hash of `mcp.json` at the last full success. Target keys
are `claude:instructions`, `codex:instructions`, `codex:mcp`, `droid:instructions`,
`droid:mcp`. It is both the skip cache and the uninstall record.

## 6. Architecture

- `vibe-core` gains generic, vocabulary-free modules: `dirs` (home/XDG/APPDATA resolution from
  an injected env map), `lock`, `markers` (D24 managed blocks, comment-style parameter),
  `json` (order-preserving value built on the existing `serde` + `serde_json`, no new crates;
  the `preserve_order` feature is ruled out — it breaks the parity corpus).
- `crates/vibe` depends on `vibe-core`, `clap`, `serde`, `serde_json` only (deny.toml bans
  `toml`, `tempfile`, `regex`, network crates). Hand-written TOML emitter for Codex.
- Every path derives from one `Env` value built from the process env; tests run the binary with
  `env_clear()` and a temporary `HOME`, never the real one.
- Adapters return plan items (file write, link create/remove, CLI call); one executor applies
  them and updates state. Hook path = same planner, file items only.

## 7. Verification

- Unit + integration tests per adapter with real-shaped fixtures (symlinked `AGENTS.md`, real
  skill dirs, third-party hooks, user MCP entries, Codex duplicate tables, reversed markers).
- Fake `claude` on `PATH` records argv (unix tests).
- Acceptance (`VIBE_E2E=1`): real `codex`, `droid`, `claude`, `opencode` read the generated files
  under a temp `HOME`.
- Each guard gets a planted-defect check before it is trusted.

## 8. Settled details (builder check, 2026-10-24)

1. **Hook ownership.** D25 splits *repo/plugin* hooks among spec, flow, instruct. D48 adds the
   front door as a fourth, user-level owner with one duty: SessionStart projection. Its output is
   at most one notice line, only when something changed or needs action; it never injects
   instruction content (instructions reach harnesses through files). D48 amends D40's "the vibe
   plugin carries no hooks": vibe registers no plugin hook; it merges one user-level entry.
2. **Executor order and rollback.** (a) all file writes in one `vibe_core::fs::commit`
   (all-or-nothing); (b) link creates/removes, each undone in reverse if a later link fails, then
   files restored; (c) CLI calls last, one per server, each recorded in state only on success. A
   failed CLI call is reported (exit 1) and does not roll back (a)/(b), which are consistent on
   their own. Link handling lives in `crates/vibe`; `vibe-core` gains no link primitive.
3. **Claude MCP.** State records, per owned name, the canonical JSON vibe added. `sync` reads
   `$CLAUDE_CONFIG_DIR/.claude.json` (else `~/.claude.json`) read-only, top-level `mcpServers`:
   owned + equal to record + store changed → `remove` then `add-json`; owned but differs from
   record → skip, `claude: <name> edited outside vibe — kept`; owned but absent → re-add with a
   notice; unowned same name → skip. Scope is always `user`. Placeholders pass through; Claude
   expands them at runtime. Test double: a script that appends argv to a log and exits 0 (or a
   configured code); real behaviour is covered by the `VIBE_E2E` leg.
4. **Hook cache key.** `store_hash` = hash of `store.json`, `AGENTS.md`, `mcp.json` bytes plus
   the sorted `(name, resolved source dir)` skill list plus the binary version. The hook does
   nothing when it equals the stamp; otherwise it plans file items and skill links (cheap
   readdirs) — never CLI items, never network. `sync` always plans everything and detects
   external drift against recorded target hashes.
5. **Hook entries.** Claude `settings.json`, Codex `hooks.json`, Factory `hooks.json` share one
   shape: append to `hooks.SessionStart` the group
   `{"matcher":"startup|resume|clear","hooks":[{"type":"command","command":"<cmd>","timeout":10}]}`
   with `<cmd>` = `"<absolute path of the installing vibe binary>" hook session-start --harness
   <id>`. Identity = the recorded command string. Missing file → created with only that key;
   invalid JSON → that target refused (exit 1), other targets proceed. `doctor` flags a hook
   whose binary path no longer exists; re-running `install` repairs it. Codex prints a trust
   notice (`/hooks`).
6. **OpenCode plugin.** Plain ES module, no imports (OpenCode 2 validates `{id, setup}`;
   `Plugin.define` is identity), embedded with `include_str!` from
   `crates/vibe/src/templates/opencode-plugin.js`, installed to
   `$XDG_CONFIG_HOME|~/.config/opencode/plugins/vibe-store.js` with a first-line marker
   `// vibe:store-plugin v1`; a file without the marker is never overwritten. Rewritten by
   install/sync whenever bytes differ (keeps it in step with the binary). It resolves `${N}`
   from `process.env`; a server with an unset variable is skipped. API shape is taken from the
   installed `@opencode/plugin` 2.0.29 type definitions; a node test drives it with a fake `ctx`.
7. **Workspace rules.** Updating `crates/vibe-parity` graph rules (4 members, `vibe` → `vibe-core`
   edge) is in scope. No new external package enters `Cargo.lock`.
8. **Skills.** Absolute link targets. Collision with a real dir or a foreign link → skip and
   report. Same name in two sources → first wins in order `skills/`, `skills.sources…`,
   toolkit; the shadowed one is reported. Uninstall removes a link only if it is still a symlink
   to the recorded target.
9. **Docs** land in this change after implementation (compound step): D48, tech note, plan row
   C0, README, AGENTS dogfood row, CHANGELOG, lessons; `updated:` bumped; `vibe-spec validate
   --strict` green.

## 9. Calls made by the builder (owner did not decide these)

Store layout and file names; `store.json` key names; marker owner `store`; Claude MCP via CLI at
sync time only; hook registration by merging into each harness's global hooks file rather than
through cached plugins; skills by symlink into `~/.agents/skills` + `~/.claude/skills`; secret
placeholder naming; skip-not-fail for untranslatable servers; no commits until asked.
