---
type: tech
applies:
  - "crates/vibe/**"
updated: 2026-10-24
---

# Global store and projection

Design: [global-store design](../../../superpowers/specs/2026-10-23-global-store-design.md). Decision: [D48](../../decisions/d48-global-store-and-projection.md). Local projection only: no network (D42, D43).

**Store** (`$VIBE_STORE` overrides the whole global root, which later slices also use; else `$XDG_CONFIG_HOME/vibe`, else `~/.config/vibe`; Windows `%APPDATA%\vibe`): `AGENTS.md` (root instructions), `mcp.json`, `skills/`, `store.json`.
- `store.json`: `harnesses.<id>.enabled` (ids `claude`, `codex`, `opencode`, `droid`), `skills.sources` (extra dirs, `~` expanded). Unknown keys tolerated.
- `mcp.json`: `{"mcpServers":{<name>:{type, command, args, env, url, headers, enabled, harnesses}}}`. Values may hold `${NAME}`. Unknown fields are not projected.

**Projection** (the hook plans file items only; `sync` also runs the Claude CLI):

| Harness | Instructions | MCP | Skills |
|---|---|---|---|
| Claude | block in `CLAUDE.md` with `@<store>/AGENTS.md` | `claude mcp add-json --scope user`, `sync` only | links in `~/.claude/skills` |
| Codex | copied block in `AGENTS.md` | marker block at end of `config.toml` | links in `~/.agents/skills` |
| OpenCode | plugin `vibe-store.js` (live) | plugin, set-if-absent | `~/.agents/skills` (native) |
| Factory | copied block in `~/.factory/AGENTS.md` | owned keys in `mcp.json` | `~/.agents/skills` (native) |

**Commands:** `store init|import-mcp|status`, `sync [--harness] [--dry-run] [--json]`, `hook session-start [--harness]`, `install [--harness] [--toolkit] [--dry-run]` (default: the detected harnesses, i.e. config dir present or CLI on `PATH`; an undetected one gets `note <h>: not detected — pass --harness <h> to wire it`; `--harness` forces), `uninstall [--dry-run]`, `doctor`, `spec …` (exec `vibe-spec`). Exit 0 ok, 1 findings or refused, 2 usage. The hook always exits 0.

**State** (`$VIBE_STATE`, else XDG state, else `~/.local/state/vibe`; schemas in `contracts/`): `v`, `store_hash` (may be empty), `mcp_hash`, `targets` keyed `claude:instructions`, `codex:instructions`, `codex:mcp`, `droid:instructions`, `droid:mcp` with `{hash}`, `owned.{mcp,hooks,plugins,links,created}`. `store_hash` covers `store.json`, `AGENTS.md`, `mcp.json`, the sorted skill list and the binary version; the hook stops when it matches.

**Markers** (D24, owner `store`): `<!-- vibe:store:begin v=1 hash=<h> -->` … `<!-- vibe:store:end -->` in Markdown; `# vibe:store:begin v=1 hash=<h>` … `# vibe:store:end` in TOML. Exact-line match, begin before end; reversed or duplicate markers refuse and leave the file untouched. `upsert` adds one blank line before a block; `remove` drops the block and that blank line only when the block ends the file or the blank line follows another blank line.

**Ownership:** vibe changes only what state records or what sits between its markers. A same-named foreign entry wins (reported kept). Symlinked target files are refused; skills links never replace a real dir or a foreign link. All file writes of a run go through one `vibe_core::fs::commit`; an exclusive lock guards the plan (hook skips when held).

**Secrets:** `import-mcp` swaps likely secrets for placeholders (`${K}` for an env secret of key `K` unless the name has another use in the store, else `${VIBE_<SERVER>_<FIELD>}`); vibe never writes a secret it detects on import (`--keep-literals` opts out). URL userinfo always warns. Literals the user writes are projected as written; `sync`/`install` print `note <server>: literal secret-looking value projected — prefer ${VAR}` once per server.

**Hook output:** one stdout line. For Claude, Codex and Factory it is `{"systemMessage":"vibe: …"}`; for OpenCode and `--harness` absent it is plain text. When Claude MCP is stale (`mcp.json` changed since the last `sync`), the reminder `vibe: Claude MCP changed; run \`vibe sync\`` repeats every session until `vibe sync`; `store status` prints `claude mcp: out of date — run vibe sync` and per-harness `mcp=` (`ok|out-of-date|drifted|absent|n/a`). The hook plans files and links only: `PlanItem::ClaudeCli` items are filtered out and the claude path is `/nonexistent/vibe-hook/claude`.

**Missing CLI:** when `claude` is not installed, the Claude MCP step is skipped with `skip claude: CLI not found — MCP not projected` (exit 0, no ownership recorded). The store stamp is written but the MCP stamp is not advanced, so `status` and the session hook keep reporting Claude MCP out of date until a sync with the CLI succeeds.

**Hook binary:** the recorded path is the `vibe` on `PATH` when it is the running binary (spelling kept), else the running binary's own path. `install` notes a path inside a `/target/` build directory. Paths with `"`, `$`, a backtick, a newline or (off Windows) a backslash are refused.

**Read-only symlinks:** an existing symlinked `store.json`, `AGENTS.md` or `mcp.json` is read as is; `install` then cannot edit a symlinked `store.json` and prints a note instead of adding the toolkit source. A dangling symlink is refused. Relative `skills.sources` entries resolve against the store directory. `store.json` `v`, when present, must be 1 (missing reads as 1).

**Env:** `VIBE_STORE`, `VIBE_STATE`, `VIBE_TOOLKIT` (toolkit skills dir), `VIBE_SPEC_BIN` (peer binary for `vibe spec`).

**Known limits:** a harness switched off in `store.json` keeps its old projection until `vibe uninstall`; `markers::remove` adds a trailing newline to text that lacked one; instructions `None` leaves blocks untouched; Windows skills links skipped; Codex root inline `mcp_servers` gets no vibe MCP block; Claude MCP only via `sync` (hook prints a notice).
