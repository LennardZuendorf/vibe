---
type: design
scope: strict-flow
updated: 2026-05-05
---

# Strict-flow state machine — design

## Why we are adding this

Today, skill activation and phase discipline rely on the model deciding to behave correctly. That's probabilistic. Hooks are deterministic. We want a small, KISS system that:

1. Re-anchors the agent every turn with phase-specific guidance (which skills, which forbidden tools, which next states).
2. Keeps phase transitions on rails — the agent can't jump from `feature.plan` straight into `feature.compound`.
3. Enforces tool restrictions by phase (e.g., `strategy.setup` cannot edit source code).
4. Stays cheap (per-turn inject ≤80 tokens, prompt-cache-friendly).
5. **Clamps third-party skills to OUR spec layout.** Skills like `superpowers:writing-plans` ship with their own default file paths; the inject overrides them every turn.

This is essentially what `obra/superpowers` does with skill descriptions, plus a state cursor on disk that names which skills are mandatory *right now* AND where their output goes.

## Topology

Two JSON files plus a CLAUDE.md bootstrap, plus a slash command for transitions, plus three named-but-deferred hooks.

- `.claude/state-machine.json` — static definition (states, allowed transitions, inject text, forbidden tools, exit predicate, canonical paths). Read-only at runtime.
- `.claude/state.json` — mutable cursor: `{ "flow", "phase", "feature?", "notes?" }`. Default `{ "flow": "idle", "phase": "idle" }`.
- `.claude/commands/flow.md` — slash command `/flow <phase>` that the agent uses to transition. Validates against current state's `next` array, then writes `state.json`. **Direct edits to `state.json` by the agent are prohibited** — use `/flow` only.
- `CLAUDE.md` — bootstrap teaching the system + spec-framework precedence rules.
- `.claude/hooks/` — three deferred hooks (M2):
  - `UserPromptSubmit` → emits current state's `inject` text (the daily driver).
  - `PreToolUse` (matcher `*`) → exits 2 if `tool_name` ∈ current state's `forbid_tools`. Turns "strict-flow" from rhetorical to real.
  - `Stop` (with `stop_hook_active` guard) → exits 2 if state has `exit_predicate` and it's unmet (e.g., `feature.test.exit_predicate = "tests_green_and_review_clean"`).

## Skill-name conventions

- **Caveman levels** are passed as args to a single skill: `caveman:caveman(lite)`, `caveman:caveman(full)`, `caveman:caveman(ultra)`. They are not separate skill identifiers.
- **Caveman subagents** keep their full names: `caveman:cavecrew-builder`, `caveman:cavecrew-investigator`, `caveman:cavecrew-reviewer`.
- **Caveman extras**: `caveman:caveman-commit`, `caveman:caveman-review`, `caveman:caveman-stats`, `caveman:compress`.
- **Superpowers skills** match the names exposed by the `obra/superpowers` plugin verbatim (`superpowers:brainstorming`, `superpowers:writing-plans`, `superpowers:test-driven-development`, etc.).

## Path conventions — spec framework owns layout

Skills (`superpowers:*`, `caveman:*`) carry their own default paths. **Those defaults are always overridden** in favour of our canon. Every inject names the exact destination so the model doesn't have to guess.

Canonical paths:

| What | Where |
|---|---|
| Steering | `.spec/product.md`, `.spec/tech.md`, `.spec/plan.md`, `.spec/lessons.md` |
| Cross-cutting branch docs | `.spec/{product,tech,plan}-<topic>.md` |
| Per-feature spec | `.spec/features/<feature>/{requirements,design,tasks,lessons}.md` (+ optional `research.md`) |
| Quick plan | `.spec/quick/<slug>.md` |
| Reusable agent skills | `.claude/skills/<skill>/SKILL.md` |
| State machine | `.claude/state-machine.json`, `.claude/state.json` |
| Slash commands | `.claude/commands/<name>.md` |

`<feature>` is the value of `feature` in `.claude/state.json`. `<slug>` is supplied per quick task.

The JSON also publishes a `paths` block (mirrored above) so the future M2 hooks can validate `tool_input.file_path` against a single source of truth — no hardcoded globs in the hook scripts.

### How injects clamp skills

Each inject does three things to keep skills inside the canon:

1. **Names the exact target file or directory** (`Write ONLY .spec/features/<feature>/requirements.md`).
2. **Tells the skill to override its own default** when the skill is known to ship one (e.g. `superpowers:writing-plans` defaults to its own plan dir — the inject says "override its default plan path").
3. **Forbids alternative locations implicitly** by listing only canonical paths. `forbid_tools` blocks the tool entirely when the phase shouldn't write at all (e.g. `strategy.setup` blocks `Edit`/`Write`/`Bash` until M2 hooks ship).

If a skill's output ends up off-canon at runtime, the rule is in CLAUDE.md: stop, relocate, log a lesson during the next COMPOUND.

## State machine JSON

**Canonical file:** `.claude/state-machine.json`. Read that for live inject text and transitions — embedding it inline here invites drift.

**15 states across 4 flows:**

| Flow | States | Purpose |
|---|---|---|
| _root_ | `idle` | No active flow. Entry point. |
| `setup.*` | `setup.detect`, `setup.apply` | One-time bootstrap: writes/merges CLAUDE.md, AGENTS.md, `.claude/state*.json`, `/flow` command, `.spec/lessons.md`. |
| `strategy.*` | `strategy.setup`, `strategy.manage`, `strategy.compound` | Global steering docs. No source code, ever. |
| `feature.*` | `feature.setup`, `feature.plan`, `feature.impl`, `feature.test`, `feature.compound` | Full lifecycle on `.spec/features/<feature>/`. |
| `quick.*` | `quick.triage`, `quick.do`, `quick.plan`, `quick.exec` | Small fixes; triage decides whether to plan. |

Per-state schema:

```jsonc
{
  "next": ["..."],            // legal transitions
  "skills": ["plugin:skill"], // skills to invoke this turn (caveman levels: "caveman:caveman(level)")
  "forbid_tools": ["Edit"],   // tool names blocked by future PreToolUse hook
  "exit_predicate": "name|null",  // checked by future Stop hook
  "inject": "..."             // ≤80 tokens, emitted by future UserPromptSubmit hook
}
```

Top-level `paths` block lists the canonical destinations (mirrored in CLAUDE.md). M2 hooks read it instead of hardcoding paths.

The full per-state inject text and transition graph live only in `.claude/state-machine.json`.

## The `setup.*` flow — bootstrap from nothing

`setup.*` is the meta-flow that brings a project under the strict-flow regime. It's how a fresh repo (or one without a complete `.spec/` + `.claude/` skeleton) becomes ready for `strategy.*` / `feature.*` / `quick.*`.

**Two states, deliberately small:**

- `setup.detect` — read-only scan. Walks the repo: does CLAUDE.md exist? AGENTS.md? `.spec/lessons.md`? `.claude/state-machine.json`? `.claude/commands/flow.md`? Builds a plan: which files to write fresh, which to merge, which are already canonical. Asks the user clarifying questions via `superpowers:brainstorming` when intent is ambiguous (e.g., "AGENTS.md exists with custom rules — merge or skip?"). Writes nothing. Forbid `Edit`/`Write`/`Bash`.
- `setup.apply` — execute the approved plan. Write or merge the bootstrap files **without clobbering** existing user content. Predicate `bootstrap_complete` checks the canonical files exist and CLAUDE.md references the strict-flow section. → `idle` on success, with a suggestion to run `/flow strategy.setup` next.

**Files setup.apply writes/merges (only these):**

| File | Action |
|---|---|
| `CLAUDE.md` | Append strict-flow bootstrap + paths-precedence rule if missing. Preserve all other content. |
| `AGENTS.md` | Mirror the strict-flow rules in agent-facing language. Append/merge if present. |
| `.claude/state-machine.json` | Write canonical version if missing. Diff + ask if present and divergent. |
| `.claude/state.json` | Write `{ "flow": "idle", "phase": "idle" }` if missing. Never overwrite. |
| `.claude/commands/flow.md` | Write canonical version if missing. |
| `.spec/lessons.md` | Scaffold empty file with frontmatter if missing. |

**Files setup.apply NEVER touches:**

- Source code (any non-`.md`/`.json` file outside `.claude/` and `.spec/`).
- Existing `.spec/product.md` / `.spec/tech.md` / `.spec/plan.md` (those are written by `strategy.setup`, not setup).
- Files under `.claude/hooks/` (deferred to M2).

**Bootstrap order from a fresh repo:**

```
fresh repo
  → /flow setup.detect       (audit, plan, get approval)
  → /flow setup.apply        (write CLAUDE.md, AGENTS.md, .claude/*, .spec/lessons.md)
  → /flow idle               (auto-transition on bootstrap_complete)
  → /flow strategy.setup     (now write product.md / tech.md / plan.md)
```

**Why a flow and not a one-shot script:** the agent is the right tool for "merge intelligently into existing CLAUDE.md/AGENTS.md without breaking custom rules." A bash script would clobber. The flow gates it: detect (read-only) → user approval → apply (bounded write set).

## Initial state cursor

```json
{ "flow": "idle", "phase": "idle" }
```

Canonical at `.claude/state.json`.

## Slash command

Canonical at `.claude/commands/flow.md`. Body:

```markdown
---
description: Transition the strict-flow state machine to a new phase
argument-hint: <phase>
---

You have been asked to transition to phase: $ARGUMENTS

Do exactly this, in order:

1. Read `.claude/state.json` to find the current phase.
2. Read `.claude/state-machine.json` to find that state's `next` array.
3. If `$ARGUMENTS` is NOT in `next`, refuse: print the current phase, the legal `next` values, and stop. Do not write.
4. If `$ARGUMENTS` IS in `next`, write the new cursor to `.claude/state.json`, preserving `feature` and `notes` unless the user asked to change them.
5. Print a one-line confirmation: `→ <new_phase>`.

Do NOT start doing the new phase's work in this turn. The user will prompt next; the inject hook will tell you what to do then.
```

## CLAUDE.md bootstrap

Append (do not replace) to project `CLAUDE.md`:

```
## Strict-flow state machine

You operate inside a strict flow state machine.
- Current flow + phase live in `.claude/state.json`.
- Static definition lives in `.claude/state-machine.json`.
- Each turn, a hook injects the current state's `inject` text — follow it literally. It names the exact skills to invoke and the rules for this phase.
- To transition phase, use `/flow <phase>`. NEVER edit `.claude/state.json` directly. The slash command validates the transition; direct edits will be reverted.
- Skills are invoked via the Skill tool. Caveman levels use the `caveman:caveman` skill with arg `lite`/`full`/`ultra`. Other skills use their `plugin:skill` names verbatim.
- If the inject forbids a tool, do not call it. Ask the user to `/flow` to a phase that allows it.
- Default phase is `idle`. Pick a flow when intent is clear: `setup.*` (first-time bootstrap), `strategy.*` (no source code), `feature.*` (full lifecycle on `.spec/features/<slug>/`), `quick.*` (small fixes; triage decides whether to plan).

### Spec framework precedence — skills bend to OUR layout

Our spec framework is the source of truth for paths. Skill defaults are overridden, always. See the canonical paths table in CLAUDE.md and the `paths` block in `.claude/state-machine.json`. If a skill writes off-canon: stop, relocate, log lesson next COMPOUND.
```

## Hard constraints (governing edits to this design)

- **No source code edits** in this PR. Markdown and JSON only.
- **No new third-party skills.** Inject text references real `superpowers:*` and `caveman:*` skill names. The `setup.*` flow uses only `superpowers:brainstorming` + caveman.
- **Inject text per state ≤80 tokens.** Approximated as ~60 words. Tighten before editing the JSON if any inject exceeds.
- **State count is 15.** (Bumped from 13 to add `setup.detect` + `setup.apply`.) Further additions invite drift between definition and bootstrap doc.
- **`forbid_tools` is tool names only.** No path globs (e.g., `Edit:src/**`). The future `PreToolUse` hook handles path filtering by inspecting `tool_input.file_path` against the JSON's `paths` block.
