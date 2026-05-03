# shards-code

A personal Claude Code framework. Three commands, four hooks, one routing script — orchestrating Compound Engineering, Superpowers, and a bundled `/spec` skill into a portable, gently-enforced workflow.

> Personal config made portable. Expect to fork.

---

## What it is

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

---

## Layers

| Layer | What | Files |
|---|---|---|
| **Workflow steering** | Hooks that nudge and enforce. Reactive — fires on Claude Code events. | `hooks/*.sh`, `settings.json` |
| **Skill facades** | Three slash commands. The user surface. Active — drives sub-phases and state. | `commands/code-*.md` |
| **Policy** | Prose telling Claude how to interpret prompts and pick a command. | `claude/CLAUDE.md` |
| **State scripts** | Single source of routing truth, state writer, COMPOUND merger. Mechanical. | `bin/detect-context.sh`, `bin/set-phase.sh`, `bin/merge-feature.sh` |

Plus the bundled **`/spec` skill** (`.agents/skills/spec/`) — independently versioned, used by `strategy:DESIGN:SPEC` and `feature:DESIGN:SPEC`. Treated as an external dependency that happens to ship with the framework.

External dependencies (NOT bundled — install yourself):
- **Compound Engineering** — `/ce-strategy`, `/ce-ideate`, `/ce-plan`, `/ce-deepen-plan`, `/ce-code-review`, `/ce-commit-push-pr`, `/ce-compound`, `/ce-work`
- **Superpowers** — `brainstorming`, `test-driven-development`, `subagent-driven-development`, `systematic-debugging`

---

## The Three Commands

| Command | When | Outputs |
|---|---|---|
| **`/code:quick <task>`** | Ad-hoc fix, ≤5 files, no architecture change | Source + ephemeral `.spec/.quick-plan.md` |
| **`/code:strategy`** | Project bootstrap or major refocus | Global specs (`product.md`, `tech.md`, optional design + plan) |
| **`/code:feature <name>`** | Build a real feature end-to-end | Feature specs → source → global spec deltas (only on COMPOUND) |

---

## The Hard Blocks

The `pre-tool-use` hook blocks exactly these writes:

- `.spec/lessons.md` outside `feature:IMPL:COMPOUND` — lessons are append-only, session-end records.
- Global specs (`product.md`, `tech.md`, `plan.md`, `product-*.md`, `tech-*.md`) outside `strategy:DESIGN:SPEC` or `feature:IMPL:COMPOUND` — globals only mutate during bootstrap or COMPOUND merge.
- Direct `.spec/.phase` edits — use `bin/set-phase.sh` instead. Structural privilege.

Everything else is a stderr warning.

---

## Install

```bash
git clone https://github.com/lennardzuendorf/shards-code.git ~/.shards-code
cd ~/.shards-code
./install.sh /path/to/your/project
```

`install.sh` (M3) symlinks `bin/`, `hooks/`, `commands/` into `~/.claude/shards-code/`, writes `<project>/.claude/settings.json` (offers a diff if one already exists), and adds `.spec/.phase` + `.spec/.quick-plan.md` to the project's `.gitignore`.

Prerequisites: bash 4+, jq, git. Compound Engineering and Superpowers must be installed separately.

---

## Status

v1 is in `.spec/` — specs only, no implementation yet. Build sequence: M1 Foundation → M2 Enforcement → M3 Lifecycle.

| Milestone | Status |
|---|---|
| M1: Foundation (`detect-context.sh`, `set-phase.sh`, `/code:quick`, `session-start.sh`) | not started |
| M2: Enforcement (`pre-tool-use.sh`, `/code:strategy`) | not started |
| M3: Lifecycle (`/code:feature`, `merge-feature.sh`, two hooks, `install.sh`) | not started |
| v1.1: `/code:amend`, `.shards/config.json` | deferred |

---

## Documentation

- **[`.spec/product.md`](.spec/product.md)** — story, requirements, design principles, target user.
- **[`.spec/tech.md`](.spec/tech.md)** — architecture, layers, stack, state file format, build sequence.
- **[`.spec/plan.md`](.spec/plan.md)** — milestone roadmap, open decisions, validation criteria.
- **[`.spec/features/`](.spec/features/)** — per-feature requirements + architecture (commands, routing, hooks).
- **[`.spec/archive/engineering-agent/`](.spec/archive/engineering-agent/)** — distilled prior art from a predecessor design.
- **[`.agents/skills/spec/SKILL.md`](.agents/skills/spec/SKILL.md)** — the bundled spec skill (v1.2).
