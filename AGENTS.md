# AGENTS.md — vibe Engineering Guide

<!-- vibe:instructions:start -->
<!-- Managed by vibe-setup (merge-agents.sh). Edits inside these markers are
     replaced on the next setup.apply / install. Content outside the markers
     (above this line, or inside vibe:active-rules) is user-owned. -->

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
It is **not** a prerequisite for doing work here. Follow the **Working model** below unless
your task is explicitly to implement flow machinery.

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

## Session start

Sessions are ephemeral; `.spec/` is the memory.

1. Read [.spec/lessons.md](.spec/lessons.md) and [.spec/plan.md](.spec/plan.md).
2. Identify the **feature** you are building (table in `plan.md`) and load its specs:
   `.spec/features/<name>/{product,tech,plan}.md`.
3. Route by intent (table below). Do **not** block on flow state.
4. **Optional** — if `.agents/skills/vibe/state.json` exists and you are explicitly continuing
   a flow session, read `{flow, phase, feature}` and resume the linked `vibe` skill.
   If the file is missing, treat as `idle` and proceed with specs.

## Working model (default)

Use this for all implementation work until the flow harness is fully wired.

```
ASK → read .spec/ → PLAN (cite unit IDs) → CONFIRM → IMPL → verify → compound
```

| Step | Do |
|---|---|
| Scope | Load feature `product.md` **Scope** table — respect owns / does-not-own |
| Plan | Follow unit IDs in feature `plan.md` (`SF*`, `VF*`, `AI*`, `U*`, …) |
| Impl | Test-first where the plan says so; cite unit ID in commits |
| Done | Run `bash .agents/skills/spec/scripts/validate.sh`; show evidence |

**Human gates:** approve plan units before large impl; approve verify evidence before "done".

### Precedence

The cursor owns sequencing and artifact destinations; delegates own method.
When a delegated skill's text names its own artifact path, commits its own
output, or hands off to another skill, the current state's orders win: write
to the state's surface, leave commits to the flow, transition only via
`set-state.sh`. `set-state.sh idle` is always legal — abort ends any flow.

Root [.spec/plan.md](.spec/plan.md) owns milestones and cross-feature order.
Feature `plan.md` owns unit tables — do not duplicate units in chat or commits prose.

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

## Write invariants

Policy lives in `detect-context.sh decide` (defaults to `idle` when `state.json` is absent):

1. `.spec/lessons.md` — writable only in `*.compound` or `setup.apply` (or when explicitly recording lessons with user approval).
2. Root `.spec/{product,tech,design,plan}.md` — only in `strategy.spec`, `feature.compound`, or `setup.apply`.
3. `.agents/skills/vibe/state.json` — only via `set-state.sh`.

Everything else is allow/warn. Check before writing:

```bash
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>
```

## Repo layout

```text
.spec/                 # durable memory (product/tech/design/plan/lessons + features/)
.agents/skills/spec/   # bundled spec framework
.agents/skills/vibe/   # workflow skill: router, phase files, state machine, scripts
AGENTS.md              # this file (canonical)
CLAUDE.md              # symlink → AGENTS.md
```

## Commands

```bash
# Spec validation — run before claiming done
bash .agents/skills/spec/scripts/validate.sh

# Write policy (works without state.json)
bash .agents/skills/vibe/scripts/detect-context.sh
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>

# Flow tooling — only when explicitly testing or building flow
cp .agents/skills/vibe/state.example.json .agents/skills/vibe/state.json   # seed cursor
bash .agents/skills/vibe/scripts/set-state.sh <flow.phase> [feature]
bash .agents/skills/vibe/scripts/validate-state.sh
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
