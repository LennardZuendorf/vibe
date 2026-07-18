# vibe — install & distribution overhaul (design)

**Date:** 2026-07-18
**Branch:** `claude/vibe-install-setup-2cknby`
**Status:** approved direction (brainstorm → design). Supersedes the deferred
row-13 (vibe-plugin) / row-14 (stack-installer) sketches with a leaner shape.

## Problem

Three usability gaps, confirmed against the tree:

1. **Skill script paths are install-model-specific.** Skills reference their
   bundled scripts as repo-root-absolute (`bash .agents/skills/spec/scripts/validate.sh`)
   or enumerate a `~/.agents/…` global variant. These only resolve in the
   source repo (via the `.agents/skills/spec → spec` symlink). Under a global or
   plugin layout the skill root is elsewhere and the command breaks. The
   agentskills.io convention — and the same files' own markdown links
   (`[scripts/validate.sh](scripts/validate.sh)`) — is skill-relative. The most
   load-bearing offender is `flow/reference/templates/AGENTS.md:71`, merged
   verbatim into every installed project.
2. **`install.sh` is not a one-command experience.** It *requires* a target-repo
   argument; bare `bash install.sh` errors out. There is no local-vs-global
   choice, no companion-tool setup, and no plugin/marketplace path — it wires
   hooks straight into the target's `.claude/settings.json`.
3. **Distribution under-delivers.** No per-user "install once, works in every
   repo" path. Companion tools (superpowers, feature-dev) are only *declared* in
   `deps.json` for `doctor.sh` to warn about, never installed.

## Decisions (owner-confirmed)

- **Scope:** everything in one pass — including the actual per-user Claude Code
  plugin + marketplace.
- **`bash install.sh` (no args):** auto-detect the current git repo, then
  interactively ask **local (this repo)** vs **global (per-user plugin)**.
  Non-interactive flags cover CI.
- **Companion tools:** installer offers **superpowers + feature-dev** (opt-in,
  `--with-plugins`), graceful-degrade. **caveman is NOT a plugin** — it becomes a
  one-line "**caveman style** — concise, high-signal, minimal fluff" preference
  baked into vibe's injected instructions. No elaborate `stack` subsystem.
- **Path sweep:** full sweep of hard-coded `.agents/skills/**/*.sh` references in
  skill prose/templates → skill-relative, plus a CI guard against regression.

## Architecture — hybrid distribution

Two carriers, one entry point.

| Carrier | Delivers | Scope |
|---|---|---|
| **Plugin** (`claude plugin install vibe@vibe`) | `vibe`+`spec` skills, `/flow`, the four hooks — self-contained under `${CLAUDE_PLUGIN_ROOT}`, hooks self-detect vibe repos | per-user, every repo |
| **`install.sh`** | per-repo state: `.spec/` seed, cursor, `.gitignore` stanzas, AGENTS.md merge; platform-neutral `.agents/` for Codex/Warp | one repo |

`install.sh` is the single interactive entry point: **local** does today's per-repo
copy+wire; **global** drives the plugin install via the `claude` CLI, then offers
the per-repo seed.

### Plugin packaging

Claude Code plugins now bundle `skills/` directly (the 2026-06-18 "can't bundle
skills" lesson is **obsolete**), but **no `../` paths** — everything resolves
under the plugin root via `${CLAUDE_PLUGIN_ROOT}`.

- Repo is its own marketplace: `.claude-plugin/marketplace.json` +
  `.claude-plugin/plugin.json` at root → `claude plugin marketplace add
  LennardZuendorf/vibe && claude plugin install vibe@vibe`.
- `build-plugin.sh` assembles bundled `skills/{spec,vibe}` from the canonical
  `spec/` + `flow/` (tests/contributor-docs stripped) so there is a single source
  of truth and no committed drift. (Committed vs build-time-only assembled tree:
  decided during planning; default is a committed `plugin/` tree refreshed by the
  build script and drift-checked in CI.)
- `plugin.json`: `"skills": "./skills/"`, `"commands": ["./commands/flow.md"]`,
  `"hooks": "./hooks/hooks.json"`. Hook commands use `${CLAUDE_PLUGIN_ROOT}`.
- Hooks **self-detect**: no `.spec/` and no `.agents/skills/vibe/state.json`
  (and no machine) → fast `exit 0`, so they are silent in non-vibe repos and do
  not double-fire alongside a project-scoped install.

### Path fix (item 1)

- Skill prose/templates: `bash .agents/skills/spec/scripts/validate.sh` →
  skill-relative `scripts/validate.sh`, matching the markdown-link form the same
  files already use. Drop the `~/.agents/…` global enumerations.
- Scripts that must locate the skills dir already self-locate via upward marker
  search — unchanged.
- Runtime hooks: `${CLAUDE_PLUGIN_ROOT}` in the plugin; `$CLAUDE_PROJECT_DIR/.claude/hooks/…`
  in the vendored install (as today).
- **CI guard:** a test fails on any new hard-coded `.agents/skills/**/*.sh`
  invocation in skill prose/templates.

### Companion tools (item 4)

- `install.sh --with-plugins` (or interactive prompt, default off) runs, when the
  `claude` CLI is present: `claude plugin marketplace add obra/superpowers` +
  `claude plugin install superpowers@…`, and the feature-dev equivalent. Absent
  CLI/network → warn + skip (never hard-fail); keeps `deps.json`/`doctor.sh`'s
  degrade contract.
- **caveman-style note:** a single frozen line added to the injected doctrine /
  instructions (e.g. "Output: caveman style — concise, high-signal, minimal
  fluff; skip filler."). No install, no dependency.

### INIT / injection / usability (items 3)

- `doctor.sh` gains an **instruction-coverage** check: pass if the AGENTS.md
  managed block is present **or** a SessionStart hook is wired **or** the plugin
  is installed.
- Interactive installer prints a "what I did / what's next" summary.
- `merge-agents.sh` is solid — unchanged.

### README (item 5)

Lead with the one-command install and the plugin path; reconcile the now-false
"no plugin registration required" claims (`README.md:70`, `flow/README.md:29`);
document local-vs-global, `--with-plugins`, and the caveman-style note.

## Verification

- **Fully testable here:** the path sweep + CI guard; `install.sh` (local) into a
  fresh `mktemp -d` with no `.git`/`.spec` (the "stranger" target from the
  release-docs lesson); `merge-agents.sh`; `doctor.sh`; manifest JSON validity;
  `build-plugin.sh` output; existing suites (`spec/tests`, `flow/tests`,
  `flow/tests/adapters`) + `validate.sh` + `check-drift.sh`.
- **Partial only (flagged, not claimed):** a real per-user marketplace
  `/plugin install vibe` round-trip — the `claude` CLI is present (v2.1.211) but
  the sandbox can't do the full network/marketplace install. Structure, manifest
  parse, and dry-runs are verified; anything not end-to-end confirmed is called
  out.

## Phased implementation

1. **Path sweep + CI parity guard** (foundation; de-risks the plugin).
2. **`install.sh` single-command interactive** — local/global, `--with-plugins`,
   caveman-style note; fresh-target eval.
3. **Plugin + marketplace** — manifests, `build-plugin.sh`, self-detecting hooks.
4. **`doctor.sh` instruction-coverage + installer summary.**
5. **README rewrite.**
6. **Tests + stranger eval + `validate.sh`/`check-drift.sh`; commit; push.**

Each phase is independently valuable and verified before the next.

## Non-goals

- OpenSpec adoption, spec-delta engine, delegation-redirect hook (separate plan
  rows).
- A standalone multi-tool `stack` command (folded into `--with-plugins`).
- Publishing to a third-party marketplace (the repo is its own marketplace).
