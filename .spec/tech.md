---
type: entrypoint
scope: technical
children:
  - tech-detect-context.md
  - tech-hooks.md
updated: 2026-05-03
---

# shards-code — Technical Architecture

## Design Philosophy

1. **File-based communication.** All state lives on disk in `.spec/` and `~/.claude/`. No in-memory state survives session boundaries. Hooks read files, commands read files, scripts read files.
2. **Shell over code.** Bash + jq for everything that isn't markdown. The framework is glue, not a product. Total LOC budget: ~890 across 12 files.
3. **Hooks emit, commands act.** Hooks print to stderr to nudge Claude; they almost never block. Commands do the real work. The two structural blocks exist because they protect cross-cutting invariants (lessons + global specs only mutate during COMPOUND).
4. **One source of routing truth.** `bin/detect-context.sh` is the keystone. Hooks call it, commands call it, anything else that needs to know the current state calls it. No parallel routing logic.
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

```
shards-code/
├── README.md                           # entry doc, install pointer
├── install.sh                          # symlinks into ~/.claude/ + project setup
├── .gitignore                          # .spec/.quick-plan.md, .spec/.phase
│
├── claude/
│   └── CLAUDE.md                       # the policy doc — symlinked into projects
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
└── .agents/
    └── skills/
        └── spec/                       # bundled /spec skill (already present)
            ├── SKILL.md
            ├── scripts/{setup,validate,list-specs}.sh
            └── reference/templates/
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
│   ├── product.md, tech.md, plan.md
│   └── design.md (if UI)
└── archive/<name>/                     # post-COMPOUND, kept for history
```

---

## Tech Stack

**Inherited:** Claude Code CLI (skills, hooks, slash commands, subagents, Agent tool), bash 4+, jq, git. macOS and Linux.

**Added:** none. No new runtime dependencies. Total ~890 LOC across 12 files.

**Why this stack:** matches the bundled `/spec` skill, which is bash + markdown. No build step. Portable across machines via symlinks. Forkable.

---

## What We Build vs Inherit

| Source | Approx. Lines | What |
|---|---|---|
| **Claude Code** (inherited) | — | Hook events, slash commands, skill loading, subagent dispatch |
| **Compound Engineering** (inherited) | — | All `/ce-*` commands, ~51 agents, ~36 skills |
| **Superpowers** (inherited) | — | brainstorming, TDD, subagent-driven dev, systematic debugging |
| **`/spec` skill** (bundled, existing) | — | Spec writing, validation, templates |
| **shards-code core** (this project) | ~890 | 3 commands, 4 hooks, 3 bin scripts, install.sh, CLAUDE.md, settings.json |

---

## Key Patterns

- **The keystone routing script.** `bin/detect-context.sh` reads `.spec/.phase` plus project state and emits JSON describing what to do. Single source of truth for workflow state. -> [tech-detect-context.md](tech-detect-context.md)
- **Four-hook gentle enforcement.** SessionStart, UserPromptSubmit, PreToolUse, Stop. Three are stderr-only; PreToolUse has exactly two structural blocks plus the `.phase` write block. -> [tech-hooks.md](tech-hooks.md)
- **State as a single line.** `.spec/.phase` holds `<workflow>:<phase>[:<feature>]`. Written only by `bin/set-phase.sh`. PreToolUse blocks direct edits.
- **Feature spec lifecycle.** Created during `feature:DESIGN:SPEC`, read-only during `feature:IMPL:*`, merged + archived during `feature:IMPL:COMPOUND` via `bin/merge-feature.sh`. Global specs only absorb cross-cutting decisions.
- **Graceful skill fallback.** `detect-context.sh` checks each `<plugin>:<skill>` against the installed skill list (via simple existence check on known paths). Missing → drop from `skills_to_load`, append warning. Workflows degrade, never fail.

---

## State File: `.spec/.phase`

Single line, three colon-separated segments, third optional:

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
- `phase` ∈ {`DESIGN:RESEARCH`, `DESIGN:DISCUSS`, `DESIGN:SPEC`, `DESIGN:PLAN`, `IMPL:VERIFY`, `IMPL:WORK`, `IMPL:REVIEW`, `IMPL:SHIP`, `IMPL:COMPOUND`} or empty (for `quick` and neutral)
- `feature` required iff `workflow=feature`; forbidden otherwise
- Strategy uses only `DESIGN:*` phases. Feature uses both `DESIGN:*` and `IMPL:*`. Quick stores no phase.

**Direct edits are hard-blocked** by PreToolUse. The path `.spec/.phase` is structurally privileged: only `bin/set-phase.sh` may write it.

---

## Build Sequence

| Order | Component | LOC est | Phase |
|---|---|---|---|
| 1 | `bin/detect-context.sh` | ~150 | M1 Foundation |
| 2 | `bin/set-phase.sh` | ~30 | M1 |
| 3 | `commands/code-quick.md` | ~50 | M1 |
| 4 | `hooks/session-start.sh` | ~40 | M1 |
| 5 | `hooks/pre-tool-use.sh` | ~100 | M2 Enforcement |
| 6 | `commands/code-strategy.md` | ~80 | M2 |
| 7 | `commands/code-feature.md` | ~150 | M3 Lifecycle |
| 8 | `bin/merge-feature.sh` | ~60 | M3 |
| 9 | `hooks/user-prompt-submit.sh` | ~50 | M3 |
| 10 | `hooks/stop.sh` | ~40 | M3 |
| 11 | `commands/code-amend.md` | ~60 | v1.1 |
| 12 | `install.sh` | ~80 | M3 |

The keystone (`detect-context.sh`) ships first because everything else depends on its output contract.

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| `detect-context.sh` becomes a kitchen sink | Hard contract: inputs (one positional + optional stdin), outputs (one JSON object). No mutations. Add new fields only when a hook or command needs them. |
| Hook latency on every prompt | All four hooks delegate to `detect-context.sh` which is bash + jq. Target ≤ 50ms. SessionStart is the only hook that does meaningful I/O (reading lessons.md). |
| Skill missing at runtime breaks command | `detect-context.sh` returns warnings + drops missing skills from `skills_to_load`. Commands check the JSON before delegating. |
| `.phase` corruption | `set-phase.sh` validates before writing. `detect-context.sh` returns `workflow=none` + warning on parse failure. |
| Two hard blocks turn out to be wrong | They're easy to remove from `pre-tool-use.sh`. Adding more is harder politically — start narrow. |
| Strategy refocus clobbers user-written branch docs | Strategy SPEC step diffs against existing files; `/spec` skill never auto-deletes. Branch docs (`product-*.md`, `tech-*.md`) are owned by the user, not the command. |
| Feature merge inserts duplicates into global specs | `merge-feature.sh` is the only writer. It diffs sections; conflicts are flagged for the user, not auto-resolved. |
| Quick task balloons mid-flight | `/code:quick` detects scope growth (>5 files touched) and suggests `/code:feature`. User decides. |
| `~/.claude` already has hooks from another framework | `install.sh` writes shards-code's settings.json next to existing files; user merges manually. We don't auto-merge to avoid clobbering. |

---

## Branch Documents

| Document | Covers |
|---|---|
| **[tech-detect-context.md](tech-detect-context.md)** | The keystone routing script: full input/output contract, routing rules per workflow + phase, fallback behavior, JSON schema. |
| **[tech-hooks.md](tech-hooks.md)** | Detailed spec for each of the four hooks: trigger event, matcher, stdin format, what it reads, what it writes to stderr, exit codes, the two hard blocks. |
