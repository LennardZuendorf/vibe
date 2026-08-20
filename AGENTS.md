# AGENTS.md — vibe kit Engineering Guide

> This repository builds vibe kit itself. The sections below are this repo's own
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
| Build flow machinery | `.spec/tech.md` + `.agents/skills/vibe/` | `.agents/skills/vibe/`, `vibe` skill |
| Build adapters / hooks | `.spec/tech.md` + `.claude/` | `.claude/`, `install.sh` |
| Build AGENTS.md provisioning | `flow/reference/templates/AGENTS.md` | templates, merge scripts |
| Set up or repair harness | `.agents/skills/vibe/SKILL.md` | `.agents/**`, managed blocks |

`vibe` skill phases are **helpers** for their domains. Read the matching feature spec first;
the skill does not override `.spec/`.

## Target harness

This is the **end-state** the repo is building. Reference the root
[product.md](.spec/product.md) and [tech.md](.spec/tech.md) — not for ordinary feature work.

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

**Output: caveman style** — terse and high-signal, no filler or hedging; compress
receipts and subagent summaries. Never compress security warnings, irreversible-action
confirmations, or code/paths/commands — those stay full and byte-exact.

## Write policy

Which paths this harness restricts, and in which states, is **data**:
`content/policy.json`, read by `detect-context.sh decide` (which defaults to `idle`
when `state.json` is absent). The rules are rendered rule-by-rule into the
`vibe:rules` block below — this section states no state list of its own, because a
second hand-written copy is a copy that can disagree with the enforcer. Check any
path before writing it:

```bash
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>
```

## Precedence

The cursor owns sequencing and artifact destinations; delegated skills own method. When a
delegated skill's text names its own artifact path or hands off elsewhere, the current
state's orders win: write to the state's surface, transition only via `set-state.sh`. Scope
edits are not a state — edit within the current state's write surface and stay put.
`set-state.sh idle` is always legal: abort ends any flow.

## Injected rules

Standing rules are authored as content blocks and composed into channels: the
per-turn prompt, session start, and the `vibe:rules` block below. Configure them in
this repo's root `vibe.json` — add, remove, reorder, disable, or define your own —
never by editing the block, which is regenerated.

## Commands

```bash
# Spec validation — run before claiming done
bash .agents/skills/spec/scripts/validate.sh
# Write policy for a path (works without state.json)
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>
# Health-check the harness wiring (hooks, cursor, machine)
bash .agents/skills/vibe/scripts/doctor.sh
# Injection config: what each channel composes, and lint it
node .agents/skills/vibe/engine/cli.mjs render --list
node .agents/skills/vibe/engine/cli.mjs render --check
# Re-render the AGENTS.md rules block after editing vibe.json
node .agents/skills/vibe/engine/cli.mjs render agents-md --write
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

- **A check that examines nothing must fail loudly, never pass quietly** — Every guard must assert its own population — a floor on what it examined, and that the things it examined exist. Absence of findings is only evidence when presence of *input* is proven. Extend this to your own verification: after planting a mutant, confirm it actually landed (`git diff --numstat`) before believing the result, and prefer structural floors over hand-written counts, which rot. A green that cannot distinguish "checked and clean" from "checked nothing" is not a green.
- **A comment asserting a safety property becomes load-bearing — test it or delete it** — Treat a comment that asserts a safety or scope property as an untested claim, and either give it a test or delete it. When a fix proves a comment wrong, correct the comment in the same commit — a stale comment is worse than none, because the next reader (and the next reviewer's reasoning) will rely on it. Prefer assertions that are structural over lists of names: scrub every `tests/` directory at any depth and assert *no test artifact anywhere* reaches a target, rather than naming two directories that the next co-located suite will silently escape.
- **A control case that depends on an unset variable must delete it, not merely not set it** — A test whose meaning is "variable X is not set" must **delete** X from the child environment, not rely on the parent not having it — and the deletion belongs on the shared spawn helper so every call site can use it. Then run the suite under each ambient variable it reads, in both states, as its own leg. Related: run every CI leg locally, byte-exact, before pushing — the round that shipped this bug ran none of them, and the round that fixed it ran all ten.
- **A guard is only as strong as the capability it bans, not the spelling it matches** — When a guard is evaded, do not add a pattern for the evasion — that buys exactly one round. Close the *capability*: state the invariant as "no module may obtain X by any means unless allowlisted", then find the mechanical property that makes it true regardless of syntax. Match against the whole comment-stripped file, never per line. Close the scanned set under whatever relation the attacker can traverse (here, module resolution — which covers static `import`, `import()`, `require`, and `createRequire` without naming any of them). Pin exemptions by exact line *and occurrence count*, never by file. And require the implementer to produce the list of evasions it tried against its **own** fix, including the ones that failed — the round that finally held was the first to produce that artifact.
- **Porting a script destroys the oracle that proves the port — freeze it first** — When replacing an implementation, copy its predecessor into the test tree as a frozen oracle **before** the first line of the replacement is written, and make differential comparison part of the suite. Prioritise by blast radius: anything that can block, delete, or halt gets its oracle frozen first. Where an oracle disagrees with *itself* across its own code paths, follow the fail-safe branch and pin the divergence with a control — losing one turn of enforcement is recoverable, wedging a session is not.
<!-- vibe:active-rules:end -->

<!-- vibe:rules -->
_Managed by vibe — rendered from the `agents-md` channel by `vibe render agents-md --write`. Edit the blocks or `vibe.json`, not this region._

## Write invariants

Every write this harness restricts is one rule in `content/policy.json`. The
list below is RENDERED from that file — the same data
`detect-context.sh decide` enforces — so the prose and the enforcer cannot
disagree. Edit the rules, never this text.

- `.agents/skills/vibe/state.json` — otherwise: block — state.json is written only via set-state.sh, never by direct edit
- `.spec/lessons.md` — `feature.compound`, `setup.apply`, `strategy.spec`, `quick.verify`: allow; otherwise: block — .spec/lessons.md is writable only during feature.compound, setup.apply, strategy.spec, or quick.verify (current: the current state)
- `.spec/product.md`, `.spec/tech.md`, `.spec/design.md`, `.spec/plan.md` — `strategy.spec`, `feature.compound`, `setup.apply`: allow; otherwise: block — root .spec specs are writable only during strategy.spec, feature.compound, or setup.apply (current: the current state)
- `.spec/features/*` — `feature.impl`, `quick.fix`: warn — .spec/features edits are frozen during impl/fix — route back to feature.design/plan to change scope (current: the current state); otherwise: allow
- `CLAUDE.md`, `AGENTS.md` — otherwise: warn — CLAUDE.md/AGENTS.md active-rules block is generated by regen-active-rules.sh; edits inside the markers are overwritten next compound
- `src/*`, `tests/*` — `feature.verify`: warn — verify writes no src — route findings back to impl (set-state.sh feature.impl); `quick.verify`: warn — verify writes no src — route findings back to fix (set-state.sh quick.fix); `feature.impl`, `quick.fix`, `setup.apply`: allow; otherwise: warn — source/test edits outside an impl/fix state (current: the current state)

Check a path before writing it:

```bash
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>
```

## Delegating to sub-agents

Model tiers for ANY delegated work — Agent-tool calls and Workflow-script `agent()`
calls alike. Set the `model` parameter explicitly on every call; never omit it
(omission silently inherits the session model):

- `haiku` — mechanical bulk work: renames, boilerplate, format conversion, log triage.
- `sonnet` — default for well-specified implementation with clear acceptance criteria.
- `opus` — genuinely tricky work: concurrency, subtle algorithms, adversarial
  verify/judge panels, gnarly debugging.
- `fable` — rare; only when independence from your context is the point (e.g.
  adversarial review of your own plan or a large diff). If the complexity of the task
  warrants a Fable sub-agent, ALWAYS check with me first — never spawn one unprompted.

When unsure between tiers, pick the cheaper and escalate on failure.

## Dynamic workflows (Workflow tool)

Applies to ALL sessions, any model. Dynamic workflows do not need to be avoided —
reach for the Workflow tool when a task has 3+ independent parallelizable subtasks or
would benefit from a pipeline/judge panel.

Standing rule on opt-in: if ultracode is NOT on for the session (no "ultracode"
keyword, no toggle, no orchestration request in my own words), plan first — propose
the workflow in one or two sentences with the rough shape and cost, and wait for my
reply; my "yes" is the opt-in. If ultracode IS on, invoke directly.

**Agent models inside workflow scripts:** every `agent()` call MUST set the `model`
parameter explicitly, chosen per "Delegating to sub-agents" above — with one
tightening: NEVER use `fable` agents in a dynamic workflow, not even with approval.
Only `haiku`, `sonnet`, or `opus`. If a Fable review is warranted, it happens AFTER
the workflow completes, as a standalone Agent-tool call (ask first, per above) —
never as a workflow stage.
<!-- /vibe:rules -->
