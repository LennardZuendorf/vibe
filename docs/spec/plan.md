---
type: hub
scope: implementation
covers: feature sequence, binary gates, validation criteria, open decisions
children: []
updated: 2026-10-19
---
<!-- vibe-spec: allow SF31 -->

# vibe — Implementation Plan

Single-purpose repo: build the self-hosting vibe workflow harness. This plan is current-only — delivered work collapses to notes, no long-horizon backlog. The active arc is v0.4: three tools, one language, no bundle — `spec`, `flow`, and `instruct` rewritten as independent static Rust binaries, sequenced as binary-gated features F1–F25 below. The queued arc is v0.5: the `vibe` CLI front door — worktree lifecycle, global spec management, global blocks (C1–C6, own section below).

**Parent specs:** [product.md](product.md), [tech.md](tech.md), [design.md](design.md). Architecture detail, including the v0.4 branch docs and the v0.5 CLI doc, lives in tech.md → `tech-contracts.md`, `tech-rust.md`, `tech-instruct.md`, `tech-spec.md`, `tech-cli.md`.

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
│  spec        spec-root tooling, validators, 2 agents          │  ┐
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
| spec | Spec tree, templates, validate/drift/promote/scan/trace, OpenSpec interchange, 2 agents, its own hooks | Cursor, injection, block composition | Nothing | `--json` answers (`root`, `lessons-for`, `plan`, `feature`); its AGENTS.md block |
| flow | `state-machine.json`, cursor, orders, `policy.json`, guard, sniffer, Stop tooth, evidence, ledger, signals, TUI | Spec content, prompt injection, block composition | spec via `vibe-spec … --json` (soft) | Payload `.vibe/run/inject/flow.json`; provider manifest; its AGENTS.md block |
| instruct | Tiers, blocks, channels, budgets, lints, providers, sources/sync, managed-block composition, `SessionStart`/`UserPromptSubmit` | Any provider's content, the cursor, spec docs | `.vibe/run/inject/*.json`, `.vibe/providers/*.json` | Nothing |

`contracts/` and `install.sh` are shared, not owned by any one tool — `contracts/run.sh` (per-tool CI) and the installer's `--only` flag are the only things that touch all three.

---

## Feature Sequence

Whole-feature delivery order with **binary** gates — a feature starts only when its upstream is `DONE`. Phases: 0 foundations · 1 flow · 2 instruct · 3 spec · 4 distribution + retirement.

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F1 | spec-path-hotfix | spec | Skill-relative path fix in `validate.sh`/`list-specs.sh`/`lessons-for.sh`/`scan-merges.sh`; upward marker search; widened path-guard test | `spec/tests/run.sh` green from a subdirectory and a bare tmp dir | DONE | — |
| F2 | contracts | spec | `contracts/` Spec JSON v1 schema + goldens (`root`, `lessons-for`, `plan`, `feature`), exit-code contract, `run.sh`; payload, provider manifest and marker grammar move to their tools' rows | Green on empty tools; wired into CI | DONE | — |
| F3 | oracles-parity | all | Frozen `oracles/js`+`oracles/bash` + `MANIFEST.sha256`; generic `vibe-parity` differential harness; spec corpus classes with floors (flow classes land with F5) | Oracle byte change reddens CI; planted mutant confirmed via `git diff --numstat` also reddens it | DONE | — |
| F4 | workspace-core | all | Cargo workspace with `vibe-core`, `vibe-spec`, `vibe-parity` only (flow/instruct/signals crates arrive with their rows); MSRV 1.88; `cargo-deny` allowlist | Green matrix (3 OS × stable+MSRV); `cargo metadata` graph assertion | DONE | F2 |
| F5 | flow-core | flow | Machine/policy in core; cursor at `.vibe/run/flow/state.json`; `state get`/`policy check` | Parity on `state get`/`policy decide` incl. mutated policies | NOT STARTED | F3, F4 |
| F6 | flow-teeth-hooks | flow | Guard + Stop gate in Rust; `hook.sh` + `hooks.json` + frozen fallback; five-tier discovery | Parity on guard corpus + receipt×git corpus; tier matrix with deleted env | NOT STARTED | F5 |
| F7 | machine-teeth | flow | `state set` enforces `next` + `--confirm`; ledger append; `/flow` a thin wrapper | Non-`next` refused, unconfirmed gate refused; parity on legal edges | NOT STARTED | F6 |
| F8 | flow-orders-payload | flow | `orders`/`doctrine`/`doctor` ported; `.vibe/run/inject/flow.json` payload writer; standalone AGENTS.md block | Parity per command; golden payload after transition; PATH-less spec probe degrades | NOT STARTED | F7 |
| F9 | flow-signals-hooks | flow | `vibe-signals` + `RulesProvider`; `PostToolUse(Task)`/`SubagentStop`/`SessionEnd` ledger events | `cargo metadata` proves core has no signals dependency; transcript fixtures → expected signals | NOT STARTED (held) | F8 |
| F10 | flow-tui | flow | `vibe-flow tui` v1 (live pane, transitions, recording); `flows list\|use` | `TestBackend` snapshots; recorded flow round-trips; gated prompts | NOT STARTED (held) | F7, F9 |
| F11 | instruct-core | instruct | Config + four-tier layering; block/channel port; marker writer v1; lints/budgets | Fixture tree renders expected winners; corrupted markers exit 2 | NOT STARTED (held) | F4 |
| F12 | instruct-providers | instruct | Payload reader + session ledger; manifest/command providers; recompose + drift event | Turn-one edge; unchanged seq level-only; standalone == composed bytes | NOT STARTED | F8, F11 |
| F13 | instruct-sources | instruct | dir/git/https sources, pins, cache, lockfile, `sync`, offline path | Unpinned repo-scope refused; offline sync exits 0 | NOT STARTED | F11 |
| F14 | instruct-adapters | instruct | Claude + OpenCode adapters; generated local files; `.claude/rules` emitter; `init`/`doctor` | Hook stdin fixtures → expected stdout; resume re-emits staleness only | NOT STARTED | F12, F13 |
| F15 | instruct-migration | instruct | Split today's Active-Rules block into committed vs global blocks; retire the old marker grammar | This repo's AGENTS.md passes `write --check` + `lint` | NOT STARTED | F14 |
| F16 | spec-core | spec | `vibe-spec` crate; root discovery; spec oracles frozen; `validate`/`list-specs`/`lessons-for`/`scan-merges` ported | Oracle diff + population floor; subdir invocation; exit 0/3 goldens | DONE | F3, F4 |
| F17 | spec-validators | spec | Substring-bug fixes; strictness config + `--strict`; grammar checks + unified R-ID; `trace`; layered-layout checks (D44: allowlist, budgets, link resolution, unique names, typed frontmatter, `code:` existence, hub reachability); superpowers artifact name + `**Spec:**` link checks | One landed mutant per bug fails; warn/error/off matrix; OpenSpec samples pass; layered fixture tree passes, each mutated rule fails | DONE | F16 |
| F18 | spec-delta | spec | Header-keyed ADDED/MODIFIED/REMOVED promotion engine; `promote.sh` port incl. superpowers promotion checklist and `promote <feature> --done` (D46); `check-drift` port with `--pre-merge`; `export\|import --openspec` | Golden trees; MODIFIED replaces in place; round-trip byte-identity; `--done` refuses on unchanged `docs/spec/` | DONE | F17 |
| F19 | spec-hooks-agents | spec | Validate hooks (`validate --changed`/`--file`, exit 0); `using-spec` SessionStart router; blocking `PreToolUse` tooth (D47); agents `spec-promoter` + `spec-health` registered on both platforms; `spec-interviewer`/`spec-tracer` retired | Changed/unchanged root; tooth blocks unskilled edit and a discriminating test fails on `exit 0`; captured subagent payload fixture; `claude plugin validate` | IN PROGRESS — local gate green; CI pending push | F16 |
| F20 | spec-migrate | spec | `setup.sh` port (`init`); `migrate --to docs/spec` (population assertion, allow-markers) + `--layout layered`; this repo → layered `docs/spec/` in one commit; `docs/spec/archive/` deleted | Dry-run table; zero literals after; full suite + `validate --strict` green | NOT STARTED | F17, F19 |
| F21 | packaging | all | Three plugin dirs (manifests, skills, commands, agents, hooks, bin, opencode); `marketplace.json`; `doctor` peer checks — spec slice in the spec program | `claude plugin validate --strict` × 3; shim exits 0 without binary | NOT STARTED | F8, F14, F19 |
| F22 | release-install | all | `cargo-dist` 5 targets, per-tool tags, `SHA256SUMS`; `install.sh` (subsets, platforms, uninstall, cargo fallback) — spec slice in the spec program | 5 verified artifacts; containerised subset × platform matrix; uninstall restores | NOT STARTED | F21 |
| F23 | retire-legacy | all | Delete `flow/engine/`, node tests, `build-plugin.sh`, old `install.sh` paths; `vibe-flow migrate`; spec slice deletes `spec/scripts/*.sh` | Full parity green; no `node` anywhere in hooks | NOT STARTED | F15, F20, F22 |
| F24 | doc-truth | all | READMEs (umbrella + per tool), CHANGELOG, root specs truth sweep; drift check extended to three tools | `check-drift` green; stranger eval from a fresh non-git target | NOT STARTED | F23 |
| F25 | spec-superpowers-default | spec | Assertive, trigger-only spec skill rewrite (pressure-tested); `using-spec` router text; superpowers-first delegate lines in `flow/`; `superpowers-frozen` write-policy rule; `docs/superpowers/` un-ignored | Pressure baseline 100 % with and without superpowers; flow + spec suites green | DONE | — |

Parallel tracks after F4: flow (F5→F10), instruct (F11→F15, F12 waits for F8), spec (F16→F20). F1 is a same-week hotfix on the current bash tree, independent of the Rust arc.

**Issue catalog (GitHub).** The F-rows and issues are 1:1; held rows have no issue yet (their architecture design is in flight and files its own issues). #22 is the v0.4 tracking issue; #18 (repo/branch-scoped cursor) is absorbed by this arc — the cursor path lands in F5, the plugin-carried runtime in F21; #12 (plugin never registered, closed) is fixed by design in F21/F22. Each phase above is a GitHub Milestone (`v0.4 Phase 0–4`, see [docs/roadmap/README.md](../roadmap/README.md#milestones)); #22 itself carries no milestone since it spans all five.

| Row | Issue | Row | Issue |
|---|---|---|---|
| F1 | #23 | F13 | #30 |
| F3 | #24 | F14 | #31 |
| F5 | #25 | F15 | #32 |
| F6 | #26 | F16 | #33 |
| F7 | #27 | F17 | #34 |
| F8 | #28 | F18 | #35 |
| F12 | #29 | F19 | #36 |
| F2, F4 | to file (spec program) | F20 | #37 |
| F9, F10, F11 | held — no issue yet | F25 | to file (spec program) |

**Superseded rows.** Rows 15–20 of the pre-v0.4 sequence never started; F1–F25 above absorbs them: machine-teeth → F7, plugin-runtime → F21/F22, spec-js → F16/F17, delegation-redirect → F9, spec-delta → F18, doc-truth → F24.

**Active focus.** The spec program (design: `docs/superpowers/specs/2026-10-13-spec-v04-design.md`) runs on one branch: amendment → F1 → F25 → F2 ∥ F3 → F4 → F16 → F17 → F18 → F19 → F20 → spec slices of F21–F24. F2 and F4 are un-held and scope-cut to the spec tool. F9, F10, F11 stay held; the flow (F5–F8) and instruct (F12–F15) tracks are unchanged and not part of this program.

---

## v0.5 — the vibe CLI arc (queued)

The follow-on arc: one `vibe` CLI front door over the three tools, plus the cross-tool capabilities no single tool can own — worktree lifecycle, global spec management, global blocks. Architecture: [tech-cli.md](tech/cli.md). Decisions: D40–D43 below. Nothing starts before F22 is DONE; C4 additionally waits on F16, C5 on F14.

**Issue catalog.** #42 tracks this arc; C1–C6 are #43–#48, filed as its sub-issues (2026-09-26), all six on the `v0.5 — CLI front door` milestone; #42 itself carries none.

| Order | Feature | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|
| C1 | cli-core | `crates/vibe`: `spec\|flow\|instruct` dispatch via the shim's discovery tiers; `~/.config/vibe/repos.json` registry; `space list`; `doctor` peer + registry checks | Dispatch matrix with each tier's env deleted (both legs); missing-peer advice lines; registry floor test | NOT STARTED | F22 |
| C2 | worktree-lifecycle | `worktree create/list/remove`: git worktree + isolated `.vibe/run/`; mid-flow remove refusal; deregistration | Refusal discriminates vs naive removal; cursor isolation per worktree; fresh non-git target exits 1 with one line | NOT STARTED | C1 |
| C3 | worktree-bootstrap | Bootstrap on create: `.vibe/spec.json` root pointer, each tool's `init`, AGENTS.md blocks; staleness in `list` | Byte-parity of bootstrapped files vs `install.sh --local` on the same tree; bootstrap idempotent on re-run | NOT STARTED | C2 |
| C4 | global-lessons | Global library at `~/.config/vibe/spec/lessons.md`; `lessons-for` global source (repo wins on tag conflict); `space lessons edit/promote` | Merged-answer goldens (repo-only, global-only, conflict); promote drops repo-pattern prose; absence is clean, not warned | NOT STARTED | F16, C1 |
| C5 | space-blocks | `space blocks` list/edit global tier + pin a personal git source; sync via `vibe-instruct sync` | Pinned-source refuse-when-unpinned; offline sync path; no fetch code in `crates/vibe` (structural assertion) | NOT STARTED | F14, C1 |
| C6 | worktree-archive | `archive`: uncommitted `features/<name>/` residue moved to the main checkout, then remove; registry updates | Residue golden (before ≥1, after ==0, files ≥ floor); committed-edits-travel-by-merge control | NOT STARTED | C2, C4 |

---

## Critical Architecture Decisions

### Decided

- [D8 — Lessons split](decisions/d8-lessons-split.md)
- [D12 — Orders in the skill](decisions/d12-orders-in-the-skill.md)
- [D13 — Carrier split](decisions/d13-carrier-split.md)
- [D14 — One JS engine](decisions/d14-one-js-engine.md)
- [D15 — Content is data](decisions/d15-content-is-data.md)
- [D16 — Injection is trigger-classed](decisions/d16-injection-is-trigger-classed.md)
- [D17 — The gate lives in the writer](decisions/d17-the-gate-lives-in-the-writer.md)
- [D18 — Three tools, no umbrella](decisions/d18-three-tools-no-umbrella.md)
- [D19 — Rust, static, zero runtime deps](decisions/d19-rust-static-zero-runtime-deps.md)
- [D20 — Contracts are the only coupling](decisions/d20-contracts-are-the-only-coupling.md)
- [D21 — `.vibe/` is the per-repo directory](decisions/d21-vibe-is-the-per-repo-directory.md)
- [D22 — Provider payload contract](decisions/d22-provider-payload-contract.md)
- [D23 — Budgets keep today's numbers](decisions/d23-budgets-keep-todays-numbers.md)
- [D24 — Managed-block grammar v1, per owner](decisions/d24-managed-block-grammar-v1-per-owner.md)
- [D25 — Hook ownership](decisions/d25-hook-ownership.md)
- [D26 — Hook shim and discovery](decisions/d26-hook-shim-and-discovery.md)
- [D27 — The writer is the gate](decisions/d27-the-writer-is-the-gate.md)
- [D28 — Ledger + advisory signals](decisions/d28-ledger-advisory-signals.md)
- [D29 — TUI and recorded workflows](decisions/d29-tui-and-recorded-workflows.md)
- [D30 — Port with frozen oracles, hard units first](decisions/d30-port-with-frozen-oracles-hard-units.md)
- [D31 — Instruct tiers](decisions/d31-instruct-tiers.md)
- [D32 — Sources and sync](decisions/d32-sources-and-sync.md)
- [D33 — Spec root and discovery](decisions/d33-spec-root-and-discovery.md)
- [D34 — Document model](decisions/d34-document-model.md)
- [D35 — Strictness ratchet](decisions/d35-strictness-ratchet.md)
- [D36 — Spec subagents are real platform agents](decisions/d36-spec-subagents-are-real-platform-agents.md)
- [D37 — Packaging: the tool directory is the plugin](decisions/d37-packaging-the-tool-directory-is-the.md)
- [D38 — One installer, three release trains](decisions/d38-one-installer-three-release-trains.md)
- [D39 — Doctrine stays](decisions/d39-doctrine-stays.md)
- [D40 — The vibe CLI is the front door (v0.5)](decisions/d40-the-vibe-cli-is-the-front.md)
- [D41 — Worktree lifecycle (v0.5)](decisions/d41-worktree-lifecycle-v0-5.md)
- [D42 — Global spec space (v0.5)](decisions/d42-global-spec-space-v0-5.md)
- [D43 — Instructions sharing via the global tier (v0.5)](decisions/d43-instructions-sharing-via-the-global-tier.md)
- [D44 — Layered spec layout](decisions/d44-layered-spec-layout.md)
- [D45 — Superpowers-first feature layer](decisions/d45-superpowers-first-feature-layer.md)
- [D46 — Promote, then delete](decisions/d46-promote-then-delete.md)
- [D47 — Router + tooth](decisions/d47-router-tooth.md)

### Resolved

- [x] **OPEN-2 — Skill count.** Seven `vibe-*` shims consolidated into one `vibe` skill (2026-06-29).
- [x] **OPEN-3 — Install mode.** `install.sh` copies core + Claude adapter, merges `AGENTS.md`, supports partial/dry-run/uninstall.
- [x] **OPEN-4/OPEN-7 — Hook strictness.** Shipped warn-first; only the pre-existing hard blocks deny.
- [x] **OPEN-6 — Skill degradation.** `check-skills.sh` warns on unverifiable delegates and degrades to inline orders.

---

## Spec vs Implementation

F1 and F25 are delivered. The rest of the v0.4 arc — F2 through F24 — is spec-ahead-of-code. The tree is still v0.3 apart from F1 and F25: rows 12–14 (js-core, content-layer, inject-triggers) are delivered, and the old rows 15–20 (machine-teeth, plugin-runtime, spec-js, delegation-redirect, spec-delta, doc-truth) were never started before this rewrite superseded them. F1 delivered the spec-path fix: the scripts `validate.sh`, `list-specs.sh`, `lessons-for.sh`, and `scan-merges.sh` walk up from the cwd to the directory holding `.git`, and a relative `SPEC_DIR` now resolves against the root found. `promote.sh`, `setup.sh`, and `check-drift.sh` are unchanged.

The v0.5 arc (C1–C6) is likewise spec-ahead — planned, not started. It gates on F22 (installer/release surface), with C4 after F16 and C5 after F14.

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
