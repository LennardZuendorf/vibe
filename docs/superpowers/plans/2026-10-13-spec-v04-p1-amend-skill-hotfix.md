# Spec v0.4 Phase 1 — Roadmap Amendment, F1 Hotfix, F25 Skill Rewrite — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the v0.4 roadmap amendment in the canonical specs and issue files, fix the spec scripts' root discovery (F1), and ship F25 — superpowers-first flow content, a new write-policy rule, and a pressure-tested, assertive spec skill.

**Architecture:** Markdown, JSON and bash only; no Rust in this phase. Root-spec edits run inside the vibe flow's `strategy.spec` state because the write guard blocks them elsewhere. F1 runs before the skill rewrite (the design lists F25 first; F1 is mechanical and the rewrite then starts from fixed paths). Skill wording is driven by a headless pressure-test harness (RED → GREEN → REFACTOR per `superpowers:writing-skills`).

**Tech Stack:** bash 3.2+ (`set -euo pipefail`, shellcheck-clean), Node ≥18 (flow engine tests, pressure-harness parsing), Claude Code CLI 2.1.x (`claude -p` with an empty `CLAUDE_CONFIG_DIR`).

**Spec:** `docs/superpowers/specs/2026-10-13-spec-v04-design.md`

## Global Constraints

- Commits: Conventional Commits, imperative, lowercase, ≤50-char subject, no body, no footer.
- Bash MUST use `set -euo pipefail` and be shellcheck-clean; deterministic, idempotent, warn-not-fail.
- Never edit `.agents/skills/vibe/state.json` by hand; transitions only via `bash .agents/skills/vibe/scripts/set-state.sh <flow.phase> [feature]`.
- Root `.spec/{product,tech,design,plan,lessons}.md` edits only while the cursor is `strategy.spec`.
- `AGENTS.md` `vibe:rules` block is rendered: after any `content/policy.json` change run `node .agents/skills/vibe/engine/cli.mjs render agents-md --write`.
- Superpowers artifacts live at `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md` and `docs/superpowers/plans/YYYY-MM-DD-<topic>.md`; `.superpowers/` stays gitignored.
- Skill description: trigger-only, superpowers style, ≤1024 characters, no workflow summary.
- Spec root discovery stops at the directory holding `.git` (D33).
- A guard asserts its own population: a floor on files examined.
- Scratch check scripts live in `.superpowers/p1/` (gitignored); never `/tmp`.
- Subagent dispatch: set `model` explicitly (repo AGENTS.md). Tasks 4, 12 → `haiku`; Tasks 1–3, 5–9, 11 → `sonnet`; Task 10 → `opus` (or run it inline).

## Review Focus

1. Spec scripts invoked from a plugin-cache path while cwd is a nested project subdirectory — must resolve the project's `.spec/`, not the script's location (Task 5 test `f1_plugin_cache_layout`).
2. A nested superpowers path (`docs/superpowers/specs/sub/x-design.md`) during `feature.impl` — must warn like a flat one (Task 7 test).
3. `content/policy.json` gains a rule but the rendered `AGENTS.md` block or the `N rule(s)` count drifts — `render --check` and the CLI count test must both catch it (Task 7 Step 6).
4. Superpowers not installed — the rewritten skill must route feature work to the spec fallback, not stall (Task 9 scenarios with `SP=absent`).
5. Skill frontmatter breaks YAML or the description exceeds 1024 characters after rewrite — skill silently stops loading (Task 10 test `sf17_description_budget`).

---

### Task 1: Amend decisions and principles

**Files:**
- Modify: `.spec/plan.md` (§ Critical Architecture Decisions → Decided: D25, D33, D34, D36, D39; append D44–D47 after D43)
- Modify: `.spec/product.md:61` (principle 9), `.spec/product.md:80` (Design Principle 5), `.spec/product.md:205` (resolved 8)

**Interfaces:**
- Consumes: none.
- Produces: decision IDs D44 (layered layout), D45 (superpowers-first feature layer), D46 (promote-then-delete), D47 (router + tooth) — cited by Tasks 2–4 and by every later phase.

- [ ] **Step 1: Enter the strategy flow**

```bash
mkdir -p .superpowers/p1
bash .agents/skills/vibe/scripts/set-state.sh strategy.brainstorm
bash .agents/skills/vibe/scripts/set-state.sh strategy.spec
bash .agents/skills/vibe/scripts/detect-context.sh decide .spec/plan.md
```
Expected last line: `allow`

- [ ] **Step 2: Write the failing check**

```bash
cat > .superpowers/p1/decisions-check.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
p=.spec/plan.md; r=.spec/product.md
for d in D44 D45 D46 D47; do grep -q "^- \*\*$d — " "$p" || { echo "missing $d"; exit 1; }; done
grep -q 'spec owns a `SessionStart` router injection' "$p" || { echo "D25 not amended"; exit 1; }
grep -q 'docs/superpowers/` is a recognised artifact root' "$p" || { echo "D33 not amended"; exit 1; }
grep -q 'feature layer defaults to superpowers' "$p" || { echo "D34 not amended"; exit 1; }
grep -q 'Two spec agents' "$p" || { echo "D36 not amended"; exit 1; }
grep -q 'prose may route to a coded rule' "$p" || { echo "D39 not amended"; exit 1; }
grep -q 'Canonical paths beat skill defaults' "$r" && { echo "DP5 still present"; exit 1; }
echo OK
EOF
bash .superpowers/p1/decisions-check.sh
```
Expected: FAIL with `missing D44`

- [ ] **Step 3: Replace D25, D33, D34, D36, D39 in `.spec/plan.md`**

Replace each whole bullet line with:

```markdown
- **D25 — Hook ownership.** instruct owns SessionStart/UserPromptSubmit composition; flow owns guard/sniffer/Stop/ledger hooks; spec owns its own Stop/PostToolUse validation, and also spec owns a `SessionStart` router injection and a blocking `PreToolUse` spec-path tooth (D47). Only the validate hooks are bound to "exit 0 always".
- **D33 — Spec root and discovery.** Default `docs/spec/`, legacy `.spec/`; `--root` → env → config → default → parent search (stops at the directory holding `.git`) → exit 3. `docs/superpowers/` is a recognised artifact root: vibe reads it and deletes from it only at compound (D46).
- **D34 — Document model.** The strategic layer is vibe's own (hubs, kind folders, lessons — D44); the feature layer defaults to superpowers native artifacts (D45), with `features/<name>/` as the fallback. OpenSpec Requirement/Scenario/delta grammar is kept for interchange.
- **D36 — Spec subagents are real platform agents.** Two spec agents (`spec-promoter`, `spec-health`) with explicit frontmatter and write paths, registered on both Claude Code and OpenCode; `spec-interviewer` and `spec-tracer` retire in favour of superpowers brainstorming and writing-plans.
- **D39 — Doctrine stays.** Enforce in code, explain in prose; prose may route to a coded rule, never restate its details; data over prose; degrade gracefully; committed memory readable by any agent or none.
```

- [ ] **Step 4: Append D44–D47 after the D43 bullet**

```markdown
- **D44 — Layered spec layout.** `docs/spec/` = root hubs (`product`, `tech`, `design`, `plan`; `lessons.md` kept) + kind folders created on demand (`areas/`, `flows/`, `concepts/`, `platform/`, `tech/`, `decisions/`, `questions/`, optional `parts/<part>/`). Lowercase-kebab unique names, relative markdown links, typed frontmatter (`code:` on areas, `applies:` on tech), hub ≤80 lines, note ≤60; over budget splits, never trims.
- **D45 — Superpowers-first feature layer.** Feature design and plans default to `superpowers:brainstorming` → `superpowers:writing-plans` writing their native artifacts under `docs/superpowers/{specs,plans}/`; topic slug == feature name. Fallback to `features/<name>/` only when superpowers is absent, the user says "self", `.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec interchange is needed. Supersedes "canonical paths beat skill defaults".
- **D46 — Promote, then delete.** At compound, durable facts from a feature's superpowers artifacts are promoted into the layered spec and lessons; `vibe-spec promote <feature> --done` then deletes the artifacts and refuses if `docs/spec/` is unchanged since the branch base; `check-drift --pre-merge` fails while any `docs/superpowers/**` file remains.
- **D47 — Router + tooth.** The spec skill carries a trigger-only, assertive description; a spec-owned `SessionStart` hook injects a short `using-spec` router; a spec-owned `PreToolUse` hook blocks edits under the spec root and `docs/superpowers/**` unless the matching skill was loaded in the session; it fails open on unreadable input and honours an owner-set `"tooth": "warn"`.
```

- [ ] **Step 5: Amend `.spec/product.md`**

Line 61 → `9. **Enforce in code, explain in prose.** Prose may route to a coded rule, never restate it. Data over prose: anything the machine knows is interpolated, never restated in English.`

Line 80 → `5. **Superpowers first, vibe as memory.** Feature design and plans use superpowers' own artifacts; vibe promotes their durable facts into the layered spec, then deletes them (D45, D46).`

Line 205 → `8. **OpenSpec.** Adopt the Requirement/Scenario grammar for interchange; the strategic layout is vibe's layered vault (D44), the feature layer is superpowers-first (D45).`

- [ ] **Step 6: Run the check and the validator**

```bash
bash .superpowers/p1/decisions-check.sh
bash spec/scripts/validate.sh
```
Expected: `OK`; validate exits 0 (warnings allowed, no errors).

- [ ] **Step 7: Commit**

```bash
git add .spec/plan.md .spec/product.md
git commit -m "docs(spec): amend v0.4 decisions for spec program"
```

---

### Task 2: Amend the feature sequence

**Files:**
- Modify: `.spec/plan.md` § Feature Sequence table rows F2, F3, F4, F16–F24; add row F25; § issue catalog table; § "Active focus" paragraph; § Spec vs Implementation first paragraph.

**Interfaces:**
- Consumes: D44–D47 (Task 1).
- Produces: row F25 `spec-superpowers-default`; the statuses Tasks 4 and every later phase update.

- [ ] **Step 1: Write the failing check**

```bash
cat > .superpowers/p1/rows-check.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
p=.spec/plan.md
grep -q '^| F25 | spec-superpowers-default |' "$p" || { echo "missing F25 row"; exit 1; }
grep -E '^\| F(2|4) \|' "$p" | grep -q '(held)' && { echo "F2/F4 still held"; exit 1; }
grep -q 'promote <feature> --done' "$p" || { echo "F18 not widened"; exit 1; }
grep -q 'layered' <(grep '^| F20 |' "$p") || { echo "F20 not widened"; exit 1; }
echo OK
EOF
bash .superpowers/p1/rows-check.sh
```
Expected: FAIL with `missing F25 row`

- [ ] **Step 2: Replace rows F2, F3, F4 (whole lines)**

```markdown
| F2 | contracts | spec | `contracts/` Spec JSON v1 schema + goldens (`root`, `lessons-for`, `plan`, `feature`), exit-code contract, `run.sh`; payload, provider manifest and marker grammar move to their tools' rows | Green on empty tools; wired into CI | NOT STARTED | — |
| F3 | oracles-parity | all | Frozen `oracles/js`+`oracles/bash` + `MANIFEST.sha256`; generic `vibe-parity` differential harness; spec corpus classes with floors (flow classes land with F5) | Oracle byte change reddens CI; planted mutant confirmed via `git diff --numstat` also reddens it | NOT STARTED | — |
| F4 | workspace-core | all | Cargo workspace with `vibe-core`, `vibe-spec`, `vibe-parity` only (flow/instruct/signals crates arrive with their rows); MSRV 1.88; `cargo-deny` allowlist | Green matrix (3 OS × stable+MSRV); `cargo metadata` graph assertion | NOT STARTED | F2 |
```

- [ ] **Step 3: Replace rows F17–F24 (whole lines)**

```markdown
| F17 | spec-validators | spec | Substring-bug fixes; strictness config + `--strict`; grammar checks + unified R-ID; `trace`; layered-layout checks (D44: allowlist, budgets, link resolution, unique names, typed frontmatter, `code:` existence, hub reachability); superpowers artifact name + `**Spec:**` link checks | One landed mutant per bug fails; warn/error/off matrix; OpenSpec samples pass; layered fixture tree passes, each mutated rule fails | NOT STARTED | F16 |
| F18 | spec-delta | spec | Header-keyed ADDED/MODIFIED/REMOVED promotion engine; `promote.sh` port incl. superpowers promotion checklist and `promote <feature> --done` (D46); `check-drift` port with `--pre-merge`; `export\|import --openspec` | Golden trees; MODIFIED replaces in place; round-trip byte-identity; `--done` refuses on unchanged `docs/spec/` | NOT STARTED | F17 |
| F19 | spec-hooks-agents | spec | Validate hooks (`validate --changed`/`--file`, exit 0); `using-spec` SessionStart router; blocking `PreToolUse` tooth (D47); agents `spec-promoter` + `spec-health` registered on both platforms; `spec-interviewer`/`spec-tracer` retired | Changed/unchanged root; tooth blocks unskilled edit and a discriminating test fails on `exit 0`; captured subagent payload fixture; `claude plugin validate` | NOT STARTED | F16 |
| F20 | spec-migrate | spec | `setup.sh` port (`init`); `migrate --to docs/spec` (population assertion, allow-markers) + `--layout layered`; this repo → layered `docs/spec/` in one commit; `.spec/archive/` deleted | Dry-run table; zero literals after; full suite + `validate --strict` green | NOT STARTED | F17, F19 |
| F21 | packaging | all | Three plugin dirs (manifests, skills, commands, agents, hooks, bin, opencode); `marketplace.json`; `doctor` peer checks — spec slice in the spec program | `claude plugin validate --strict` × 3; shim exits 0 without binary | NOT STARTED | F8, F14, F19 |
| F22 | release-install | all | `cargo-dist` 5 targets, per-tool tags, `SHA256SUMS`; `install.sh` (subsets, platforms, uninstall, cargo fallback) — spec slice in the spec program | 5 verified artifacts; containerised subset × platform matrix; uninstall restores | NOT STARTED | F21 |
| F23 | retire-legacy | all | Delete `flow/engine/`, node tests, `build-plugin.sh`, old `install.sh` paths; `vibe-flow migrate`; spec slice deletes `spec/scripts/*.sh` | Full parity green; no `node` anywhere in hooks | NOT STARTED | F15, F20, F22 |
| F24 | doc-truth | all | READMEs (umbrella + per tool), CHANGELOG, root specs truth sweep; drift check extended to three tools | `check-drift` green; stranger eval from a fresh non-git target | NOT STARTED | F23 |
| F25 | spec-superpowers-default | spec | Assertive, trigger-only spec skill rewrite (pressure-tested); `using-spec` router text; superpowers-first delegate lines in `flow/`; `superpowers-frozen` write-policy rule; `docs/superpowers/` un-ignored | Pressure baseline 100 % with and without superpowers; flow + spec suites green | NOT STARTED | — |
```

- [ ] **Step 4: Update the issue catalog and Active focus**

In the catalog table replace the held line with `| F2, F4 | to file (spec program) | F9, F10, F11 | held — no issue yet |` and add `| F25 | to file (spec program) |`.

Replace the **Active focus** paragraph with:

```markdown
**Active focus.** The spec program (design: `docs/superpowers/specs/2026-10-13-spec-v04-design.md`) runs on one branch: amendment → F1 → F25 → F2 ∥ F3 → F4 → F16 → F17 → F18 → F19 → F20 → spec slices of F21–F24. F2 and F4 are un-held and scope-cut to the spec tool. F9, F10, F11 stay held; the flow (F5–F8) and instruct (F12–F15) tracks are unchanged and not part of this program.
```

- [ ] **Step 5: Run the checks**

```bash
bash .superpowers/p1/rows-check.sh
bash spec/scripts/validate.sh
bash spec/scripts/check-drift.sh
```
Expected: `OK`; both scripts exit 0.

- [ ] **Step 6: Commit**

```bash
git add .spec/plan.md
git commit -m "docs(spec): amend v0.4 rows for spec program"
```

---

### Task 3: Amend the tech docs

**Files:**
- Modify: `.spec/tech-spec.md` §§ Root and discovery, Migrate, Adherence checks, Subagents, Hooks, Commands (v1)
- Modify: `.spec/tech.md:124-132` (Spec Framework Contract layout block)
- Modify: `.spec/tech-rust.md` § Workspace (first paragraph)

**Interfaces:**
- Consumes: D44–D47.
- Produces: the command surface later phases implement — `vibe-spec promote <feature> [--dry-run|--done]`, `vibe-spec check-drift [--pre-merge]`, `vibe-spec hook {session-start,guard,validate}`, `vibe-spec migrate --to docs/spec [--layout layered]`.

- [ ] **Step 1: Write the failing check**

```bash
cat > .superpowers/p1/tech-check.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
grep -q 'Layout unchanged from today' .spec/tech.md && { echo "tech.md layout not amended"; exit 1; }
grep -q 'hook {session-start,guard,validate}' .spec/tech-spec.md || { echo "commands not amended"; exit 1; }
grep -q -- '--layout layered' .spec/tech-spec.md || { echo "migrate not amended"; exit 1; }
grep -q '^| spec-interviewer |' .spec/tech-spec.md && { echo "retired agents still listed"; exit 1; }
grep -q 'crates arrive with their rows' .spec/tech-rust.md || { echo "tech-rust not amended"; exit 1; }
echo OK
EOF
bash .superpowers/p1/tech-check.sh
```
Expected: FAIL with `tech.md layout not amended`

- [ ] **Step 2: Replace `.spec/tech.md` lines 124–132 with**

````markdown
Default root `docs/spec/`; `.spec/` legacy. Layered layout (D44):

```text
docs/spec/
├── README.md · AGENTS.md · product.md · tech.md · design.md · plan.md · lessons.md
├── areas/ flows/ concepts/ platform/ tech/ decisions/ questions/   (on demand)
├── parts/<part>/                                                   (optional)
├── features/<feature>/{product,tech,design,plan,research}.md        (fallback, D45)
└── archive/<feature>/                                              (fallback, transient)
docs/superpowers/{specs,plans}/                                     (default feature layer, D45/D46)
```
````

- [ ] **Step 3: Amend `.spec/tech-spec.md`**

Append to § Root and discovery:

```markdown
`docs/superpowers/` is a recognised artifact root (D33): `vibe-spec` reads
`specs/YYYY-MM-DD-<topic>-design.md` and `plans/YYYY-MM-DD-<topic>.md`,
binds them to a feature by topic slug, and deletes them only through
`promote <feature> --done`.
```

Append to § Migrate:

```markdown
`--layout layered` runs after the move: `{tech,product,plan}-<topic>.md`
→ `<kind>/<topic>.md`, each D-record → `decisions/dNN-<slug>.md`, hubs
seeded from templates. Splitting oversized prose into kind notes is
agent-led via the spec skill's restructure mode; the gate is
`validate --strict` green.
```

Replace the § Adherence checks allowlist sentence (from `layout allowlist` to `anything else\nfails);`) with:

```markdown
layout allowlist (root hubs + `README.md`/`AGENTS.md`/`lessons.md`,
kind folders `areas|flows|concepts|platform|tech|decisions|questions`,
`parts/<part>/` with the same kinds, `features/<name>/{product,tech,design,plan,research}.md`,
`archive/<name>/**`, `quick/<slug>.md`, `templates/**`, and
`docs/superpowers/{specs,plans}/*.md` — anything else fails); budgets
(hub ≤80, note ≤60); link resolution; unique names; typed frontmatter
with `code:` existence on areas and `applies:` on tech; every kind note
reachable from a hub;
```

Replace the § Subagents table with:

```markdown
| Agent | Model | Tools | Writes |
|---|---|---|---|
| spec-promoter | sonnet | Read, Edit, Bash(`vibe-spec promote`) | layered notes + lessons, only through the promote checklist |
| spec-health | haiku | Read, Bash(`vibe-spec validate`, `check-drift`) | nothing — returns findings |

`spec-interviewer` and `spec-tracer` are retired (D36): feature
interviews and plans run through `superpowers:brainstorming` and
`superpowers:writing-plans`.
```

Append to § Hooks:

```markdown
Router and tooth (D47): `SessionStart` (`startup|clear|compact`, sync)
runs `vibe-spec hook session-start`, which prints the `using-spec`
router inside `<EXTREMELY_IMPORTANT>`. `PreToolUse` on
`Edit|Write|MultiEdit|NotebookEdit` runs `vibe-spec hook guard`: exit 2
when the target is under the spec root or `docs/superpowers/**` and the
session transcript holds no load of the matching skill; exit 0 with one
stderr line on unreadable transcript, missing binary, no spec root, or
malformed input; `.vibe/spec.json` `"tooth": "warn"` downgrades to warn.
```

Replace the § Commands (v1) paragraph with:

```markdown
`root`, `init`, `validate [--changed|--file|--strict|--format text|json|hook]`,
`check-drift [--pre-merge]`, `promote <feature> [--dry-run|--done]`,
`scan-merges`, `lessons-for <tags>`, `list-specs`, `trace`,
`migrate --to docs/spec [--layout layered] [--dry-run]`,
`export|import --openspec`, `hook {session-start,guard,validate}`,
`setup [--git-hooks|--claude|--opencode]`, `doctor`.
```

- [ ] **Step 4: Amend `.spec/tech-rust.md` § Workspace**

Insert after the crate tree code block:

```markdown
The spec program (F2–F4, F16–F20) creates `vibe-core`, `vibe-spec` and
`vibe-parity` only; the flow, instruct and signals crates arrive with their rows.
```

- [ ] **Step 5: Run the checks**

```bash
bash .superpowers/p1/tech-check.sh
bash spec/scripts/validate.sh
```
Expected: `OK`; validate exits 0.

- [ ] **Step 6: Commit**

```bash
git add .spec/tech.md .spec/tech-spec.md .spec/tech-rust.md
git commit -m "docs(spec): amend tech docs for spec program"
```

---

### Task 4: Regenerate the roadmap issue files

**Files:**
- Modify: `docs/roadmap/issues/02-F2-contracts.md`, `04-F4-workspace-core.md` (remove `hold:` line and `> **Held.**` notice), `03-F3-oracles-parity.md`, `17-F17-spec-validators.md` … `24-F24-doc-truth.md`, `00-epic.md`
- Create: `docs/roadmap/issues/25-F25-spec-superpowers-default.md`
- Modify: `docs/roadmap/README.md` (status table + "Today:" counts)

**Interfaces:**
- Consumes: Task 2 row texts (Deliverable/Test columns are copied verbatim into each file's `## Deliverable` and `## Acceptance`).
- Produces: issue files ready for owner-confirmed posting in Phase 5.

- [ ] **Step 1: Write the failing check**

```bash
cat > .superpowers/p1/issues-check.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
d=docs/roadmap/issues
[[ -f $d/25-F25-spec-superpowers-default.md ]] || { echo "missing F25 file"; exit 1; }
grep -l '^hold:' $d/02-F2-contracts.md $d/04-F4-workspace-core.md 2>/dev/null && { echo "F2/F4 still held"; exit 1; }
n=$(grep -l 'Amended 2026-10-13' $d/*.md | wc -l | tr -d ' ')
[[ $n -ge 11 ]] || { echo "only $n files amended (floor 11)"; exit 1; }
echo OK
EOF
bash .superpowers/p1/issues-check.sh
```
Expected: FAIL with `missing F25 file`

- [ ] **Step 2: Create `docs/roadmap/issues/25-F25-spec-superpowers-default.md`**

```markdown
---
title: "F25 [spec] spec-superpowers-default: assertive skill, router text, superpowers-first flow"
id: F25
tool: spec
phase: 3
milestone: "v0.4 Phase 3 — spec"
depends_on: []
parent: epic
---

## Deliverable
Rewrite the spec skill (`spec/SKILL.md`, `feature.md`, `strategy.md`, `reference/**`, templates) in a concise, imperative, assertive voice with a trigger-only, superpowers-style description; author the `using-spec` router text; replace the redirect/skip delegate lines in `flow/{feature,strategy}.md` so superpowers writes its native artifacts under `docs/superpowers/{specs,plans}/`; add the `superpowers-frozen` write-policy rule; un-ignore `docs/superpowers/`.

## Scope
**Owns:** skill prose and frontmatter, router text, flow delegate lines and `writes:` lists, the one policy rule, the pressure-test harness.
**Does not own:** the router hook and tooth (F19), promotion (F18), layout validators (F17).

## Acceptance (evidence, not intent)
- [ ] pressure harness: every scenario passes with superpowers present and absent, isolated `CLAUDE_CONFIG_DIR`, no CLAUDE.md
- [ ] `bash tests/run.sh` and `node flow/engine/tests/run.mjs` green
- [ ] `node .agents/skills/vibe/engine/cli.mjs render --check` green
- [ ] description ≤1024 characters, starts "You MUST use this" or "Use when"

## Depends on
—

## Decisions this implements
- D45 — superpowers-first feature layer
- D47 — router + tooth (prose half)

Phase: 3 · Tool: spec · Source: `.spec/plan.md` row F25
```

- [ ] **Step 3: Un-hold F2 and F4**

Delete the `hold: architecture` frontmatter line and the `> **Held.** …` line from both files; replace each `## Deliverable` body with the Task 2 row's Deliverable cell, verbatim.

- [ ] **Step 4: Append the amendment section to F3, F16 … F24 and 00-epic**

For each file append (cell text copied verbatim from Task 2's row; for F16 and 00-epic the second bullet reads `- Unchanged deliverable; part of the spec program.`):

```markdown
## Amended 2026-10-13
- Spec program: `docs/superpowers/specs/2026-10-13-spec-v04-design.md`.
- Deliverable now: <Task 2 Deliverable cell for this row>
```

Add the same section to `02-F2-contracts.md` and `04-F4-workspace-core.md` with `- Un-held; scope-cut to the spec tool.`

- [ ] **Step 5: Update `docs/roadmap/README.md`**

Status table: F2 → `to file`, F4 → `to file`, add `| F25 | spec-superpowers-default | to file |`. Replace "Today: 19 already posted, 5 held, 0 to create" with "Today: 19 already posted, 3 held, 3 to create (F2, F4, F25)".

- [ ] **Step 6: Run the checks, leave the flow**

```bash
bash .superpowers/p1/issues-check.sh
bash spec/scripts/validate.sh
bash .agents/skills/vibe/scripts/set-state.sh idle
```
Expected: `OK`; validate exits 0; cursor idle.

- [ ] **Step 7: Commit**

```bash
git add docs/roadmap
git commit -m "docs(roadmap): regenerate issues for spec program"
```

---

### Task 5: F1 — spec scripts discover the project root

**Files:**
- Modify: `spec/scripts/validate.sh:8`, `spec/scripts/list-specs.sh:6-8`, `spec/scripts/lessons-for.sh:18`, `spec/scripts/scan-merges.sh:17`
- Test: `spec/tests/run.sh` (new functions before the `=== results` line)

**Interfaces:**
- Consumes: none.
- Produces: bash function `find_spec_root` (prints the nearest ancestor of `$PWD` holding `.spec/`, stopping at the directory holding `.git`; returns 1 if none). Every script `cd`s to that root before reading `.spec/`; `SPEC_DIR` env still overrides in `lessons-for.sh`/`scan-merges.sh`.

- [ ] **Step 1: Write the failing tests**

Add before the `echo "=== results` line in `spec/tests/run.sh`:

```bash
# ── F1: spec scripts resolve the project root from cwd, never from their own path ──
test_f1_subdir_invocation() {
  local out rc
  out="$(cd "$REPO_ROOT/flow" && bash ../spec/scripts/validate.sh 2>&1)"; rc=$?
  if [[ $rc -eq 0 ]] && ! grep -qF "No .spec/ directory found" <<<"$out"; then
    pass F1 "validate.sh from flow/ resolves the repo .spec/"
  else
    fail F1 "validate.sh from flow/ resolves the repo .spec/ (rc=$rc)"
  fi
}
test_f1_subdir_invocation

test_f1_plugin_cache_layout() {
  local tmp proj cache s out rc bad=0
  tmp="$(mktemp -d)"; proj="$tmp/proj"; cache="$tmp/cache/spec"
  mkdir -p "$proj/src/deep" "$cache"
  cp -R "$REPO_ROOT/spec/scripts" "$cache/scripts"
  (cd "$proj" && git init -q . && bash "$cache/scripts/setup.sh" >/dev/null 2>&1) || true
  [[ -d "$proj/.spec" ]] || mkdir -p "$proj/.spec"
  printf '%s\n' '# Lessons' '' '### x' '**Tags:** alpha' >"$proj/.spec/lessons.md"
  for s in validate list-specs lessons-for scan-merges; do
    case "$s" in lessons-for) set -- alpha ;; *) set -- ;; esac
    out="$(cd "$proj/src/deep" && bash "$cache/scripts/$s.sh" "$@" 2>&1)"; rc=$?
    if [[ $rc -ne 0 && $rc -ne 1 ]] || grep -qF "No .spec/ directory found" <<<"$out"; then
      bad=$((bad + 1)); echo "        offender: $s (rc=$rc)"
    fi
  done
  rm -rf "$tmp"
  [[ $bad -eq 0 ]] && pass F1 "4 scripts from a plugin-cache path + nested cwd find the project .spec/" \
    || fail F1 "$bad script(s) failed to find the project .spec/ from a nested cwd"
}
test_f1_plugin_cache_layout

test_f1_git_boundary() {
  local tmp out
  tmp="$(mktemp -d)"; mkdir -p "$tmp/outer/.spec" "$tmp/outer/inner/sub"
  (cd "$tmp/outer/inner" && git init -q .)
  out="$(cd "$tmp/outer/inner/sub" && bash "$REPO_ROOT/spec/scripts/list-specs.sh" 2>&1 || true)"
  rm -rf "$tmp"
  if grep -qF "No .spec/ directory found" <<<"$out"; then
    pass F1 "root search stops at the .git boundary (outer .spec/ not used)"
  else
    fail F1 "root search crossed the .git boundary"
  fi
}
test_f1_git_boundary
```

- [ ] **Step 2: Run to verify they fail**

Run: `bash spec/tests/run.sh 2>&1 | grep -E '^\s*(PASS|FAIL).*F1'`
Expected: `FAIL F1 validate.sh from flow/ …` and `FAIL F1 … plugin-cache …`

- [ ] **Step 3: Implement — add to the top of each of the 4 scripts, right after `set -euo pipefail`**

```bash
# Resolve the project's spec root from the caller's cwd (never from this
# script's own path — under a plugin install the script lives in a cache).
# Walk up to the directory holding .git, no further (D33).
find_spec_root() {
  local d; d="$(pwd -P)"
  while :; do
    if [[ -d "$d/.spec" ]]; then printf '%s\n' "$d"; return 0; fi
    [[ -e "$d/.git" || "$d" == "/" ]] && return 1
    d="$(dirname "$d")"
  done
}
if SPEC_ROOT_DIR="$(find_spec_root)"; then cd "$SPEC_ROOT_DIR"; fi
```

`lessons-for.sh:18` and `scan-merges.sh:17` keep `${SPEC_DIR:-.spec}` unchanged (they now run from the root). `validate.sh:8` and `list-specs.sh` keep `.spec` relative paths unchanged.

- [ ] **Step 4: Run to verify they pass, plus shellcheck**

```bash
bash spec/tests/run.sh 2>&1 | tail -3
shellcheck spec/scripts/*.sh
```
Expected: `=== results: N passed, 0 failed ===`; shellcheck prints nothing.

- [ ] **Step 5: Commit**

```bash
git add spec/scripts spec/tests/run.sh
git commit -m "fix(spec): resolve spec root from caller cwd"
```

---

### Task 6: F1 — skill-relative script references + widened guard

**Files:**
- Modify: `spec/SKILL.md:319`, `spec/agents/spec-promoter/SKILL.md:16`, `:19`
- Test: `spec/tests/run.sh` — replace `test_skill_validator_paths_relative` (lines ~1478–1500)

**Interfaces:**
- Consumes: Task 5 root discovery (scripts now work from any cwd).
- Produces: guard test `test_skill_script_paths_relative` with a population floor.

- [ ] **Step 1: Replace the guard test with the widened version**

```bash
test_skill_script_paths_relative() {
  local f bad=0 scanned=0 floor names=()
  local s; for s in "$REPO_ROOT"/spec/scripts/*.sh; do names+=("$(basename "$s")"); done
  floor="$(find "$REPO_ROOT/spec" -name '*.md' -not -path '*/tests/*' | wc -l | tr -d ' ')"
  while IFS= read -r f; do
    scanned=$((scanned + 1))
    # shellcheck disable=SC2088
    # Ban the capability (any install-layout path to the skill), not one spelling.
    if grep -qF '.agents/skills/spec' "$f" || grep -qF '~/.agents/skills/' "$f"; then
      bad=$((bad + 1)); echo "        offender: ${f#"$REPO_ROOT"/}"
    fi
  done < <(find "$REPO_ROOT/spec" -name '*.md' -not -path '*/tests/*')
  if [[ ${#names[@]} -lt 7 || $scanned -lt 10 || $scanned -ne $floor ]]; then
    fail "install-agnostic-paths" "guard examined too little (md=$scanned/$floor, scripts=${#names[@]})"
  elif [[ $bad -eq 0 ]]; then
    pass "install-agnostic-paths" "$scanned spec docs reference ${#names[@]} scripts skill-relative"
  else
    fail "install-agnostic-paths" "$bad spec doc(s) hard-code an install-absolute script path"
  fi
}
test_skill_script_paths_relative
```

- [ ] **Step 2: Run to verify it fails**

Run: `bash spec/tests/run.sh 2>&1 | grep install-agnostic`
Expected: `FAIL … 2 spec doc(s) hard-code …` with offenders `spec/SKILL.md`, `spec/agents/spec-promoter/SKILL.md`

- [ ] **Step 3: Fix the three references**

- `spec/SKILL.md:319` → `` !`bash "${CLAUDE_SKILL_DIR}/scripts/list-specs.sh"` `` (Claude Code substitutes `${CLAUDE_SKILL_DIR}` as text before running `!` commands, for project, personal and plugin skills — https://code.claude.com/docs/en/skills; no install-layout fallback)
- `spec/agents/spec-promoter/SKILL.md:16` → ``1. Run `bash scripts/scan-merges.sh <name>` (path relative to the spec skill) — display the block count, line ranges, and preview to the user``
- `spec/agents/spec-promoter/SKILL.md:19` → ``4. On yes: run `bash scripts/promote.sh <name>` (path relative to the spec skill)``

- [ ] **Step 4: Run the suites; grep the whole spec tree for the banned capability**

```bash
bash spec/tests/run.sh 2>&1 | tail -2
bash tests/run.sh 2>&1 | tail -2
grep -rnF '.agents/skills/spec' spec --include='*.md' | grep -v '/tests/' || echo "none"
```
Expected: suites green; last command prints `none`. Any remaining hit (e.g. in `spec/README.md`) is fixed to a skill-relative path in this step.

- [ ] **Step 5: Commit**

```bash
git add spec/SKILL.md spec/agents/spec-promoter/SKILL.md spec/tests/run.sh
git commit -m "fix(spec): use skill-relative script paths"
```

---

### Task 7: F25 — `superpowers-frozen` write-policy rule

**Files:**
- Modify: `flow/content/policy.json` (new rule after `features-frozen`)
- Modify: `flow/scripts/detect-context.sh` (new `case` arm after the `.spec/features/*` arm, ~line 235)
- Modify: `flow/engine/tests/policy.test.mjs:600-606` (`6 rule(s)` → `7 rule(s)`, id list)
- Modify: `flow/engine/policy.mjs:1`, `:21` (comments say "the six" → "the seven"; a stale safety comment is a defect)
- Modify: `flow/reference/templates/AGENTS.md:131` (rendered rules list; add the new rule line after it)
- Check: `flow/engine/tests/{hook,parity,orders}.test.mjs` reference `.spec/features` fixtures — they must stay green unchanged
- Test: `flow/engine/tests/policy.test.mjs` (new test after line ~152), `flow/tests/run.sh` (asserts after line ~899)
- Regenerate: `AGENTS.md` `vibe:rules` block

**Interfaces:**
- Consumes: none.
- Produces: policy rule id `superpowers-frozen`, match `docs/superpowers/specs/*`, warn in `feature.impl`/`quick.fix`, allow otherwise.

- [ ] **Step 1: Write the failing JS test**

```js
test('shipped: docs/superpowers/specs/* warns during impl/fix (nested too), allows otherwise', () => {
  const { rules } = loadPolicy(REPO_VIBE_DIR);
  const policy = { rules };
  for (const file of ['docs/superpowers/specs/2026-10-13-x-design.md', 'docs/superpowers/specs/sub/y-design.md']) {
    const warned = decide(policy, file, 'feature.impl');
    assertEqual(warned.verdict, 'warn');
    assertEqual(
      warned.reason,
      'superpowers design docs are frozen during impl/fix — route back to feature.design to change scope (current: feature.impl)',
    );
    assertEqual(decide(policy, file, 'quick.fix').verdict, 'warn');
    assertEqual(decide(policy, file, 'feature.design').verdict, 'allow');
    assertEqual(decide(policy, file, 'idle').verdict, 'allow');
  }
  assertEqual(decide(policy, 'docs/superpowers/plans/2026-10-13-x.md', 'feature.impl').verdict, 'allow');
});
```

And in `flow/tests/run.sh` after the `.spec/features` block:

```bash
out="$(bash "$DETECT" decide docs/superpowers/specs/2026-10-13-w-design.md feature.impl)"
assert_contains "spec-v04/F25" "decide superpowers spec warns under feature.impl" "$out" "warn:"
out="$(bash "$DETECT" decide docs/superpowers/specs/sub/w-design.md quick.fix)"
assert_contains "spec-v04/F25" "decide nested superpowers spec warns under quick.fix" "$out" "warn:"
out="$(bash "$DETECT" decide docs/superpowers/specs/2026-10-13-w-design.md feature.design)"
assert_eq "spec-v04/F25" "decide superpowers spec allows under feature.design" "$out" "allow"
```

Update the CLI test: `assertIncludes(result.stdout, '7 rule(s)');` and add `'superpowers-frozen'` to the id list.

- [ ] **Step 2: Run to verify they fail**

```bash
node flow/engine/tests/run.mjs 2>&1 | grep -E 'superpowers|rule\(s\)'
bash flow/tests/run.sh 2>&1 | grep 'spec-v04/F25'
```
Expected: FAIL lines for the new test, the `7 rule(s)` assertion, and all three bash asserts.

- [ ] **Step 3: Add the rule to `flow/content/policy.json`**

```json
{
  "id": "superpowers-frozen",
  "match": "docs/superpowers/specs/*",
  "arms": [
    { "states": ["feature.impl", "quick.fix"], "verdict": "warn", "reason": "superpowers design docs are frozen during impl/fix — route back to feature.design to change scope (current: {state})" },
    { "states": "*", "verdict": "allow", "reason": "" }
  ]
},
```

- [ ] **Step 4: Add the bash arm to `flow/scripts/detect-context.sh` after the `.spec/features/*` arm**

```bash
  # Warning: superpowers design docs are frozen once implementation begins (D45).
  case "$path" in
    docs/superpowers/specs/*|*/docs/superpowers/specs/*)
      case "$state" in
        feature.impl|quick.fix) echo "warn:superpowers design docs are frozen during impl/fix — route back to feature.design to change scope (current: $state)" ;;
        *) echo "allow" ;;
      esac
      return 0
      ;;
  esac
```

- [ ] **Step 5: Run the suites**

```bash
node flow/engine/tests/run.mjs 2>&1 | tail -3
bash flow/tests/run.sh 2>&1 | tail -2
shellcheck flow/scripts/detect-context.sh
```
Expected: all green; parity tests included in the engine run stay green.

- [ ] **Step 5b: Sync the template and comments**

Insert after `flow/reference/templates/AGENTS.md:131`:

```markdown
- `docs/superpowers/specs/*` — `feature.impl`, `quick.fix`: warn — superpowers design docs are frozen during impl/fix — route back to feature.design to change scope (current: the current state); otherwise: allow
```

In `flow/engine/policy.mjs` lines 1 and 21 replace `six` with `seven`. Then `grep -rn "\bsix\b" flow/engine/policy.mjs flow/engine/commands/policy.mjs` must print nothing that counts rules.

- [ ] **Step 6: Re-render AGENTS.md and check**

```bash
node .agents/skills/vibe/engine/cli.mjs render agents-md --write
node .agents/skills/vibe/engine/cli.mjs render --check
grep -c 'docs/superpowers/specs/\*' AGENTS.md
```
Expected: render check exits 0; grep prints `1`.

- [ ] **Step 7: Commit**

```bash
git add flow/content/policy.json flow/scripts/detect-context.sh flow/engine/policy.mjs flow/engine/tests/policy.test.mjs flow/tests/run.sh flow/reference/templates/AGENTS.md AGENTS.md
git commit -m "feat(flow): freeze superpowers specs during impl"
```

---

### Task 8: F25 — superpowers-first delegate lines

**Files:**
- Modify: `flow/feature.md:25-30` (brainstorming delegate), `:46-50` (writing-plans delegate), `:90` (subagent-driven inject path)
- Modify: `flow/strategy.md:21-22`
- Modify: `flow/state-machine.json:65` (`feature.design` writes), `:74` (`feature.plan` writes), `:77` (`feature.plan` exit)
- Modify: `flow/SKILL.md` orders blocks `feature.design`, `feature.plan`, `feature.impl`, `feature.compound` (injected every turn; must match the delegate lines)
- Modify: `flow/reference/templates/AGENTS.md:16` and the managed block in `AGENTS.md` (session-start feature lookup)
- Check: `flow/engine/tests/orders.test.mjs:627` expects `.spec/features/0/plan.md` in feature.plan orders — the fallback path stays in the text
- Test: `flow/tests/run.sh` (asserts appended to the flow-legibility section, ~line 460)

**Interfaces:**
- Consumes: D45 fallback triggers.
- Produces: `feature.design` writes `docs/superpowers/specs/*-<feature>-design.md`; `feature.plan` writes `docs/superpowers/plans/*-<feature>.md`; fallback writes unchanged `.spec/features/<feature>/…`.

- [ ] **Step 1: Write the failing asserts**

```bash
feat_body="$(cat "$FLOW/feature.md")"
assert_not_contains "spec-v04/F25" "feature.md no longer skips the superpowers spec write" "$feat_body" "skip: its terminal write under \`docs/superpowers/specs/\`"
assert_not_contains "spec-v04/F25" "feature.md no longer skips the superpowers plan path" "$feat_body" "skip: its own \`docs/superpowers/plans/\` path"
assert_contains "spec-v04/F25" "feature.md names the fallback triggers" "$feat_body" "fallback → \`.spec/features/<name>/\` only when"
machine="$(cat "$FLOW/state-machine.json")"
assert_contains "spec-v04/F25" "feature.design writes the superpowers spec" "$machine" "docs/superpowers/specs/*-<feature>-design.md"
assert_contains "spec-v04/F25" "feature.plan writes the superpowers plan" "$machine" "docs/superpowers/plans/*-<feature>.md"
```

- [ ] **Step 2: Run to verify they fail**

Run: `bash flow/tests/run.sh 2>&1 | grep 'spec-v04/F25'`
Expected: 5 FAIL lines.

- [ ] **Step 3: Replace `flow/feature.md` lines 29–30**

```markdown
   > - output: superpowers' own design doc at `docs/superpowers/specs/YYYY-MM-DD-<name>-design.md` (topic = `<name>`); keep its self-commit; stop at its writing-plans handoff — `feature.plan` runs it
   > - fallback → `.spec/features/<name>/{product,tech,design}.md` only when: superpowers absent, user says *self*, `.vibe/spec.json` `"featureLayer": "vibe"`, or OpenSpec interchange needed
```

- [ ] **Step 4: Replace `flow/feature.md` lines 49–50**

```markdown
   > - output: superpowers' own plan at `docs/superpowers/plans/YYYY-MM-DD-<name>.md`; its `**Spec:**` line points at the design doc
   > - its execution-choice question IS the plan gate: record the answer, then `/flow feature.impl <name> confirm`
   > - fallback → `.spec/features/<name>/plan.md` with stable `<name>/n` unit IDs, same triggers as design
```

- [ ] **Step 5: Replace `flow/feature.md:90` inject path**

```markdown
   > - inject: the superpowers plan `docs/superpowers/plans/YYYY-MM-DD-<name>.md` (fallback: `.spec/features/<name>/plan.md`); work on the feature branch from the plan gate (worktree only if offered+accepted); runtime artifacts stay under `.superpowers/**` (gitignored runtime, not memory)
```

- [ ] **Step 6: Replace `flow/strategy.md:21-22`**

```markdown
   > - output: nothing written here; the direction lands in the NEXT state (`strategy.spec`) as root spec edits
   > - skip: its terminal design-doc write — strategy direction is recorded in the spec, not as a superpowers feature artifact
```

- [ ] **Step 7: Update `flow/state-machine.json`**

Line 65 → `"writes": ["docs/superpowers/specs/*-<feature>-design.md", ".spec/features/<feature>/product.md", ".spec/features/<feature>/tech.md", ".spec/features/<feature>/design.md", ".spec/features/<feature>/research.md"],`
Line 74 → `"writes": ["docs/superpowers/plans/*-<feature>.md", ".spec/features/<feature>/plan.md"],`
Line 77 → `"exit": "superpowers plan (or fallback plan.md with stable unit IDs) written; human approved moving to impl"`

- [ ] **Step 7b: Rewrite the four orders blocks in `flow/SKILL.md` (whole line between the markers)**

```text
<!-- vibe:orders:feature.design -->
skill=vibe · delegate superpowers:brainstorming (topic=<feature>) · READ lessons.md + root product/tech · WRITE docs/superpowers/specs/*-<feature>-design.md (fallback: .spec/features/<feature>/{product,tech,design?,research?}.md) · no source · done → set-state.sh feature.plan
<!-- /vibe:orders -->

<!-- vibe:orders:feature.plan -->
skill=vibe · delegate superpowers:writing-plans · WRITE docs/superpowers/plans/*-<feature>.md (fallback: .spec/features/<feature>/plan.md with STABLE unit IDs <feature>/n) · verification per task · no source · HUMAN GATE before impl · gate: plan-approval+mode · on approval → /flow feature.impl confirm
<!-- /vibe:orders -->
```

In `feature.impl` replace `do NOT edit .spec/**` with `do NOT edit .spec/** or docs/superpowers/specs/**` and `cite plan unit IDs (<feature>/n)` with `cite plan task numbers (fallback: unit IDs <feature>/n)`.

In `feature.compound` insert after `WRITE tagged .spec/lessons.md, ` the text `promote durable facts from docs/superpowers/{specs,plans}/*-<feature>* into root specs, then git rm them, `.

Replace `flow/reference/templates/AGENTS.md:16` with:

```markdown
   `docs/superpowers/specs/*-<name>-design.md` + `docs/superpowers/plans/*-<name>.md`, or the fallback `.spec/features/<name>/{product,tech,plan}.md`.
```

Then refresh the managed block: `bash .agents/skills/vibe/scripts/merge-agents.sh` (the block in `AGENTS.md` must stay byte-identical to the template).

- [ ] **Step 8: Run the suites**

```bash
bash flow/tests/run.sh 2>&1 | tail -2
node flow/engine/tests/run.mjs 2>&1 | tail -2
bash tests/run.sh 2>&1 | tail -2
```
Expected: all green.

- [ ] **Step 9: Commit**

```bash
git add flow/feature.md flow/strategy.md flow/state-machine.json flow/SKILL.md flow/reference/templates/AGENTS.md AGENTS.md flow/tests/run.sh
git commit -m "feat(flow): default feature layer to superpowers"
```

---

### Task 9: F25 — pressure-test harness and RED baseline

**Files:**
- Create: `spec/tests/pressure/run.sh` (harness), `spec/tests/pressure/scenarios.tsv`, `spec/tests/pressure/parse.mjs`
- Create: `spec/tests/pressure/baseline-red.md` (recorded results + verbatim rationalizations)

**Interfaces:**
- Consumes: `plugin/` (Claude plugin dir whose `skills/spec` symlinks to `spec/`).
- Produces: `bash spec/tests/pressure/run.sh [--sp present|absent|both]` → prints `PASS|FAIL <id> <sp> first-skill=<name|none>` per scenario and `pressure: N/M passed`; exit 0 only when all pass; exit 2 when neither `ANTHROPIC_API_KEY` nor `CLAUDE_CODE_OAUTH_TOKEN` is set (never a silent skip). Not wired into CI (costs tokens; ~12 sessions per run).
- Isolation: NOT `--bare` (bare mode withholds the skill listing, so model-driven invocation is impossible — https://code.claude.com/docs/en/headless). Instead each session runs with `CLAUDE_CONFIG_DIR` set to an empty temp dir (no user CLAUDE.md, settings, hooks or installed plugins) plus token auth, and loads only a throwaway plugin that contains just `skills/spec` (so the flow `vibe` skill cannot confound the result).

- [ ] **Step 1: Write `spec/tests/pressure/scenarios.tsv`**

```text
id	accept	prompt
flag	spec|superpowers:brainstorming	Add a --verbose flag to src/cli.sh.
prd	spec|superpowers:brainstorming	Write a PRD for exporting reports as CSV.
refactor	spec|superpowers:brainstorming|superpowers:writing-plans	Plan the refactor of src/cli.sh into modules.
quick	spec|superpowers:brainstorming|superpowers:systematic-debugging	Quick fix: the greeting in src/cli.sh says "Helo". Just fix it.
howbuild	spec|superpowers:brainstorming	How should we build user authentication here?
archnote	spec	Update the architecture notes to say we store data in SQLite.
```

With `--sp absent`, the accept set for every row is `spec` only.

- [ ] **Step 2: Write `spec/tests/pressure/parse.mjs`**

```js
// Reads Claude stream-json on stdin; prints the first Skill name invoked
// before any Edit/Write/MultiEdit/NotebookEdit, or "none".
import { createInterface } from 'node:readline';
const WRITES = new Set(['Edit', 'Write', 'MultiEdit', 'NotebookEdit']);
let first = null;
for await (const line of createInterface({ input: process.stdin })) {
  if (first) continue;
  let ev; try { ev = JSON.parse(line); } catch { continue; }
  const content = ev?.message?.content;
  if (ev.type !== 'assistant' || !Array.isArray(content)) continue;
  for (const c of content) {
    if (c.type !== 'tool_use') continue;
    if (c.name === 'Skill') { const n = String(c.input?.skill ?? 'none'); first = /(^|:)spec$/.test(n) ? 'spec' : n; break; }
    if (WRITES.has(c.name)) { first = 'none'; break; }
  }
}
process.stdout.write(`${first ?? 'none'}\n`);
```

- [ ] **Step 3: Write `spec/tests/pressure/run.sh`**

```bash
#!/usr/bin/env bash
# Pressure-tests the spec skill: fresh headless sessions with no CLAUDE.md,
# no user settings, no hooks (empty CLAUDE_CONFIG_DIR). Passes only if the first skill
# invoked — before any write — is in the scenario's accept set.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
SP_DIR="${SUPERPOWERS_DIR:-$(ls -d "$HOME"/.claude/plugins/cache/claude-plugins-official/superpowers/*/ 2>/dev/null | sort -V | tail -1)}"
mode="both"; [[ "${1:-}" == "--sp" ]] && mode="$2"
[[ -n "${ANTHROPIC_API_KEY:-}${CLAUDE_CODE_OAUTH_TOKEN:-}" ]] || { echo "pressure: set ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN (claude setup-token); not run" >&2; exit 2; }

sandbox() { # prints: <project dir> <plugin dir> <config dir>
  local d; d="$(mktemp -d)"
  mkdir -p "$d/proj/src" "$d/plugin/.claude-plugin" "$d/plugin/skills" "$d/config"
  printf '#!/usr/bin/env bash\necho "Helo"\n' >"$d/proj/src/cli.sh"
  printf '{"name":"spec-under-test","version":"0.0.0"}\n' >"$d/plugin/.claude-plugin/plugin.json"
  ln -s "$REPO/spec" "$d/plugin/skills/spec"
  (cd "$d/proj" && git init -q . && bash "$REPO/spec/scripts/setup.sh" >/dev/null 2>&1 || true)
  printf '%s %s %s\n' "$d/proj" "$d/plugin" "$d/config"
}
claude_iso() { # cfg plugin-args... -- prompt
  local cfg="$1"; shift
  CLAUDE_CONFIG_DIR="$cfg" claude -p "$@" --permission-mode plan --output-format stream-json --verbose
}

pass=0 total=0
run_one() { # id accept prompt sp
  local id="$1" accept="$2" prompt="$3" sp="$4" proj plug cfg first args=()
  [[ "$sp" == absent ]] && accept="spec"
  read -r proj plug cfg < <(sandbox); args=(--plugin-dir "$plug")
  [[ "$sp" == present && -n "$SP_DIR" ]] && args+=(--plugin-dir "$SP_DIR")
  first="$(cd "$proj" && claude_iso "$cfg" "${args[@]}" "$prompt" 2>/dev/null | node "$HERE/parse.mjs")"
  rm -rf "$(dirname "$proj")"; total=$((total + 1))
  if [[ "|$accept|" == *"|$first|"* ]]; then pass=$((pass + 1)); echo "PASS $id $sp first-skill=$first"
  else echo "FAIL $id $sp first-skill=$first (accept: $accept)"; fi
}

sps=(present absent); [[ "$mode" != both ]] && sps=("$mode")
while IFS=$'\t' read -r id accept prompt; do
  [[ "$id" == id ]] && continue
  for sp in "${sps[@]}"; do run_one "$id" "$accept" "$prompt" "$sp"; done
done <"$HERE/scenarios.tsv"
[[ $total -ge 6 ]] || { echo "pressure: only $total scenarios ran (floor 6)"; exit 1; }
echo "pressure: $pass/$total passed"
[[ $pass -eq $total ]]
```

- [ ] **Step 4: Verify the harness is honest before trusting it**

```bash
shellcheck spec/tests/pressure/run.sh
env -u ANTHROPIC_API_KEY -u CLAUDE_CODE_OAUTH_TOKEN bash spec/tests/pressure/run.sh; echo "rc=$?"
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Write","input":{}}]}}' | node spec/tests/pressure/parse.mjs
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"spec-under-test:spec"}}]}}' | node spec/tests/pressure/parse.mjs
```
Expected: shellcheck silent; `rc=2` with the "not run" line; `none`; `spec`.

- [ ] **Step 5: Run the RED baseline (current skill, unchanged)**

Run: `bash spec/tests/pressure/run.sh --sp both | tee .superpowers/p1/pressure-red.txt`
Expected: at least one FAIL (the current description is a workflow summary, not a trigger). If neither token is set, stop and ask the owner to run `claude setup-token` and export `CLAUDE_CODE_OAUTH_TOKEN` (or export `ANTHROPIC_API_KEY`) for this step and Tasks 10–11.

Isolation check, before trusting any RED result — run one sandboxed session and inspect its `system`/`init` line:

```bash
d="$(mktemp -d)"; mkdir -p "$d/c" "$d/p/.claude-plugin" "$d/p/skills"; ln -s "$PWD/spec" "$d/p/skills/spec"
printf '{"name":"spec-under-test","version":"0.0.0"}\n' >"$d/p/.claude-plugin/plugin.json"
(cd "$d" && CLAUDE_CONFIG_DIR="$d/c" claude -p --plugin-dir "$d/p" --permission-mode plan --output-format stream-json --verbose "Write a PRD for CSV export." ) >"$d/out.jsonl"
head -1 "$d/out.jsonl" | node -e 'const e=JSON.parse(require("fs").readFileSync(0));console.log(JSON.stringify({plugins:e.plugins?.map(p=>p.name),skills:e.skills,memory:e.memory_paths}))'
node spec/tests/pressure/parse.mjs <"$d/out.jsonl"
```
Expected: `plugins` lists only `spec-under-test`; no user-level CLAUDE.md is loaded; the skill appears in `skills` (if the CLI omits that field, the parse line printing `spec` is the proof). If `none` is printed and the skill is listed, plan mode may block the Skill tool: replace `--permission-mode plan` in `claude_iso` with `--disallowedTools Edit,Write,MultiEdit,NotebookEdit` and rerun.

- [ ] **Step 6: Record rationalizations verbatim**

For every FAIL, rerun that scenario with `--output-format stream-json` saved to a file and copy the assistant's text preceding its first tool call into `spec/tests/pressure/baseline-red.md` as a table `| scenario | sp | first-skill | agent's words (verbatim) |`.

- [ ] **Step 7: Commit**

```bash
git add spec/tests/pressure
git commit -m "test(spec): add skill pressure-test harness"
```

---

### Task 10: F25 — rewrite the spec skill (GREEN → REFACTOR)

**Files:**
- Modify: `spec/SKILL.md` (frontmatter `description`; body restructured, ≤250 lines)
- Create: `spec/reference/using-spec.md` (router text; F19 wires it into the hook)
- Test: `spec/tests/run.sh` (new `test_sf17_description_budget`)

**Interfaces:**
- Consumes: `baseline-red.md` rationalizations (Task 9); fallback triggers (D45).
- Produces: frontmatter keys unchanged except `description`; body sections in this order: `<HARD-GATE>`, The Rule, Routing, Red Flags, Layers, Commands, Fallback feature spec, Config. `spec/reference/using-spec.md` ≤150 words with a `<SUBAGENT-STOP>` block.

- [ ] **Step 1: Write the failing frontmatter test**

```bash
test_sf17_description_budget() {
  local desc chars
  desc="$(awk '/^description:/{f=1; sub(/^description:[[:space:]]*\|?[[:space:]]*/,""); if(length) print; next} f && /^[a-z_-]+:/{exit} f{sub(/^[[:space:]]+/,""); print}' "$SPEC_SKILL/SKILL.md" | tr '\n' ' ')"
  chars="${#desc}"
  if [[ $chars -gt 20 && $chars -le 1024 ]] && grep -qE '^(You MUST use this|Use when)' <<<"$desc" && ! grep -qF 'Maintains' <<<"$desc"; then
    pass SF17 "description is trigger-first and within budget ($chars chars)"
  else
    fail SF17 "description is trigger-first and within budget ($chars chars)"
  fi
}
test_sf17_description_budget
```

Run: `bash spec/tests/run.sh 2>&1 | grep 'trigger-first'`
Expected: FAIL (current description starts "Maintains").

- [ ] **Step 2: Replace the `description` block**

```yaml
description: |
  You MUST use this before changing what the product does, how it is built,
  or any file under docs/spec, .spec or docs/superpowers — features,
  refactors, architecture, decisions, PRDs, plans, roadmaps. Use before
  writing code, before brainstorming alone, before answering "how should
  we build X", and whenever the user mentions spec, PRD, design doc, plan,
  or roadmap.
```

- [ ] **Step 3: Rewrite the body opening (replace everything from the first `#` heading through the end of `## Superpower tips`)**

```markdown
# spec

<HARD-GATE>
Do not write code, plans, or design docs until this skill has routed the
work. Routing is not optional. A reply to the user is not routing.
</HARD-GATE>

## The Rule

1. Invoke this skill before you act on any change to behaviour, structure, or docs.
2. Read `lessons.md` and the hub of the area you touch.
3. Route the work by the table below. Announce the route in one line.
4. Write only where the route says. Nowhere else.

## Routing

| Work | Do this | Writes |
|---|---|---|
| Strategy: vision, areas, tech directives, decisions | Edit the spec yourself (dialogue via `superpowers:brainstorming` allowed) | spec root only |
| Feature, new or structural | Invoke `superpowers:brainstorming`, then `superpowers:writing-plans`. Topic = feature name. | `docs/superpowers/specs/`, `docs/superpowers/plans/` |
| Feature, bounded | Invoke `superpowers:brainstorming` (bounded path) | nothing |
| Fallback | Run `feature.md` | `features/<name>/` |

Fallback only when: superpowers is not installed, the user says *self*,
`.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec interchange is required.

## Red Flags

| Thought | Reality |
|---|---|
| "This is too small for a spec" | Small work routes to the bounded path. Route it. |
| "I'll update the spec after the code" | The spec leads. Route first. |
| "The user only asked a question" | "How should we build X" is design. Route it. |
| "I know this codebase" | Lessons exist because agents knew the codebase. Read them. |
| "Superpowers will handle it" | Superpowers designs. This skill decides where it is recorded. Invoke both. |
```

Then append one Red Flags row per verbatim rationalization in `baseline-red.md`, worded `| "<their words>" | <one-line correction> |`.

- [ ] **Step 4: Compress the remaining sections**

Rewrite every remaining section (`Navigation Rules` … `Validation`) as imperative one-rule lines: delete hedges ("should", "consider", "you may want to"), delete history, keep every command and path byte-exact. Remove `## Roles` content that the Routing table now states. Keep a `## Layers` section that links `reference/product.md`, `reference/tech.md`, `reference/design.md`, `reference/plan.md`, `strategy.md` and `feature.md` — tests SF1 and SF17 pin those links. Keep the `!`-exec line from Task 6. Target ≤250 lines total.

- [ ] **Step 5: Create `spec/reference/using-spec.md`**

```markdown
<SUBAGENT-STOP>
If you were dispatched as a subagent for a specific task, skip this.
</SUBAGENT-STOP>

# Using spec

This repo records its product, architecture and decisions in a spec.
If there is even a 1 % chance the work changes behaviour, structure, or
docs, invoke the `spec` skill BEFORE any other action — including
clarifying questions and exploration.

Order: `spec` decides where the work is recorded. Superpowers process
skills decide how a feature is designed. Superpowers missing → `spec`
runs its own feature flow.

| Thought | Reality |
|---|---|
| "Just a quick change" | Quick changes route too. |
| "I'll check files first" | Route first; the skill tells you which files. |
| "The spec is out of date anyway" | Then the route includes fixing it. |
```

- [ ] **Step 6: Run GREEN**

```bash
bash spec/tests/run.sh 2>&1 | tail -2
bash spec/tests/pressure/run.sh --sp both | tee .superpowers/p1/pressure-green.txt
```
Expected: spec suite green; `pressure: 12/12 passed`.

- [ ] **Step 7: REFACTOR until stable**

For each FAIL: capture the new rationalization verbatim, add a Red Flags row (or sharpen the description), rerun the failing scenario three times. Stop when two consecutive full runs print `12/12`. Append both runs' output to `spec/tests/pressure/baseline-red.md` under `## GREEN`.

- [ ] **Step 8: Commit**

```bash
git add spec/SKILL.md spec/reference/using-spec.md spec/tests/run.sh spec/tests/pressure/baseline-red.md
git commit -m "feat(spec): rewrite skill as assertive router"
```

---

### Task 11: F25 — rewrite companion docs in the same voice

**Files:**
- Modify: `spec/feature.md` (fallback-only framing; first line states when it applies)
- Modify: `spec/strategy.md`, `spec/reference/{product,tech,design,plan}.md`, `spec/reference/templates/*.md` (voice only; no structure change — F17/F20 own structure)
- Test: `spec/tests/run.sh` (new `test_f25_voice`)

**Interfaces:**
- Consumes: Task 10 Routing table wording.
- Produces: `spec/feature.md` first non-frontmatter line reads exactly `Fallback feature flow. Run this only when the spec skill's Routing table sends you here.`

- [ ] **Step 1: Write the failing voice test**

```bash
test_f25_voice() {
  local f hits=0 scanned=0
  for f in "$SPEC_SKILL"/SKILL.md "$SPEC_SKILL"/feature.md "$SPEC_SKILL"/strategy.md "$SPEC_SKILL"/reference/*.md; do
    [[ -f "$f" ]] || continue; scanned=$((scanned + 1))
    hits=$((hits + $(grep -ciE '\b(you may want to|consider (using|adding)|it might be|feel free|optionally consider)\b' "$f" || true)))
  done
  grep -qxF 'Fallback feature flow. Run this only when the spec skill'"'"'s Routing table sends you here.' "$SPEC_SKILL/feature.md" || hits=$((hits + 1))
  if [[ $scanned -lt 7 ]]; then fail F25 "voice check examined $scanned files (floor 7)"
  elif [[ $hits -eq 0 ]]; then pass F25 "skill docs use imperative voice ($scanned files)"
  else fail F25 "$hits hedge phrase(s) or missing fallback header"; fi
}
test_f25_voice
```

Run: `bash spec/tests/run.sh 2>&1 | grep F25`
Expected: FAIL.

- [ ] **Step 2: Rewrite**

Add the exact fallback header line to `spec/feature.md` directly under its H1. In every listed file: one rule per line, imperative mood, no hedges, no history; commands and paths byte-exact. Run `asd-ste100` over `spec/feature.md` and `spec/strategy.md` and apply its output.

- [ ] **Step 3: Run the suites and the pressure harness once more**

```bash
bash spec/tests/run.sh 2>&1 | tail -2
bash tests/run.sh 2>&1 | tail -2
bash spec/tests/pressure/run.sh --sp both | tail -1
```
Expected: green; `pressure: 12/12 passed`.

- [ ] **Step 4: Commit**

```bash
git add spec
git commit -m "docs(spec): rewrite companion docs imperatively"
```

---

### Task 12: Phase 1 gate

**Files:**
- Modify: `.spec/plan.md` (rows F1, F25 status → `DONE`) inside `strategy.spec`
- Modify: `.spec/lessons.md` (one entry per defect found during this phase, if any)

**Interfaces:**
- Consumes: everything above.
- Produces: green baseline for Phase 2.

- [ ] **Step 1: Run every affected check and keep the output**

```bash
bash tests/run.sh 2>&1 | tail -3
node flow/engine/tests/run.mjs 2>&1 | tail -3
bash spec/scripts/validate.sh; echo "validate rc=$?"
bash spec/scripts/check-drift.sh; echo "drift rc=$?"
node .agents/skills/vibe/engine/cli.mjs render --check; echo "render rc=$?"
git ls-files '*.sh' | xargs shellcheck; echo "shellcheck rc=$?"
```
Expected: all suites green; every `rc=0`.

- [ ] **Step 2: Mark rows DONE and record lessons**

```bash
bash .agents/skills/vibe/scripts/set-state.sh strategy.brainstorm
bash .agents/skills/vibe/scripts/set-state.sh strategy.spec
```
Set F1 and F25 `Status` cells to `DONE`. Add a lesson entry (Pattern / Rule / Tags / Date: 2026-10-13) for every failed command or late-found defect in this phase. Then `bash .agents/skills/vibe/scripts/set-state.sh idle`.

- [ ] **Step 3: Commit**

```bash
git add .spec/plan.md .spec/lessons.md
git commit -m "docs(spec): close phase 1 of spec program"
```
