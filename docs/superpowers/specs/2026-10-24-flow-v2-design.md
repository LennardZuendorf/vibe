# flow v2 — Design (F5–F8, F11, skills v2, GSD compat)

**Date:** 2026-10-24 · **Status:** proposed · **Owner:** Lennard (rulings locked 2026-10-24)
**Rows:** F5 (#25), F6 (#26), F7 (#27), F8 (#28), F11 (un-held, issue to file), Track 0 (no row — prose/data sweep), GSD layer (new decision).
**Supersedes:** the F5–F8 scope as originally filed — this arc additionally covers skills v2, the docs/spec route-in, the GSD compatibility layer, and sequencing F12→F24 to v0.4-done.

## Goal

Rebase the entire flow half on one Rust CLI (`vibe-flow`), make the shipped skills terse and unambiguous, route every flow surface through the post-v0.4 spec tree (`docs/spec`, superpowers-first features), adopt GSD compatibility additively, and sequence the remaining v0.4 rows to completion. All state saves **inside the repo**; the global tier is deferred to v0.5.

## Rulings (locked with Lennard, 2026-10-24)

- **FV-R1 Repo-local storage.** Shipped flow data home is `<root>/.agents/flow`; runtime state is `<root>/.vibe/run/`. Nothing reads or writes `~/.config/vibe/` in v0.4; global tier is a v0.5 C-series data move, not a code change.
- **FV-R2 Features are superpowers-only (D48).** `feature.design` = superpowers:brainstorming → `docs/superpowers/specs/*-design.md`; `feature.plan` = superpowers:writing-plans. `docs/spec/features/**` is legacy: readable, never authored, existing policy warn-arms stay. The spec skill owns strategy spec (root specs) only.
- **FV-R3 Skills v2 — one file.** All flow prose, doctrine, and the 13 orders blocks merge into a single `SKILL.md` (shipped at `.agents/flow/SKILL.md`); the six phase files are deleted. Budgets: SKILL.md ≤240 lines/≤12KB, orders block ≤400B (existing pin, new average ≤250B), doctrine ≤700B, ste100 summary ≤220B, template AGENTS.md ≤110 lines. Every `description:` is rewritten to name the smallest work in the user's words.
- **FV-R4 CLI surface.** `vibe-flow state get|set`, `policy check` (`decide` = byte-parity alias), `policy list|render`, `orders`, `doctrine`, `doctor`, `status`, `hook <event>`, `emit`, `init`, `render agents-md [--write|--check]`. `state set` is the writer-is-gate (D27): `next` membership + `--confirm` on gated edges, `idle` always legal, no `--force`.
- **FV-R5 Exit codes.** `policy check` maps refused → 1 (contract-clean); the `decide` alias keeps the oracle's exit-2 refusal. Hooks exit 0 (pass/degraded) or 2 (block, only with a shape-proved single `blocked:`-style line).
- **FV-R6 Cursor.** New path `.vibe/run/flow/state.json`, written atomically (0600, tmp+rename). Read: new path first, legacy `.agents/skills/vibe/state.json` read-only fallback; write always new. No silent auto-migrate; `doctor` reports both.
- **FV-R7 Runtime writes.** `state set` order: validate (pure) → write cursor → append one ledger `transition` event → rewrite payload (seq+1). Ledger/payload failures warn and continue (advisory, D28); cursor failure fails the transition. Ledger: `O_APPEND`, rotate at 5 MB keeping one generation, kinds `transition|hook|signal`.
- **FV-R8 GSD compat is additive-only.** Alias commands in the `/gsd-*` grammar mapping onto `vibe-flow state set`/`status`; `.planning/` is read-only context (never state, never written); disk-strict verification verdicts (`passed|gaps_found|human_needed`) ride the evidence receipt; no GSD state machine, no `--auto`, no milestone layer, no Node (D19). vibe's gates and D12 orders stay authoritative.
- **FV-R9 F11 un-held, parallel.** `vibe-instruct` builds in parallel with F5–F8 (option a). D25 ownership: instruct composes SessionStart/UserPromptSubmit; flow keeps guard/Stop. The JS engine keeps running until F23; the shim flip happens once, at F12.
- **FV-R10 Freeze ordering.** Generation-2 oracle re-freeze is the **last** commit of Track 0 (flow + js halves; the spec half stays gen-1 as vibe-spec's port record). The Rust parity target is post-sweep live behavior; defects found later are expected-divergence pins fixed in Rust only.
- **FV-R11 Parity mechanism.** `vibe-parity` gains additive case fields (`stdin`, `env`, `stderr`, `normalize`, `stamps`, `git`, PATH placeholders) and a new `tests/parity/flow.json` suite over a generated corpus. Illegal-edge/gate refusals are pure-Rust negative tests with machine-derived counts (the bash oracle is toothless there); legal edges are byte-parity.

## Architecture

### Layout v2

```
<root>/.agents/flow/            # shipped data (installed, dereferenced from flow/)
  state-machine.json  content/{policy.json,vibe.default.json,blocks/**}
  SKILL.md                      # skills v2: prose + doctrine + 13 orders blocks
  hooks-fallback/{pre-tool-use-guard,stop-gate}.sh   # frozen bash, permanent tier-4
  scripts/*.sh                  # legacy shims (migration-only; deleted in F23)
  reference/  state.example.json
<root>/.vibe/run/flow/           # runtime (gitignored)
  state.json  ledger.jsonl  last-inject  evidence/{feature-<f>.md,quick.md}
<root>/.vibe/run/inject/flow.json      # payload v1 (F8; instruct reads at F12)
<root>/.vibe/run/instruct/            # F11+: sessions ledger, agents-md fragments
<root>/.vibe/providers/flow.json       # provider manifest v1
```

Source repo authors in `flow/`; `.agents/flow` is a symlink (same pattern as the existing `.agents/skills/vibe`). Root resolution is pinned in core: `--root` → `CLAUDE_PROJECT_DIR` → `VIBE_ROOT` → upward markers (`.vibe/`, `.git`) → cwd, with deleted-env tier tests. All runtime I/O resolves from the project root; plugin/`argv[0]` layouts never steer reads.

### vibe-flow crate (F5–F8)

Oracle-mirrored top-level modules — `machine`, `cursor`, `policy`, `orders`, `doctrine`, `doctor` — each split at the JS engine's existing pure/IO seam; flow-only modules `paths` (the one `FlowPaths` layout owner), `ledger`, `payload`, `emit`, `init`, `agents_md`; `hook/` split into `guard`, `stop`, `inject`, `session`. `machine::check_transition` is pure and is the single enforcement point called by `state set`, `/flow`, and (F7) the ship-edge verdict tooth. The policy matcher is flow's own fnmatch dialect (`*`/`?` cross `/`, boundary-suffix matching, exact-before-glob) with a memoized/DP match and a hostile-pattern time-bound test. Hook events read policy and cursor via `FlowPaths` — no skills-dir dependence. `flow/bin/hook.sh` (POSIX sh, ≤40 lines, shellcheck-clean) implements the five-tier discovery and forwards exit 2 only on a proved block line; every other outcome maps to allow.

### Skills v2 (Track 0)

One `SKILL.md`: `## Contract` (the only prose home for gates, auto-advance, never-hand-edit, precedence, alias note) → `## Delegation protocol` (stated once) → `## Flows: setup/strategy/feature/quick/verify/compound` (procedures only, section-capped) → `<!-- vibe:doctrine -->` → 13 orders blocks. The description frontmatter names every entry trigger in the user's words plus mid-flow triggers. Orders blocks are rewritten for D48 (superpowers-only features, `vibe-flow state set` writer, `.vibe/run/flow/evidence/` receipts, `vibe-spec promote --done` in compound). Machine `reads`/`writes` arrays drop all `docs/spec/features/**` authoring. Every moved pin rides the same commit as the text.

### GSD compatibility layer

`flow/reference/gsd-aliases.json` (alias → state, arg rule, gate requirement, refusal text) + thin per-alias command files that only resolve via the table and relay the CLI's refusal verbatim — gate teeth live in the writer, not the files. `.planning/**` becomes a `block` policy rule (idle is the escape hatch). Receipts gain frontmatter `verdict: passed|gaps_found|human_needed` + `sources:`; `state set feature.compound` requires `verdict == passed` (grandfathering frontmatter-less legacy receipts). `vibe-flow status` reads STATE.md as one advisory line only. `/gsd-autonomous` is a documented refusal alias; milestones/UI-phase/workspace commands have no mapping.

### vibe-instruct crate (F11, parallel)

Pipeline-shaped modules mirroring the frozen oracle exports: `model` (Block/Channel/Trigger), `layers` (shipped → repo-shared → repo-local; global slot reserved, unpopulated), `parse`, `channels`, `placeholders` (flow vocabulary excluded — D20), `compose` (level → edge-slot → event → standing), `budget`, `lint`, `write`. The marker/block grammar lands as vocabulary-free span primitives in `vibe-core::markers` (shared with flow's orders/doctrine; fallback: private module, promoted in one move). Budgets are lint-time errors on authored content (D23 numbers stay); runtime truncation touches provider overflow only. F11 ships CLI-only (`render`, `lint`, `write`, `blocks`, `init`, `doctor` slice); F12 adds the payload reader, session ledger, providers, and the single-commit shim flip.

### Oracles, parity, CI

Track 0 lands as sweep commits (both bash suites + engine suite green in each), then the gen-2 freeze commit (oracle bytes + MANIFEST + pin + README generation log only). `tests/parity/flow.json` + generated corpus `tests/parity/corpus/flow/**` with derived floors (paths per rule, 13 states, one mutation per rule, guard stdin shapes, receipt×git classes). The parity CI job gains Node (js oracles) and a jq-present assert; `rust-test` stays pure Rust. A `port-order` job fails the build on TUI/signals code ahead of `GREEN.hard`. Workspace members 3→4→5; `REVIEWED_PACKAGES` grows by exactly `vibe-flow` and `vibe-instruct` (zero new transitives asserted).

## Tech Stack

Rust 1.88 / edition 2024; deps unchanged (`vibe-core`, `clap`, `serde_json`; no new externals). POSIX sh for the shim; bash 3.2-safe test scripts. Node ≥20.19 only for the js-oracle parity legs (dev/CI-only). Machine, policy, skill text, alias tables: JSON/Markdown data.

## Risks

1. **Byte-parity drift** in reason strings, sorted state lists, `(current: {state})` — mitigated by frozen oracles + generated corpus + pins-as-data.
2. **Exit-2 double meaning** leaking into hooks — hooks never spawn the CLI; shim forwards 2 only on a proved block line.
3. **Two data homes during migration** (`.agents/skills/vibe` vs `.agents/flow`) — one `FlowPaths` struct; `doctor` reports both; legacy is read-only.
4. **Self-stale receipts** — `.vibe/run/**` joins the gate's own-runtime exclusion list; gitignore covers `.vibe/run/`.
5. **Pin explosion** on the writer — teeth are pure-Rust tests with derived counts, not enumerated parity pins.
6. **GSD upstream churn** — aliases map intent not flags; unknown flags warn-and-ignore; the table is versioned data.
7. **Mid-flight skill rewrite changes every orders byte** — one atomic Track 0 version bump; freeze immediately after.
8. **`vibe_core::markers` cross-track dependency** — private-module fallback keeps F11 unblocked.

## Open decisions (defaults Lennard can overturn)

1. Receipts at `.vibe/run/flow/evidence/` (cluster conflict resolved: all flow runtime under one dir). *Default: yes.*
2. `verdict` required for all new receipts vs only when GSD sources are cited. *Default: all new; legacy grandfathered.*
3. `.planning/**` block vs warn. *Default: block; `state set idle` is the escape.*
4. Spec-half oracles re-freeze too? *Default: keep gen-1 for `bash/spec/**`; re-freeze flow + js only.*
5. Source stays in `flow/` with `.agents/flow` symlink. *Default: yes (mirrors `.agents/skills/vibe`).*
6. macOS parity leg now. *Default: ubuntu-only until F23; os-pins stay dormant.*
7. Codex `$gsd-*` alias derivation in v1. *Default: table-only; derivation is mechanical later.*
8. F9/F10 scheduling. *Default: F9 after F8; F10 last (dangling rows).*