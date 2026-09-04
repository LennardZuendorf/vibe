---
type: entrypoint
scope: implementation
covers: feature sequence, binary gates, validation criteria, open decisions
children: []
updated: 2026-09-04
---

# vibe — Implementation Plan

Single-purpose repo: build the self-hosting vibe workflow harness. This plan is current-only — delivered work collapses to notes, no long-horizon backlog. The active arc is v0.4: three tools, one language, no bundle — `spec`, `flow`, and `instruct` rewritten as independent static Rust binaries, sequenced as binary-gated features F1–F24 below.

**Parent specs:** [product.md](product.md), [tech.md](tech.md), [design.md](design.md). Architecture detail, including the v0.4 branch docs, lives in tech.md → `tech-contracts.md`, `tech-rust.md`, `tech-instruct.md`, `tech-spec.md`.

**Delivered features** — all compounded into the root docs; branch spec folders removed (history in the Delivered section below). Truth is the code + tests:

| Feature | Owns | Tests |
|---|---|---|
| spec | Spec tree + templates + `validate.sh` | `spec/tests/run.sh` |
| vibe-flow | State machine + `vibe` skill + D12 orders | `flow/tests/run.sh` |
| agent-instructions | `AGENTS.md` template + marker merge + adapter symlinks | `flow/tests/adapters/run.sh` |
| platform-adapters | Three Claude hooks + `/flow` + `install.sh` settings wiring | `flow/tests/adapters/run.sh` |
| dogfood | Hook/merge/install behaviours exercised end-to-end | scripted + lessons |
| monorepo-split | `spec/`+`flow/` canonical split, compat symlinks, truth sweep | suites + validate + grep evidence |
| install-tooling | `--only`/`--dry-run`/`--uninstall`, `doctor.sh`, `deps.json` | `flow/tests/adapters/run.sh` + `flow/tests/run.sh` |
| release-docs | READMEs, trust rails, logo, stranger eval | CI + eval report |
| flow-mvp | Precedence + delegation contracts, hybrid plan grammar, two edge-keyed gates, evidence-receipt verify tooth | `flow/tests/run.sh` |
| flow-legibility | Self-carrying orders, SessionStart doctrine hook, loop edges, drift-first nudges | `flow/tests/run.sh` |
| install-distribution | One-command `install.sh`, `--with-plugins`, self-hosting plugin + marketplace | `flow/tests/adapters/run.sh` + `spec/tests/run.sh` |
| js-core | `engine/` + `vibe` CLI dispatch; one cursor reader, root resolver, marker grammar | `engine/tests/` + existing suites |
| content-layer | `content/blocks/**`, typed placeholders, `vibe.json` override layer, `vibe render` | `engine/tests/` |
| inject-triggers | Level/edge/event trigger classes, `content/policy.json`, `Stop` predicate cut | `engine/tests/` + `flow/tests/adapters/run.sh` |

---

## Feature Boundaries

Each tool is a closed, deliverable, testable box. Cross-tool coupling is a whole-feature gate (Feature Sequence), never a unit-to-unit edge.

```text
┌─────────────────────────────────────────────────────────────┐
│  memory      spec root tree, AGENTS.md per-owner blocks      │  repo
├─────────────────────────────────────────────────────────────┤
│  contracts   schemas + golden fixtures, run.sh                │  shared
├─────────────────────────────────────────────────────────────┤
│  spec        spec-root tooling, validators, 4 agents          │  ┐
├─────────────────────────────────────────────────────────────┤  │
│  flow        state machine, cursor, guard, TUI, ledger        │  ├ tools
├─────────────────────────────────────────────────────────────┤  │
│  instruct    tiers, blocks, channels, providers, sync         │  ┘
├─────────────────────────────────────────────────────────────┤
│  installer   install.sh: subsets, platforms, uninstall        │  repo
└─────────────────────────────────────────────────────────────┘
```

| Tool | Owns | Does not own | Reads from peers | Writes for peers |
|---|---|---|---|---|
| spec | Spec tree, templates, validate/drift/promote/scan/trace, OpenSpec interchange, 4 agents, its own hooks | Cursor, injection, block composition | Nothing | `--json` answers (`root`, `lessons-for`, `plan`, `feature`); its AGENTS.md block |
| flow | `state-machine.json`, cursor, orders, `policy.json`, guard, sniffer, Stop tooth, evidence, ledger, signals, TUI | Spec content, prompt injection, block composition | spec via `vibe-spec … --json` (soft) | Payload `.vibe/run/inject/flow.json`; provider manifest; its AGENTS.md block |
| instruct | Tiers, blocks, channels, budgets, lints, providers, sources/sync, managed-block composition, `SessionStart`/`UserPromptSubmit` | Any provider's content, the cursor, spec docs | `.vibe/run/inject/*.json`, `.vibe/providers/*.json` | Nothing |

`contracts/` and `install.sh` are shared, not owned by any one tool — `contracts/run.sh` (per-tool CI) and the installer's `--only` flag are the only things that touch all three.

---

## Feature Sequence

Whole-feature delivery order with **binary** gates — a feature starts only when its upstream is `DONE`. Phases: 0 foundations · 1 flow · 2 instruct · 3 spec · 4 distribution + retirement.

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F1 | spec-path-hotfix | spec | Skill-relative path fix in `validate.sh`/`list-specs.sh`/`lessons-for.sh`/`scan-merges.sh`; upward marker search; widened path-guard test | `spec/tests/run.sh` green from a subdirectory and a bare tmp dir | NOT STARTED | — |
| F2 | contracts | all | `contracts/` schemas + goldens (payload, provider manifest, marker grammar, spec JSON) + `run.sh` | Green on empty tools; wired into CI | NOT STARTED | — |
| F3 | oracles-parity | flow | Frozen `oracles/js`+`oracles/bash`, corpus generator with floors, `vibe-parity` differential harness | Oracle byte change reddens CI; planted mutant confirmed via `git diff --numstat` also reddens it | NOT STARTED | — |
| F4 | workspace-core | all | Cargo workspace + `vibe-core` (root, atomic write, markers, config, hook parsing, peer probe, ledger); MSRV; `cargo-deny` | Green matrix (3 OS × stable+MSRV); `cargo metadata` graph assertion | NOT STARTED | F2 |
| F5 | flow-core | flow | Machine/policy in core; cursor at `.vibe/run/flow/state.json`; `state get`/`policy check` | Parity on `state get`/`policy decide` incl. mutated policies | NOT STARTED | F3, F4 |
| F6 | flow-teeth-hooks | flow | Guard + Stop gate in Rust; `hook.sh` + `hooks.json` + frozen fallback; five-tier discovery | Parity on guard corpus + receipt×git corpus; tier matrix with deleted env | NOT STARTED | F5 |
| F7 | machine-teeth | flow | `state set` enforces `next` + `--confirm`; ledger append; `/flow` a thin wrapper | Non-`next` refused, unconfirmed gate refused; parity on legal edges | NOT STARTED | F6 |
| F8 | flow-orders-payload | flow | `orders`/`doctrine`/`doctor` ported; `.vibe/run/inject/flow.json` payload writer; standalone AGENTS.md block | Parity per command; golden payload after transition; PATH-less spec probe degrades | NOT STARTED | F7 |
| F9 | flow-signals-hooks | flow | `vibe-signals` + `RulesProvider`; `PostToolUse(Task)`/`SubagentStop`/`SessionEnd` ledger events | `cargo metadata` proves core has no signals dependency; transcript fixtures → expected signals | NOT STARTED | F8 |
| F10 | flow-tui | flow | `vibe-flow tui` v1 (live pane, transitions, recording); `flows list\|use` | `TestBackend` snapshots; recorded flow round-trips; gated prompts | NOT STARTED | F7, F9 |
| F11 | instruct-core | instruct | Config + four-tier layering; block/channel port; marker writer v1; lints/budgets | Fixture tree renders expected winners; corrupted markers exit 2 | NOT STARTED | F4 |
| F12 | instruct-providers | instruct | Payload reader + session ledger; manifest/command providers; recompose + drift event | Turn-one edge; unchanged seq level-only; standalone == composed bytes | NOT STARTED | F8, F11 |
| F13 | instruct-sources | instruct | dir/git/https sources, pins, cache, lockfile, `sync`, offline path | Unpinned repo-scope refused; offline sync exits 0 | NOT STARTED | F11 |
| F14 | instruct-adapters | instruct | Claude + OpenCode adapters; generated local files; `.claude/rules` emitter; `init`/`doctor` | Hook stdin fixtures → expected stdout; resume re-emits staleness only | NOT STARTED | F12, F13 |
| F15 | instruct-migration | instruct | Split today's Active-Rules block into committed vs global blocks; retire the old marker grammar | This repo's AGENTS.md passes `write --check` + `lint` | NOT STARTED | F14 |
| F16 | spec-core | spec | `vibe-spec` crate; root discovery; spec oracles frozen; `validate`/`list-specs`/`lessons-for`/`scan-merges` ported | Oracle diff + population floor; subdir invocation; exit 0/3 goldens | NOT STARTED | F3, F4 |
| F17 | spec-validators | spec | Substring-bug fixes; strictness config + `--strict`; grammar checks + unified R-ID; `trace` | One landed mutant per bug fails; warn/error/off matrix; OpenSpec samples pass | NOT STARTED | F16 |
| F18 | spec-delta | spec | Header-keyed ADDED/MODIFIED/REMOVED promotion engine; `check-drift` port; `export\|import --openspec` | Golden trees; MODIFIED replaces in place; round-trip byte-identity | NOT STARTED | F17 |
| F19 | spec-hooks-agents | spec | Claude + OpenCode hooks (`validate --changed`/`--file`); four agents registered on both platforms | Changed/unchanged root; `claude plugin validate`; each agent names write path | NOT STARTED | F16 |
| F20 | spec-migrate | spec | `migrate` (population assertion, allow-marker insertion); this repo → `docs/spec/` in one commit | Dry-run table; zero literals after; full suite + validate green | NOT STARTED | F17, F19 |
| F21 | packaging | all | Three plugin dirs (manifests, skills, commands, agents, hooks, bin, opencode); `marketplace.json`; `doctor` peer checks | `claude plugin validate --strict` × 3; shim exits 0 without binary | NOT STARTED | F8, F14, F19 |
| F22 | release-install | all | `cargo-dist` 5 targets, per-tool tags, `SHA256SUMS`; `install.sh` (subsets, platforms, uninstall, cargo fallback) | 5 verified artifacts; containerised subset × platform matrix; uninstall restores | NOT STARTED | F21 |
| F23 | retire-legacy | all | Delete `flow/engine/`, node tests, `build-plugin.sh`, old `install.sh` paths; `vibe-flow migrate` | Full parity green; no `node` anywhere in hooks | NOT STARTED | F15, F20, F22 |
| F24 | doc-truth | all | READMEs (umbrella + per tool), CHANGELOG, root specs truth sweep; drift check extended to three tools | `check-drift` green; stranger eval from a fresh non-git target | NOT STARTED | F23 |

Parallel tracks after F4: flow (F5→F10), instruct (F11→F15, F12 waits for F8), spec (F16→F20). F1 is a same-week hotfix on the current bash tree, independent of the Rust arc.

**Superseded rows.** Rows 15–20 of the pre-v0.4 sequence never started; F1–F24 above absorbs them: machine-teeth → F7, plugin-runtime → F21/F22, spec-js → F16/F17, delegation-redirect → F9, spec-delta → F18, doc-truth → F24.

**Active focus.** F1 (spec-path-hotfix) ships this week on the current bash tree — it is not part of the Rust arc. F2 (contracts), F3 (oracles-parity), and F4 (workspace-core) run in parallel; every other feature waits on F4, and F12 additionally waits on F8.

---

## Critical Architecture Decisions

### Decided

- **D8 — Lessons split.** Lesson format lives in the spec skill bundle; read-on-entry and tag-scan live in flow.
- **D12 — Orders in the skill.** Per-turn orders are authored once in the linked skill's phase file, not duplicated in the machine.
- **D13 — Carrier split.** Plugin carries the runtime, repo carries the memory. (superseded by D18 — three tools, no umbrella plugin.)
- **D14 — One JS engine.** Deterministic machinery is one Node engine; hooks are thin shims over it. (superseded by D19 — Rust, static, zero runtime deps.)
- **D15 — Content is data.** Every injected sentence is an authored block; `policy.json` is the sole write-invariant source.
- **D16 — Injection is trigger-classed.** Payload splits level/edge/event with per-channel line budgets linted in CI.
- **D17 — The gate lives in the writer.** `vibe state set` refuses illegal and unconfirmed gated edges.
- **D18 — Three tools, no umbrella.** `vibe-spec`, `vibe-flow`, `vibe-instruct`; no umbrella binary — the installer is the umbrella. Supersedes D13.
- **D19 — Rust, static, zero runtime deps.** Cargo workspace with `vibe-core` shared internals; no tool crate depends on another; deps allowlisted. Supersedes D14.
- **D20 — Contracts are the only coupling.** Versioned schemas + goldens in `contracts/`; tools exchange files under `.vibe/` plus one soft probe call, never imports.
- **D21 — `.vibe/` is the per-repo directory.** Committed config and blocks, git-excluded `local/`, gitignored `run/` (cursor, ledger, payloads); global tier under `~/.config/vibe/`.
- **D22 — Provider payload contract.** flow writes `.vibe/run/inject/<provider>.json` (level/edge/session/events); instruct also reads manifest-declared command providers.
- **D23 — Budgets keep today's numbers.** `user-prompt.level` 2 lines, `.edge` 15, `.event` 10, `session-start` 15, `agents-md` 80; lints fail an over-budget render.
- **D24 — Managed-block grammar v1, per owner.** One marker grammar in `vibe-core`; each tool writes its own block; instruct recomposes in order. Supersedes the old marker grammars.
- **D25 — Hook ownership.** instruct owns SessionStart/UserPromptSubmit; flow owns guard/sniffer/Stop/ledger hooks; spec owns its own Stop/PostToolUse validation.
- **D26 — Hook shim and discovery.** Each plugin ships a tiny `bin/hook.sh`; five-tier binary discovery ending in a frozen bash fallback, never a download.
- **D27 — The writer is the gate.** `vibe-flow state set` enforces `next` and `--confirm`; `/flow` and the TUI call the same check. No `--force`.
- **D28 — Ledger + advisory signals.** Append-only `ledger.jsonl`; `vibe-signals` cannot be depended on by core, so guard and gate structurally cannot read it.
- **D29 — TUI and recorded workflows.** `vibe-flow tui`: live pane, gated transitions, recording a linear workflow subset to the global config tier.
- **D30 — Port with frozen oracles, hard units first.** JS and bash oracles frozen; differential parity harness; JS deleted only when every unit is parity-green.
- **D31 — Instruct tiers.** global < repo-shared < repo-local < session-runtime; only repo-shared may compose the committed `agents-md` block.
- **D32 — Sources and sync.** Pinned dir/git/https sources per tool; `vibe-instruct sync` fetches and locks; session start stays offline and fast.
- **D33 — Spec root and discovery.** Default `docs/spec/`, legacy `.spec/`; `--root` → env → config → default → parent search → exit 3.
- **D34 — OpenSpec: grammar-compatible, layout-native.** Adopts the Requirement/Scenario/delta grammar for interchange; keeps vibe's own root and document model.
- **D35 — Strictness ratchet.** Every new check ships warn; `--strict` promotes to error in CI; a check moves to error only after migration.
- **D36 — Spec subagents are real platform agents.** Four spec agents with explicit frontmatter and write paths, registered on both Claude Code and OpenCode.
- **D37 — Packaging: the tool directory is the plugin.** Each of `spec/`, `flow/`, `instruct/` is a complete plugin; no build step, no generated payload.
- **D38 — One installer, three release trains.** `install.sh` detects the platform, fetches signed per-tool binaries from GitHub Releases, falls back to `cargo install`.
- **D39 — Doctrine stays.** Enforce in code, explain in prose, never both; data over prose; degrade gracefully; committed memory readable by any agent or none.

### Resolved

- [x] **OPEN-2 — Skill count.** Seven `vibe-*` shims consolidated into one `vibe` skill (2026-06-29).
- [x] **OPEN-3 — Install mode.** `install.sh` copies core + Claude adapter, merges `AGENTS.md`, supports partial/dry-run/uninstall.
- [x] **OPEN-4/OPEN-7 — Hook strictness.** Shipped warn-first; only the pre-existing hard blocks deny.
- [x] **OPEN-6 — Skill degradation.** `check-skills.sh` warns on unverifiable delegates and degrades to inline orders.

---

## Spec vs Implementation

The whole v0.4 arc — F1 through F24 — is spec-ahead-of-code: nothing below has started. The tree is still v0.3: rows 12–14 (js-core, content-layer, inject-triggers) are delivered, and the old rows 15–20 (machine-teeth, plugin-runtime, spec-js, delegation-redirect, spec-delta, doc-truth) were never started before this rewrite superseded them. The reproduced spec-path bug (F1) is live in the tree today — `validate.sh`, the spec subagents, and `feature.md` hardcode `.agents/skills/spec/scripts/*.sh`, so a plugin install or any cwd other than the repo root breaks them.

---

## Delivered (history)

Cleansed notes for shipped work — detail lives in live surfaces, not this plan.

- **spec** — four-layer model, warn-first `validate.sh`, feature-authoring flow. DONE.
- **vibe-flow** — 13-state machine, six scripts, D12 orders via `orders.sh`. DONE.
- **agent-instructions** — `AGENTS.md` template, marker merge, adapter symlinks. DONE.
- **platform-adapters** — three hooks as shims + `install.sh` settings wiring. DONE.
- **dogfood** — hook/merge/install behaviours scripted end-to-end. DONE.
- **monorepo-split** — canonical halves at `spec/`+`flow/`, compat symlinks, truth sweep. DONE (2026-07-03).
- **install-tooling** — `--only`/`--dry-run`/`--uninstall`, `doctor.sh`, `deps.json`. DONE (2026-07-03).
- **release-docs** — READMEs, trust rails, logo, stranger eval. DONE (2026-07-03).
- **flow-mvp** — precedence contract, two edge-keyed gates, evidence-receipt tooth. DONE (2026-07-08).
- **flow-legibility** — self-carrying orders, SessionStart doctrine hook, drift-first nudges. DONE (2026-07-18).
- **install-distribution** — one-command install, self-hosting plugin + marketplace. DONE.
- **js-core** — `engine/` + `vibe` CLI dispatch, one cursor reader, byte-identical port. DONE.
- **content-layer** — `content/blocks/**`, typed placeholders, `vibe render` with budgets. DONE.
- **inject-triggers** — level/edge/event trigger classes, `policy.json`, `Stop` predicate cut. DONE.
