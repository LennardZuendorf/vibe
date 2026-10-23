# Spec v0.4 — layered docs/spec, superpowers-first, enforced, Rust — Design

**Date:** 2026-10-13 · **Branch:** `villain/vibe-integration` · **Delivery:** one branch, one PR,
one commit series per row · **Tracking:** amends #22 (v0.4)

## 1. Intent

**Outcome.** vibe's spec system lives at `docs/spec/` as a layered vault; feature design and
planning default to superpowers; the spec skill is as hard to skip as superpowers; no spec bash
script remains — every spec command is the `vibe-spec` Rust binary.

**Stated by the owner.**
- Move `.spec/` → `docs/spec/`.
- Layered strategic spec modelled on `customer-portal/specs/`, keeping some root documents.
- superpowers `brainstorming` + `writing-plans` by default; vibe's feature spec only as fallback.
- Superpowers-style skill description plus a hook that leaves no room to skip the tools.
- Rewrite the whole skill in a concise, clear, assertive voice that forces spec use by default,
  even with no system prompt telling the agent to.
- Rust for all spec scripts (Node ruled out by D19).
- Amend the v0.4 roadmap and deliver every v0.4 issue that covers the spec tool in this task.

**Assumptions.** This repo dogfoods the result. `customer-portal` is a model only and is not
changed.

**Success criteria.**
1. `vibe-spec validate --strict` green on this repo's migrated, layered `docs/spec/`.
2. Zero `.spec` literals outside `oracles/` and lines tagged `<!-- literal -->`.
3. `spec/scripts/*.sh` deleted; `cargo test --workspace` and parity green.
4. Pressure-test baseline: with no CLAUDE.md/AGENTS.md, fresh subagents invoke spec (or the
   superpowers chain it routes to) in every scenario.
5. The PreToolUse tooth blocks an unskilled edit under `docs/spec/**` and a discriminating test
   fails if the tooth is replaced by `exit 0`.
6. Every acceptance criterion of #23, #24, #33–#37 met; spec slices of #38–#41 met.

## 2. Scope

| In | Out |
|---|---|
| F1, F16–F20 (spec track) | flow Rust port F5–F8 |
| F2 (Spec JSON slice), F3, F4 (scope-cut) — un-held | F9 signals, F10 TUI |
| Spec slices of F21–F24 | instruct F11–F15 |
| New row F25 (skill rewrite, superpowers default) | v0.5 CLI C1–C6 |
| Flow *content* edits forced by the path move and D45 | flow `engine/` changes beyond literals |

## 3. Roadmap amendment (lands first, via `/flow` strategy)

### Decisions revised

- **D25 Hook ownership.** Adds: spec owns a `SessionStart` router injection and a blocking
  `PreToolUse` spec-path tooth. F19's "exit 0 always" applies to the validate hooks only.
- **D33 Spec root.** Discovery order unchanged. Adds: `docs/superpowers/` is a recognised
  artifact root; vibe reads it and deletes from it only at compound.
- **D34 Document model.** The strategic layer stays vibe's own (hubs, kind folders, lessons). The
  feature layer defaults to superpowers' native artifacts; `features/<f>/` is the fallback.
- **D39 Doctrine.** "Enforce in code, explain in prose": prose may *route* to a coded rule, never
  restate its details.
- **product.md Design Principle 5** ("canonical paths beat skill defaults") is replaced by D45.

### Decisions added

- **D44 Layered layout** — §4.
- **D45 Superpowers-first feature layer** — §5.
- **D46 Promote-then-delete** for superpowers artifacts — §5.
- **D47 Router + tooth** enforcement — §6.

### Rows

| Row | Change |
|---|---|
| **F25 (new)** spec-superpowers-default | Markdown/flow content only: full skill rewrite (§6.1), `using-spec` router text, delete redirect/`skip:` lines, un-ignore `docs/superpowers/`, fallback rules. Starts: amendment. |
| F1 | Unchanged. |
| F2 | Un-held. Spec JSON v1 + exit-code contract only. |
| F3 | Full oracle freeze; `vibe-parity` harness; spec corpus classes only (flow classes move to F5). |
| F4 | Un-held. Crates `vibe-core`, `vibe-spec`, `vibe-parity` only. |
| F16 | Unchanged (straight parity port). |
| F17 | Widened: layered allowlist, budgets, links, unique names, frontmatter, `code:` existence, orphans, superpowers artifact checks. |
| F18 | Widened: owns `promote.sh` port; promotion from superpowers artifacts; promote-then-delete; `check-drift --pre-merge`. |
| F19 | Widened: router hook + blocking tooth; retire `spec-interviewer`/`spec-tracer`. |
| F20 | Widened: owns `setup.sh` port (`init`); layered migration. |
| F21–F24 | Spec slices only. |

Process: edit `.spec/plan.md` (then `tech.md`, `tech-spec.md`, `tech-rust.md`,
`tech-contracts.md`, `product.md`) inside `strategy.spec`; regenerate `docs/roadmap/issues/`.

### Sequence

```text
amendment → F25 → F1 → F2 ∥ F3 → F4 → F16 → F17 → F18 → F19 → F20 → F21s → F22s → F23s → F24s
```

Port first, restructure after: each script lands parity-green against its frozen oracle before
any behaviour change; deliberate changes become `expected-divergence` cases.

## 4. Layout and validators (D44, F17)

```text
docs/spec/
├── README.md        vault entry, links hubs
├── AGENTS.md        how to write and change the spec
├── product.md       hub: story, users, non-goals, links down
├── tech.md          hub: architecture, links down
├── design.md        hub
├── plan.md          hub: sequence, gates, decision index
├── lessons.md       kept (D34)
├── areas/           where things happen          (code: required)
├── flows/           journeys across areas
├── concepts/        shared nouns
├── platform/        always-true rules
├── tech/            directives                   (applies: required)
├── decisions/       one D-record per file (dNN-<slug>.md)
├── questions/       open questions, deleted when answered
├── parts/<part>/    optional; same kind folders per part
├── features/<f>/    fallback feature spec
└── archive/         transient, fallback only
```

- Kind folders are created on demand. `vibe-spec init` seeds only hubs, `README.md`, `AGENTS.md`,
  `lessons.md`.
- Names: lowercase-kebab files and folders, unique across the tree. One fact, one place.
- Links: relative markdown (renders on GitHub); every link must resolve.
- Frontmatter: `type` ∈ `hub|area|flow|concept|platform|tech|decision|question` and `updated`
  required; `code:` (path or list, each must exist) on `area`; `applies:` (glob) on `tech`.
- Budgets: hub ≤80 lines, note ≤60, `lessons.md` unbounded. Over budget → split into a folder
  with its own hub; never trim facts.
- Notes state today's truth only. No history, no TODOs.

**F17 checks** (warn-first; ratchet to error after F20 migrates this repo): layout allowlist (tree
above + `docs/superpowers/{specs,plans}/*.md`), budgets, link resolution, unique names, required
frontmatter per type, `code:` existence, every kind note reachable from a hub. Superpowers
artifacts: name `YYYY-MM-DD-<topic>(-design)?.md`; a plan's `**Spec:**` link resolves; content
format not linted. SF9–SF12 unit-ID checks apply only to fallback `features/<f>/plan.md`.

## 5. Superpowers integration and lifecycle (D45, D46, F18, F25)

| Work | Driver | Writes |
|---|---|---|
| Strategy (vision, areas, directives, decisions) | `spec` (brainstorming may run the dialogue) | `docs/spec/**` |
| Feature — architectural | `superpowers:brainstorming` → `writing-plans` | `docs/superpowers/specs/YYYY-MM-DD-<feature>-design.md`, `docs/superpowers/plans/YYYY-MM-DD-<feature>.md` |
| Feature — bounded | brainstorming bounded path | nothing (chat design) |
| Fallback | `spec` feature flow | `docs/spec/features/<feature>/` |

- Binding: superpowers topic slug == vibe feature name. Flow orders pass `<feature>`; without flow
  the spec skill names the topic.
- Fallback triggers (exhaustive): superpowers absent (`check-skills`); user says "self";
  `.vibe/spec.json` `"featureLayer": "vibe"`; OpenSpec Requirement/Scenario interchange needed.
- superpowers' chain runs untouched; its plan approval + execution choice is flow's
  `feature.plan>feature.impl` gate.
- Flow content (F25): delete redirect/`skip:` lines in `flow/{feature,strategy,quick}.md`;
  update `writes:` in `state-machine.json`; `policy.json` frozen-during-impl warn adds
  `docs/superpowers/specs/**`; `.gitignore` drops `docs/superpowers/`, keeps `.superpowers/`.

**Compound (F18).**
1. `vibe-spec promote <feature>` lists the feature's superpowers artifacts and prints a promotion
   checklist with target notes: chosen approach → `decisions/`; new nouns → `concepts/`; built
   places → `areas/` (+`code:`); rules → `tech/`; failures → `lessons.md`.
2. The agent writes the notes (artifacts are free prose; extraction is agent-led, mechanically
   gated).
3. `vibe-spec promote <feature> --done` deletes the artifacts; refuses if `docs/spec/` is
   unchanged since the branch base.
4. `vibe-spec check-drift --pre-merge` fails if any `docs/superpowers/**` file remains. Flow
   compound and `finishing-a-development-branch` run it; plain CI only warns.

`<!-- merge -->` block promotion stays for fallback feature folders.

**Dogfood.** This document is promoted, then deleted, before this PR merges.

## 6. Enforcement (D47, F19, F25)

### 6.1 Skill rewrite (F25)

- Rewrite `spec/SKILL.md`, `feature.md`, `strategy.md`, `reference/**`, templates: concise,
  imperative, assertive; one rule per line; no hedging. Method: `superpowers:writing-skills`
  RED-GREEN-REFACTOR; wording via `asd-ste100`.
- Description: trigger-only, superpowers style, no workflow summary. Draft: *"You MUST use this
  before changing what the product does, how it is built, or any file under docs/spec or
  docs/superpowers — features, refactors, architecture, decisions, PRDs, plans. Use before writing
  code, before brainstorming alone, before answering 'how should we build X'."*
- Body carries a `<HARD-GATE>`, a Red Flags table (Thought | Reality) and the routing table (§5).
- Pressure tests: subagents with no CLAUDE.md/AGENTS.md, realistic prompts ("add a flag", "write a
  PRD", "plan this refactor", "quick fix"). Rationalizations captured verbatim become Red Flags
  rows. Pass bar: spec (or its routed superpowers skill) used in 100 % of scenarios.

### 6.2 Router injection (F19)

- New skill `using-spec`, ≤150 words, `<SUBAGENT-STOP>`, 1 % rule, Red Flags table.
- Spec-owned `SessionStart` hook (`startup|clear|compact`, sync) injects it inside
  `<EXTREMELY_IMPORTANT>`, one JSON shape per harness.
- Coexistence with superpowers' router, stated once: spec decides *where* work is recorded;
  superpowers process skills decide *how* feature work is designed; superpowers absent → spec
  fallback.
- OpenCode: router via spec's AGENTS.md fragment (≤6 lines); API checked via docs in F19.

### 6.3 Tooth (F19)

`PreToolUse` on `Edit|Write|MultiEdit|NotebookEdit` → `vibe-spec hook guard`.

| Target | Skill required in the session transcript |
|---|---|
| spec root (`docs/spec/**`, `.vibe/spec.json`) | `spec` (any plugin namespace) |
| `docs/superpowers/specs/**` | `superpowers:brainstorming` or `spec` |
| `docs/superpowers/plans/**` | `superpowers:writing-plans` or `spec` |

- Block: exit 2, one line, e.g. `blocked: invoke Skill(spec) before editing docs/spec/areas/checkout.md`.
- Bash redirects to these paths: warn only.
- Fail open (one stderr line each): transcript unreadable, binary missing (shim exits 0), no spec
  root, malformed hook input.
- Owner override: `.vibe/spec.json` `"tooth": "warn"` — itself behind the tooth.
- Subagents: F19 confirms from a captured real payload that a subagent's `PreToolUse` sees the
  parent's skill load; if not, also accept `spec` named in the Task prompt.
- Independent of flow's guard; both must allow.
- Agents: retire `spec-interviewer`, `spec-tracer`; keep `spec-promoter`, `spec-health`. Validate
  hooks (`PostToolUse`/`Stop`) stay exit 0.

## 7. Rust foundations and port (F2, F3, F4, F16)

- **F4:** `crates/{vibe-core,vibe-spec,vibe-parity}`; MSRV 1.88; `cargo-deny` allowlist
  `clap`/`serde`/`serde_json`; hand-written glob + atomic write in core; `cargo metadata` graph
  test; CI `check`, `test` (3 OS × stable+MSRV), `parity`.
- **F2:** `contracts/` Spec JSON v1 schema + goldens (`root`, `lessons-for`, `plan`, `feature`),
  exit-code contract, `run.sh`.
- **F3:** `oracles/{bash,js}/` + `MANIFEST.sha256`; generic `vibe-parity`; spec corpus classes
  with asserted floors.

| Today | Command | Row |
|---|---|---|
| `validate.sh` | `vibe-spec validate [--changed\|--file\|--strict\|--format]` | F16, F17 |
| `list-specs.sh` | `vibe-spec list-specs` | F16 |
| `lessons-for.sh` | `vibe-spec lessons-for <tags>` | F16 |
| `scan-merges.sh` | `vibe-spec scan-merges` | F16 |
| `check-drift.sh` | `vibe-spec check-drift [--pre-merge]` | F18 |
| `promote.sh` | `vibe-spec promote <feature> [--dry-run\|--done]` | F18 |
| `setup.sh` | `vibe-spec init` | F20 |
| — | `vibe-spec root`, `trace` | F16, F17 |
| — | `vibe-spec hook {session-start,guard,validate}` | F19 |
| — | `vibe-spec migrate --to docs/spec [--layout layered]` | F20 |

- `--json` on every command; exit 0 ok · 1 findings · 2 usage · 3 no root.
- Skills, CI and flow switch from `bash spec/scripts/*` to `vibe-spec` in the same commit as each
  port; `!list-specs.sh` bang-exec → `!vibe-spec list-specs`.
- Dev bootstrap until F22: `cargo build` + shim `$VIBE_SPEC_BIN`.

## 8. Migration, packaging, retirement (F20–F24 spec slices)

**F20.**
1. Mechanical `vibe-spec migrate --to docs/spec`: `git mv`; `.vibe/spec.json` `root`; literal
   rewrite over an explicit file set — for this repo widened to `flow/**`, `plugin/**`,
   `install.sh`, `build-plugin.sh`, `.github/**` (flow literals: `policy.json`, `detect-context`
   arms, marker search, `session-start.sh` via its generator); population assertion or revert.
2. `--layout layered`: `{tech,product,plan}-<topic>.md` → `<kind>/<topic>.md`; D-records →
   `decisions/dNN-<slug>.md`; hubs seeded. Splitting oversized `product.md` (219) and `tech.md`
   (284) into notes is agent-led via the skill's restructure mode, gated by
   `validate --strict` green. This repo: parts `parts/{spec,flow,instruct,cli}/`.
3. `.spec/archive/` (9 frozen files) deleted in the migration commit; git keeps history. Owner
   confirmed 2026-10-13.
4. `docs/roadmap/README.md` and its generator point at `docs/spec/plan.md`.

**F21s.** `spec/` becomes a plugin dir: `.claude-plugin/plugin.json`, `skills/{spec,using-spec}`,
`agents/{spec-promoter,spec-health}`, `hooks/hooks.json` → `bin/hook.sh`, OpenCode shim. The
`vibe` bundle drops `skills/spec` and becomes flow-only. `marketplace.json` lists both.
`claude plugin validate --strict` passes.

**F22s.** cargo-dist for `vibe-spec` (5 targets, `spec-v*`); `install.sh` verified binary fetch +
`cargo install` fallback, recorded in `~/.vibe/installed.json`. Pushing a `spec-v0.4.0` tag is
outward-facing — owner confirms first.

**F23s.** Delete `spec/scripts/*.sh` (copies stay in `oracles/bash/`); bash `spec/tests/run.sh`
replaced by cargo tests + parity. `flow/engine/` untouched.

**F24s.** Spec README, CHANGELOG, hub truth sweep, `check-drift` green, stranger eval from a fresh
non-git target.

**GitHub (owner confirms each before posting).** New issue for F25; issues for un-held F2, F4. PR
body `Closes #23 #24 #33 #34 #35 #36 #37`. Comment the delivered spec slice on #38–#41 (stay
open). Amendment comment on #22.

**Open PR #49** adds `.spec` literals; whichever PR merges second re-runs `vibe-spec migrate`
(idempotent literal rewrite).

## 9. Testing

- Each issue's acceptance criteria, verbatim, as the row's exit gate.
- Parity green before any behaviour change; deliberate changes as `expected-divergence`.
- Discriminating tests: tooth vs `exit 0`; `promote --done` refusal vs naive delete; migrate
  revert vs half-migrated tree.
- Hook fixtures from captured payloads: path × skill present/absent × each fail-open cause.
- Skill pressure-test baseline before and after hooks.
- End gate: `cargo test --workspace`, parity, `vibe-spec validate --strict`, remaining bash/Node
  flow suites, shellcheck — all green.

## 10. Risks

| Risk | Mitigation |
|---|---|
| One very large PR | One commit series per row; each row's gate evidence in the PR body. |
| Subagent hook payload lacks parent skill load | Verified from a real payload in F19 before the rule is final; Task-prompt fallback. |
| superpowers changes its default paths | Paths held in one `vibe-core` constant; drift check flags unknown artifact dirs. |
| Tooth absent where binary is missing | Fail-open is explicit; `vibe-spec doctor` and the router name the missing binary. |
| Validators redden live specs | Warn-first; ratchet to error only after F20. |
| Flow literals missed by migrate | Population assertion over the widened file set; flow suites must stay green. |
