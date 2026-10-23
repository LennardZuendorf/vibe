---
type: tech
applies:
  - "crates/vibe-spec/**"
  - ".opencode/**"
  - ".git/hooks/**"
updated: 2026-10-19
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

Known limits:
- Fresh installs still seed the legacy root through `setup.sh` until F22, because no binary ships before then. The shipped prose names both roots and the `validate.sh` fallback.
- `install.sh` does not wire spec hooks into targets (P5). They reach targets through the plugin.
- The OpenCode shim has not run against live OpenCode.
- A hooks dir outside the project (a linked worktree or an external `core.hooksPath`) skips the symlink walk.

## Commands (v1)

`root`, `init`, `validate [--changed|--file|--strict|--format text|json|hook]`,
`check-drift [--pre-merge] [--json]`,
`promote <feature> [--dry-run] [--target <file>] [--done] [--base <ref>] [--json]`,
`scan-merges`, `lessons-for <tags>`, `list-specs`, `trace`,
`migrate --to docs/spec [--layout layered] [--dry-run]`,
`export|import --openspec <dir>`, `hook {session-start,guard,validate}`,
`setup [--git-hooks|--opencode|--agents-md]`, `doctor`.
