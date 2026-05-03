---
type: entrypoint
scope: technical
children: []
updated: 2026-05-03
---

# shards-code — Technical Architecture

Project-level architecture and stack. Feature-level implementation detail lives under `.spec/features/<name>/tech.md`.

---

## Design Philosophy

1. **File-based communication.** All state lives on disk in `.spec/` and `~/.claude/`. No in-memory state survives session boundaries. Hooks read files, commands read files, scripts read files.
2. **Shell over code.** Bash + jq for everything that isn't markdown. The framework is glue, not a product. Total LOC budget: ~890 across 12 files.
3. **Hooks emit, commands act.** Hooks print to stderr to nudge Claude; they almost never block. Commands do the real work.
4. **One source of routing truth.** A single keystone script (`bin/detect-context.sh`) is the only thing that reads workflow state and decides what to load. Hooks call it. Commands call it.
5. **Subagents are disposable.** Skills delegate to subagents with focused tasks and minimal context. Subagents return compact summaries written to files, not raw transcripts in conversation.
6. **Graceful degradation.** Missing skill, corrupt `.phase`, missing `.spec/` — none of these crash a session. Warnings to stderr, fall back to neutral, let the user proceed.

---

## Architecture Overview

```
User input
   │
   ▼
[ UserPromptSubmit hook ] ──► detect-context.sh ──► routing suggestion → stderr
   │
   ▼
Claude reads CLAUDE.md (policy) + suggestion
   │
   ▼
Slash command invoked (/code:quick, /code:strategy, /code:feature)
   │
   ▼
Command markdown calls detect-context.sh ──► JSON {workflow, phase, skills, paths, warnings}
   │
   ▼
Command delegates to: CE skill / Superpowers skill / /spec skill / built-in logic
   │
   ▼
[ PreToolUse hook ] before every Edit/Write ──► detect-context.sh ──► allow / warn / block
   │
   ▼
Tool executes (or doesn't, if blocked)
   │
   ▼
[ Stop hook ] ──► detect-context.sh ──► end-of-turn integrity check → stderr
```

Three subsystems sit behind this flow:

- **Routing** — the keystone (`bin/detect-context.sh`) and state writer (`bin/set-phase.sh`). One source of truth, JSON contract, validated state file. -> [features/routing/](features/routing/tech.md)
- **Commands** — three markdown files (`commands/code-*.md`) that drive the workflow lifecycles. They consume the routing JSON and delegate to skills. -> [features/commands/](features/commands/tech.md)
- **Hooks** — four bash scripts that wire the keystone into Claude Code's lifecycle events. -> [features/hooks/](features/hooks/tech.md)

Plus two small support scripts that don't warrant their own feature: `install.sh` and `bin/merge-feature.sh`.

---

## File Layout

```
shards-code/
├── README.md                           # entry doc, install pointer
├── install.sh                          # symlinks into ~/.claude/ + project setup
├── .gitignore                          # .spec/.quick-plan.md, .spec/.phase
│
├── claude/
│   └── CLAUDE.md                       # policy doc — symlinked into projects
│
├── commands/
│   ├── code-quick.md                   # /code:quick <task>
│   ├── code-strategy.md                # /code:strategy
│   ├── code-feature.md                 # /code:feature <name>
│   └── code-amend.md                   # v1.1
│
├── bin/
│   ├── detect-context.sh               # KEYSTONE — routing logic
│   ├── set-phase.sh                    # only sanctioned way to write .phase
│   └── merge-feature.sh                # feature → global merge for IMPL:COMPOUND
│
├── hooks/
│   ├── session-start.sh                # show phase + skills + lessons
│   ├── user-prompt-submit.sh           # gentle command suggestions
│   ├── pre-tool-use.sh                 # phase gate (warnings + 2 hard blocks)
│   └── stop.sh                         # end-of-turn integrity
│
├── settings.json                       # registers hooks + permissions
│
└── .agents/skills/spec/                # bundled /spec skill (already present)
```

Per-project addition (created lazily by commands):
```
<project>/.spec/
├── .phase                              # SINGLE LINE: <workflow>:<phase>[:<feature>]
├── .quick-plan.md                      # ephemeral, gitignored
├── product.md, tech.md, lessons.md     # global specs
├── plan.md                             # optional global roadmap
├── product-design-language.md          # optional design system doc
├── features/<name>/                    # ephemeral per-feature specs
└── archive/<name>/                     # post-COMPOUND, kept for history
```

---

## Tech Stack

**Inherited:** Claude Code CLI (skills, hooks, slash commands, subagents, Agent tool), bash 4+, jq, git. macOS and Linux.

**Added:** none. No new runtime dependencies.

**Why this stack:** matches the bundled `/spec` skill, which is bash + markdown. No build step. Portable across machines via symlinks. Forkable.

---

## State File Format

Single line, three colon-separated segments, third optional. Lives at `.spec/.phase`.

```
                                        # empty / neutral
quick                                   # quick task in progress
strategy:DESIGN:RESEARCH                # bootstrapping global specs
strategy:DESIGN:DISCUSS
strategy:DESIGN:SPEC
strategy:DESIGN:PLAN
feature:DESIGN:RESEARCH:dark-mode
feature:DESIGN:DISCUSS:dark-mode
feature:DESIGN:SPEC:dark-mode
feature:DESIGN:PLAN:dark-mode
feature:IMPL:VERIFY:dark-mode
feature:IMPL:WORK:dark-mode
feature:IMPL:REVIEW:dark-mode
feature:IMPL:SHIP:dark-mode
feature:IMPL:COMPOUND:dark-mode
```

**Validation rules** (enforced by `bin/set-phase.sh`):
- `workflow` ∈ {`quick`, `strategy`, `feature`} or empty
- `phase` ∈ {`DESIGN:RESEARCH`, `DESIGN:DISCUSS`, `DESIGN:SPEC`, `DESIGN:PLAN`, `IMPL:VERIFY`, `IMPL:WORK`, `IMPL:REVIEW`, `IMPL:SHIP`, `IMPL:COMPOUND`} or empty
- `feature` required iff `workflow=feature`; forbidden otherwise
- Strategy uses only `DESIGN:*`. Feature uses both `DESIGN:*` and `IMPL:*`. Quick stores no phase.

**Direct edits are hard-blocked** by PreToolUse. Only `bin/set-phase.sh` may write it. Detail in [features/routing/tech.md](features/routing/tech.md).

---

## Basic Implementation

### `install.sh`

Single-shot setup script. Idempotent. Reads its own location, then:

1. Symlinks `bin/`, `hooks/`, `commands/` from this repo into `~/.claude/shards-code/`.
2. Writes `<target-project>/.claude/settings.json` registering the four hooks (offers a unified diff if a settings file already exists; never auto-overwrites).
3. Adds `.spec/.phase` and `.spec/.quick-plan.md` to the target project's `.gitignore`.
4. Prints next-step suggestion (`/code:strategy` if no `.spec/`, `/code:quick` otherwise).

~80 LOC. No build step. No package manager.

### `bin/merge-feature.sh`

Called only during `feature:IMPL:COMPOUND`. Reads `.spec/features/<name>/tech.md`, identifies sections marked as cross-cutting (frontmatter `merge: true` or explicit `<!-- merge -->` markers), proposes a unified diff against global `.spec/tech.md`, asks the user to confirm. On approval: applies the diff, then `mv .spec/features/<name>/ .spec/archive/<name>/`. Conflicts are flagged for the user, never auto-resolved.

~60 LOC. Detail in [features/commands/tech.md](features/commands/tech.md) (it's part of `/code:feature`'s COMPOUND phase).

### `claude/CLAUDE.md`

Policy doc. Decision tree for picking a command, skill-loading rules ("the hooks tell me which skills to load — I check `detect-context.sh` output"), spec discipline (product.md = what & why, tech.md = how, lessons.md = read at session start, written only during COMPOUND), and the "what I never do" list (skip phases, edit `.phase` directly, write to lessons.md outside COMPOUND, inline review when `ce-code-review` is the right tool).

`install.sh` symlinks it into per-project `<project>/CLAUDE.md`.

### `settings.json`

Registers the four hooks against their events with the right matchers (`SessionStart` → `startup`, `PreToolUse` → `Edit|Write|NotebookEdit`, `UserPromptSubmit` and `Stop` → all). Defines a permissions allowlist for `Read`, `Glob`, `Grep`, `TodoWrite`, the three skills, and the bash invocations of the keystone scripts. Detail in [features/hooks/tech.md](features/hooks/tech.md).

---

## Build Sequence

| Order | Component | Feature | LOC est | Phase |
|---|---|---|---|---|
| 1 | `bin/detect-context.sh` | routing | ~150 | M1 |
| 2 | `bin/set-phase.sh` | routing | ~30 | M1 |
| 3 | `commands/code-quick.md` | commands | ~50 | M1 |
| 4 | `hooks/session-start.sh` | hooks | ~40 | M1 |
| 5 | `hooks/pre-tool-use.sh` | hooks | ~100 | M2 |
| 6 | `commands/code-strategy.md` | commands | ~80 | M2 |
| 7 | `commands/code-feature.md` | commands | ~150 | M3 |
| 8 | `bin/merge-feature.sh` | commands | ~60 | M3 |
| 9 | `hooks/user-prompt-submit.sh` | hooks | ~50 | M3 |
| 10 | `hooks/stop.sh` | hooks | ~40 | M3 |
| 11 | `commands/code-amend.md` | commands | ~60 | v1.1 |
| 12 | `install.sh` | (basic) | ~80 | M3 |

The keystone (`detect-context.sh`) ships first because everything else depends on its output contract.

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Routing layer becomes a kitchen sink | Hard contract: inputs (one positional + optional stdin), outputs (one JSON object). No mutations. |
| Hook latency on every prompt | All four hooks delegate to `detect-context.sh`. Target ≤ 50ms per call. |
| Skill missing at runtime breaks command | Routing returns warnings + drops missing skills from `skills_to_load`. Commands check the JSON before delegating. |
| `.phase` corruption | `set-phase.sh` validates before writing. Routing returns `workflow=none` + warning on parse failure. |
| Two hard blocks turn out to be wrong | Easy to remove from `pre-tool-use.sh`. Adding more is harder politically — start narrow. |
| `~/.claude` already has hooks from another framework | `install.sh` writes shards-code's settings.json next to existing files; user merges manually. |

---

## Features

Detailed architecture and contracts live in feature folders:

| Feature | Covers |
|---|---|
| **[features/commands/](features/commands/tech.md)** | Command file structure, sub-phase implementation, skill delegation tables, state transitions, `merge-feature.sh`. |
| **[features/routing/](features/routing/tech.md)** | `bin/detect-context.sh` JSON contract and routing rules; `bin/set-phase.sh` validation grammar; performance contract. |
| **[features/hooks/](features/hooks/tech.md)** | Per-hook trigger/matcher/exit-code spec, the two hard blocks justification, settings.json registration, performance contract. |
