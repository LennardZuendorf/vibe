---
type: tech
applies:
  - "crates/vibe-spec/**"
  - ".opencode/**"
  - ".git/hooks/**"
updated: 2026-11-12
---

# Setup

Setup: `vibe-spec setup [--git-hooks] [--opencode] [--agents-md] [--json]`
needs at least one flag and no spec root. Every file it writes carries a
marker. It refuses to replace a file without the marker, replaces its own
idempotently, and restores on failure. `--git-hooks` writes `pre-commit`
(`# vibe-spec pre-commit v1`) under `git rev-parse --git-path hooks`. The hook
runs `validate --changed --strict` with `VIBE_SPEC_STRICTNESS='{"SF29":"error"}'`
and is the only hook that exits 1. `VIBE_SPEC_STRICTNESS` is a JSON object in
the `strictness` schema, merged over the config. `--opencode` writes
`.opencode/agents/spec-promoter.md`, `.opencode/agents/spec-health.md` and
`.opencode/plugins/vibe-spec.js`, a `tool.execute.after` shim that runs `hook
validate`. `--agents-md` adds or refreshes the router block between
`<!-- vibe-spec:router:start -->` and `<!-- vibe-spec:router:end -->`.

`init [--to <dir>] [--json]` seeds `README.md`, `AGENTS.md`, `product.md`,
`tech.md`, `design.md`, `plan.md` and `lessons.md` from compiled-in templates
under `--to`, else the discovered root, else `<project>/docs/spec`. It never
overwrites: an existing file prints `SKIP: <rel>`. It is a redesign, not a port of `setup.sh`.

Central `vibe spec` and compatibility `vibe-spec` share these commands. Project
`init --mode repo-first|global-first` and `attach [--main]` register storage and
checkout identity; they do not install a harness or migrate existing spec modes.
Global `branch init NAME --from REF` copies an exact stored base without code checkout.
Its result uses only a matching validated caller anchor; otherwise code context
is unavailable. It never selects another clone/main. Normal Context rules remain.
`context`, `document list|read|write|remove`, `snapshot`, `reconcile` and record
`status|read|resolve` use explicit context and opaque expected revision tokens.
Write accepts `--input FILE` or stdin when omitted. Exact reconcile plans use
`--input FILE|- --expect TOKEN`; resolution adds `--expect-resolution TOKEN`.
Init/attach CLI `--expect` binds registry metadata; branch init additionally
accepts `--expect-project`. CLI omission captures once; MCP requires both relevant
tokens. Context tokens, metadata tokens and Git OIDs are not interchangeable.
Management context selectors must follow the command/subcommand; prefix forms
refuse before effects. Existing operations/MCP use prefix context selectors.
Prefix `--root` keeps legacy behavior; hooks reject the new context selectors.
Local launch: `vibe spec --project ID mcp --stdio`. This does not install MCP
harness config or make install.sh distribute a released central binary.

Known limits:
- Project and global installs leave spec creation to the user. Default installs ship no binary. Global `--build-spec-bin` can build and install it; without it, bundled scripts validate flat legacy roots only. See [Installer](../../areas/installer.md).
- `install.sh` does not wire spec hooks into targets (P5). They reach targets through the plugin.
- The OpenCode shim has not run against live OpenCode.
- A hooks dir outside the project (a linked worktree or an external `core.hooksPath`) skips the symlink walk.

## Commands

Legacy groups: `root`, `init`, `validate`, `check-drift`, `promote`, `scan-merges`,
`lessons-for`, `list-specs`, `trace`, `migrate --to docs/spec`,
`export|import --openspec DIR`, `hook {session-start,guard,validate}`, `setup`.
Detailed flags and the storage walkthrough live in the spec README.
