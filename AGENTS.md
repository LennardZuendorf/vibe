# AGENTS.md — vibe Engineering Guide

> This repository builds vibe itself. The sections below are this repo's own
> dogfood guide (user-owned prose). The generic vibe-flow contract that ships to
> every install target lives in the managed `vibe:instructions` block further
> down — it is byte-identical to `flow/reference/templates/AGENTS.md` and is
> replaced on every `setup.apply` / install, so edit the template, not the block.

**Repository:** vibe — self-hosting bash/Markdown/JSON workflow harness (in active build).
**Canonical:** this file. `CLAUDE.md` symlinks here.

## Prime Directive

**Spec first. Always.**

1. **ASK** — clarify requirements; no assumptions.
2. **PLAN** — break down, read `.spec/`, present approach.
3. **CONFIRM** — approval happens at the two human gates (plan→impl, verify→ship), not before every action.
4. **EXECUTE** — implement step-by-step; verify before claiming done.

- MUST read `.spec/` before writing code.
- MUST NOT write code without an approved plan or spec.
- MUST NOT invent filenames or state files the repo does not have (see **Dogfood status**).

The **vibe flow** (state machine, hooks, per-turn routing) is what this repo is building.
It is **not** a prerequisite for doing work here. Follow the **Working model** in the managed
block below unless your task is explicitly to implement flow machinery.

## Dogfood status

Know what exists before you read or create files.

| Path | Status |
|---|---|
| `.spec/` | **Live** — primary source of truth; read every session |
| `.agents/skills/spec/` | **Live** — spec skill + `validate.sh` |
| `AGENTS.md` / `CLAUDE.md` | **Live** — this guide; `CLAUDE.md` → `AGENTS.md` |
| `.agents/skills/vibe/state-machine.json` | **Present** — static flow definition (data to edit when building flow) |
| `.agents/skills/vibe/scripts/` | **Present** — `set-state.sh`, `detect-context.sh`, etc. |
| `.agents/skills/vibe/state.example.json` | **Present** — template for the cursor file |
| `.agents/skills/vibe/state.json` | **Often absent** — gitignored runtime cursor; missing is normal |
| `.agents/skills/vibe/evidence/` (`flow/evidence/`) | **Often absent** — gitignored `*.verify` receipts (`feature-<feature>.md`, `quick.md`); written during verify, missing is normal |
| `.agents/skills/vibe/` | **Present** — workflow skill (SKILL.md router + phase files) |
| `.claude/commands/`, `.claude/hooks/`, `.claude/settings.json` | **Present** — Claude Code adapter (`/flow` command + three hook scripts wired via `settings.json`) |
| **`flow.json`** | **Does not exist** — never expect, read, or create this file |
| Per-turn inject / D12 orders-in-skills | **Live** — orders sourced from linked `vibe` skill via `orders.sh` |

When in doubt, read [.spec/plan.md](.spec/plan.md) for spec-vs-repo gaps.

## Task routing

Route by **intent**, not by flow cursor.

| Intent | Load first | Write surface |
|---|---|---|
| Understand the project | `.spec/product.md`, `.spec/tech.md` | — |
| Build a named feature | `.spec/features/<name>/` + root `plan.md` | per feature Scope |
| Small bounded fix | relevant feature spec or `.spec/quick/<slug>.md` | minimal |
| Spec / plan work | `.agents/skills/spec/SKILL.md` | `.spec/**` per write rules |
| Build flow machinery | `.spec/features/vibe-flow/` | `.agents/skills/vibe/`, `vibe` skill |
| Build adapters / hooks | `.spec/features/platform-adapters/` | `.claude/`, `install.sh` |
| Build AGENTS.md provisioning | `.spec/features/agent-instructions/` | templates, merge scripts |
| Set up or repair harness | `.agents/skills/vibe/SKILL.md` | `.agents/**`, managed blocks |

`vibe` skill phases are **helpers** for their domains. Read the matching feature spec first;
the skill does not override `.spec/`.

## Target harness

This is the **end-state** the repo is building. Reference when implementing
[vibe-flow](.spec/features/vibe-flow/product.md) or [platform-adapters](.spec/features/platform-adapters/product.md) — not for ordinary feature work.

- **Cursor:** `.agents/skills/vibe/state.json` — `{flow, phase, feature, updated}`; create from
  `state.example.json` only when testing transitions.
- **Machine:** `.agents/skills/vibe/state-machine.json` — static states, skills, legal `next`.
- **Transitions:** only via `bash .agents/skills/vibe/scripts/set-state.sh <flow.phase> [feature]`;
  never edit `state.json` by hand.
- **Routing:** per-turn orders live in the `vibe` skill (D12, `## Orders` in `SKILL.md`); the machine
  holds the `skill` link, not prose. `orders.sh` resolves the current state's orders.
- **Adapters:** hooks read `.agents/skills/vibe`, not `.claude/state.json`.

`.claude/` is a runtime adapter — not canonical.

## Repo layout

```text
.spec/                 # durable memory (product/tech/design/plan/lessons + features/)
.agents/skills/spec/   # bundled spec framework
.agents/skills/vibe/   # workflow skill: router, phase files, state machine, scripts
AGENTS.md              # this file (canonical)
CLAUDE.md              # symlink → AGENTS.md
```

## Conventions

- Bash MUST use `set -euo pipefail`; MUST be shellcheck-clean.
- Scripts MUST be deterministic, idempotent, graceful-degrade (warn, never hard-fail).
- State machine is **data** — edit `state-machine.json`, not prose duplicates.
- Paths and commands stay byte-exact.
- Prefer editing existing files. Do not create files without necessity.
- Do not add comments that narrate the obvious.

## Boundaries

**Always**
- Read `.spec/` before code.
- Run `validate.sh` before claiming work is done.
- Cite plan unit IDs in tests and commits during impl.
- Respect feature **Scope** tables — do not implement another feature's units.

**Ask first**
- Root spec edits outside strategy / compound / setup.
- Scope escalation (small fix → named feature).
- Gated edges (plan→impl, verify→ship) and quick→feature escalation; other transitions auto-advance.
- Adding dependencies or deleting files.

**Never**
- Expect or create `flow.json`.
- Treat missing `state.json` as an error for normal work.
- Edit `state.json` by hand.
- Edit inside `<!-- vibe:active-rules:* -->` markers.
- Clobber user-owned content outside managed blocks.
- Treat `.claude/` as canonical.

## Commits

Conventional Commits. Imperative, lowercase, ≤50-char subject, no trailing period.

```text
feat(flow): add inject hook wiring
fix(spec): correct feature frontmatter check
docs(agents): rewrite instruction set
```

## Spec layout

Root: `.spec/{product,tech,design,plan,lessons}.md`.
Features: `.spec/features/<feature>/{product,tech}.md` required; `design.md`,
`plan.md`, `research.md` optional.

<!-- vibe:instructions:start -->
<!-- Managed by vibe (merge-agents.sh). Edits inside these markers are
     replaced on the next setup.apply / install. Content outside the markers
     (above this line, or inside vibe:active-rules) is user-owned. -->

This project runs the **vibe flow** — a spec-first workflow harness. `.spec/` is the
durable memory; sessions are ephemeral. Read the specs before you write code.

## Session start

1. Read `.spec/lessons.md` and `.spec/plan.md`.
2. Identify the feature you are working on and load its specs:
   `.spec/features/<name>/{product,tech,plan}.md`.
3. If `.agents/skills/vibe/state.json` exists and you are continuing a flow session,
   read `{flow, phase, feature}` and resume the linked `vibe` skill. A missing cursor
   means `idle` — proceed with the specs; a missing cursor is not an error.

## Driving the flow

- `/flow <flow.phase> [feature] [confirm]` transitions the state machine. Pass the feature
  name to carry it into the cursor; add the literal `confirm` token to cross a gated edge
  (the two human gates below). Never edit `state.json` by hand.
- Under the hood `/flow` runs
  `bash .agents/skills/vibe/scripts/set-state.sh <flow.phase> [feature]`. That script is a
  *writer*, not a gate — it moves the cursor but enforces no approval. The gate teeth live
  in the `/flow` command and the hooks, not the writer.
- Per-turn **orders** for the current state come from the linked `vibe` skill, resolved by
  `orders.sh`. Follow the current state's orders over any delegated skill's own prose.

## Working model

```
ASK → read .spec/ → PLAN → CONFIRM → IMPL → verify → compound
```

Most transitions auto-advance. Two edges are **human gates** — stop and get explicit
approval before crossing: plan → impl, and verify → ship. Everything else flows without
pausing to ask.

## Write invariants

Policy lives in `detect-context.sh decide` (defaults to `idle` when `state.json` is absent):

1. `.spec/lessons.md` — writable only in `feature.compound`, `setup.apply`, `strategy.spec`,
   or `quick.verify` (the flow-end states that carry the lesson step; or when explicitly
   recording a lesson with user approval).
2. Root `.spec/{product,tech,design,plan}.md` — only in `strategy.spec`, `feature.compound`,
   or `setup.apply`.
3. `.agents/skills/vibe/state.json` — only via `set-state.sh`.

Everything else is allow/warn. Check before writing:

```bash
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>
```

## Precedence

The cursor owns sequencing and artifact destinations; delegated skills own method. When a
delegated skill's text names its own artifact path or hands off elsewhere, the current
state's orders win: write to the state's surface, transition only via `set-state.sh`. Scope
edits are not a state — edit within the current state's write surface and stay put.
`set-state.sh idle` is always legal: abort ends any flow.

## Commands

```bash
# Spec validation — run before claiming done
bash .agents/skills/spec/scripts/validate.sh
# Write policy for a path (works without state.json)
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>
# Health-check the harness wiring (hooks, cursor, machine)
bash .agents/skills/vibe/scripts/doctor.sh
```

## Enforcement is partial — do not trust it blindly

- The `pre-tool-use-guard` hook only intercepts `Edit`, `Write`, and `NotebookEdit` tool
  calls. Shell redirection (`>`, `tee`, `sed -i`) bypasses it — honoring the write
  invariants there is a **convention**, backed only by a warn-only Bash sniffer.
- `set-state.sh` writes the cursor; it does not gate. Approvals are enforced by `/flow` and
  the hooks, not by the writer.
- The Stop receipt tooth (a `*.verify` state needs a fresh evidence receipt) fires only when
  the hooks are wired into the platform (e.g. `.claude/settings.json`). No wiring, no tooth.

## Degrade

Scripts graceful-degrade: `jq` is recommended but optional (a pure-bash path keeps the
cursor byte-identical), and other agents may edit concurrently — touch only your own files
and prefer warn over hard-fail.
<!-- vibe:instructions:end -->

<!-- vibe:active-rules:start -->
<!-- Generated from .spec/lessons.md by regen-active-rules.sh. Do not edit by hand;
     edit lessons.md and re-run during compound. Top 5, pinned first. -->

### Active Rules

- **Compound is where drift is born — enforce it mechanically** — Compound must be mechanically enforced, not trusted to discipline. A drift check (`spec/scripts/check-drift.sh`, CI-wired after `validate.sh`) fails when a directory under `.spec/features/` has no row in the root `.spec/plan.md`, and flags any `NOT STARTED` unit left in a feature `plan.md`. Hand-written assertion counts are errored the same way — they rot silently. A green suite is not a compounded feature; make the missing-compound state impossible to merge past.
- **Script self-location: search for markers, don't count hops** — Scripts reachable through compat symlinks must locate the repo root by upward marker search (`.spec`/`.git`), never fixed hop counts; pin with path-parity tests asserting byte-identical output via both real and symlinked invocation.
- **The dogfood repo is a privileged target — eval on a fresh, non-git install** — A tool that will be *installed elsewhere* must be tested from a representative fresh target (a bare `mktemp -d`, no `.git`, no `.spec`), not just the source/dogfood repo. Prefer self-location relative to the script's own path over repo-root markers the target may lack. Run a periodic "stranger" eval (fresh agent, docs-only, throwaway sandbox) as a release gate — it exercises the install-target reality the in-repo suites cannot.
- **Uninstall must surgically invert the install into shared dirs, and the test must discriminate** — An uninstaller must delete only the paths the installer created (per-file inverse of the copy), never blanket-remove a shared directory; pruning *emptied* dirs is fine. Pair every preservation guarantee with a **discriminating** test — one that fails if the safety code is replaced by the naïve destructive version (drop a user file into each shared dir, run uninstall, assert it survives *and* the shipped file is gone). Reuse the tested marker-pairing guard for the managed instruction block; never re-implement it.
- **Skill design: promote superpowers proactively, remain self-sufficient** — Skills SHOULD proactively offer their optimal executor at each step — "I can use X for this, want me to?" — and MUST self-execute from their constraint documents if the user declines or the executor is unavailable. The order is always: offer first, self-suffice second. Never silently skip the offer; never block on the answer.
<!-- vibe:active-rules:end -->
