# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

A hardening wave after a full audit of the harness: verified bug fixes, mechanical
compound enforcement, a warnings relay, an honest-docs truth sweep, and a
fresh-install stranger-eval pass that hardens the no-git / no-jq / no-awk targets.

### Added

- **Compound-enforcement gate** — `spec/scripts/check-drift.sh` (CI-wired after
  `validate.sh`) fails when a merged feature has no row in the root `.spec/plan.md`
  or leaves `NOT STARTED` units in a live feature plan, and errors on hand-written
  test-assertion counts (counts must come from the runner, never be typed by hand).
- **flow-mvp feature (the personal operating layer)** — precedence + delegation
  contract blocks, the hybrid plan grammar, auto-advance with two edge-keyed gates,
  the `quick.compound` lessons path, two impl modes, caveman demoted to vocabulary,
  and the first promoted `Stop` predicate: the evidence-receipt verify tooth. Merged
  in PR #14; its overdue compound was completed in this wave.
- **Warnings relay** — flow warnings (auto-advance nudges, exit-predicate smells)
  are surfaced back at the next prompt instead of being lost mid-turn.

### Simplified

- **Caveman levels → one style note** — the per-state `caveman` levels
  (`lite`/`full`/`ultra`), the `caveman_levels`/`safety_carveouts` machine blocks,
  and the `check-skills.sh caveman` fallback are gone. A single top-level `style`
  note in `state-machine.json` now governs output density for every state:
  compress receipts and summaries, but security warnings and irreversible-action
  confirmations stay in full prose and reasoning depth is never reduced.
- **`amend` folded into precedence** — the `amend` modifier state, the `modifiers`
  array, and `amend.md` are removed. A scope edit is not a state: edit within the
  current state's write surface and stay put; `set-state.sh idle` still aborts.
- **`strategy.compound` / `quick.compound` → conditional steps** — the two dead
  compound states are removed. The optional durable-lesson step now runs inline at
  the end of `strategy.spec` and `quick.verify` (append to `.spec/lessons.md` +
  `regen-active-rules.sh` before going idle). `feature.compound` is unchanged.
  The lessons.md write guard now allows those two flow-end states. The machine
  drops from 16 to 13 state entries (12 non-idle states).
- **Plan-template grammar relaxed** — the per-unit **Interfaces** and **Steps**
  blocks in `feature-plan.md` are marked handover-mode-only; interactive impl may
  omit them and drive units from the core blocks (Goal / Requirements / Files /
  Test scenarios / Verification).

### Fixed

- Verified bugs across the flow engine, installer, and spec skill (see the branch
  review).
- **Stranger-eval hardening (fresh no-git / no-jq / no-awk targets)** — a wave of
  fixes reproduced by installing into a bare `mktemp -d` and running a docs-only
  agent against it:
  - `orders.sh` falls back to a jq-free cursor reader, so per-turn orders still
    route to the current state on a target without jq;
  - `doctor.sh` reports the flow cursor honestly in that same no-jq case;
  - `set-state.sh`'s jq-free state validation rejects machine meta-keys (`style`,
    `version`) instead of accepting them as states;
  - the SF12 frontmatter check drops a GNU-awk-ism for portable awk;
  - `install.sh --uninstall` inverts the copy **per file** for the shared
    `.agents/skills/{spec,vibe}` dirs — user files dropped inside survive, the
    shipped payload is removed, and emptied dirs are pruned — instead of a blanket
    `rm -rf` of the shared dir;
  - uninstall unwires `.claude/settings.json` **before** deleting the hook scripts
    it references; on a no-jq target it leaves the scripts in place (no dead refs)
    and prints the manual step;
  - `merge-agents.sh` locates markers with `grep -n` + `sed` instead of awk, so
    unmerge strips both managed blocks on an awk-less target;
  - unmerge also removes the stranded vibe-branded title line, so a vibe-created
    `AGENTS.md` leaves no orphan behind.
- **Doc truth sweep** — retired the plugin-era claims (`.claude-plugin/plugin.json`
  + `hooks.json`); the live Claude wiring is `.claude/settings.json`. Corrected
  stale `tests/{spec,flow,adapters}/run.sh` paths to the split layout
  (`spec/tests/`, `flow/tests/`, `flow/tests/adapters/`). Removed hand-written
  assertion counts from the README and specs. Marked the never-shipped
  `examples/todo-api` sample and the manual gh-metadata step as deferred. Retired a
  drifting hand-maintained state table in favour of pointing at the canonical
  `flow/state-machine.json`.

## [0.1.0] - 2026-07-03

First public release of vibe — a self-hosting bash/Markdown/JSON workflow harness.

### Added

- **Spec framework** — the `.spec/` four-layer memory model (product, tech,
  design, plan, plus lessons and branch-scoped `features/`), the bundled `spec`
  skill, and `validate.sh` consistency checks.
- **Vibe flow** — a 15-state workflow state machine with the `vibe` skill
  router, per-turn order injection (decisions D8–D12), and graceful degradation
  when optional skills are absent.
- **AGENTS.md provisioning** — instruction template, marker-bounded merge that
  preserves user content, and adapter symlinks (`CLAUDE.md`, `WARP.md`).
- **Claude Code adapter** — a plugin plus three hooks: inject
  (UserPromptSubmit), guard (PreToolUse), and gate (Stop).
- **Monorepo split** — canonical `spec/` and `flow/` halves with compatibility
  symlinks under `.agents/skills/`.
- **Install tooling** — `install.sh` with `--only`, `--dry-run`, and
  `--uninstall`, a `doctor.sh` health check, and a `deps.json` manifest.

[Unreleased]: https://github.com/LennardZuendorf/vibe/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/LennardZuendorf/vibe/releases/tag/v0.1.0
