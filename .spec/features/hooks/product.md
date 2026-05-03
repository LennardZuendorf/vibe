---
type: feature-product
feature: hooks
sibling: tech.md
parent: ../../product.md
updated: 2026-05-03
---

# Feature: Hooks — Product

Four hooks turn shards-code's policy into mild enforcement. Three are stderr-only — they nudge but never block. The fourth (`pre-tool-use`) has exactly two structural blocks plus a privileged path block. Together they make the workflow visible and resistant to footguns without being adversarial.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)

---

## Why this feature exists

CLAUDE.md alone is advice — it tells Claude what to do, but Claude can drift. Hooks are the enforcement layer: they fire on actual events (session start, prompt submission, tool use, turn end) and read the routing JSON to decide what to do. The result: policy that's visible (warnings on stderr the user actually sees) and policy that's enforced where it matters (lessons.md and global specs only mutate during COMPOUND).

The design is gentle: warnings dominate, blocks are rare. Two hard blocks plus the privileged-path block. If a third becomes necessary, evidence first.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | Four hooks total: SessionStart, UserPromptSubmit, PreToolUse, Stop. |
| R2 | Three of four (all except PreToolUse) are stderr-only — always exit 0. |
| R3 | PreToolUse has exactly three hard-block conditions and no more in v1. |
| R4 | Hard blocks come with stderr messages that explain what to do instead (not just "blocked"). |
| R5 | Warnings cite the active phase and reference the rule, so the user can either fix or override. |
| R6 | Hooks degrade gracefully if `bin/detect-context.sh` is missing or misbehaving — they exit 0 silently rather than blocking the user. |
| R7 | Latency: the hottest hooks (PreToolUse, UserPromptSubmit) target ≤ 30ms. SessionStart can afford 100ms. |
| R8 | The hook layer is the only place that enforces phase-aware write rules. Commands trust that the hooks are doing their job. |

---

## What the User Sees

### At session start

```
--- shards-code ---
Phase:   feature:IMPL:WORK:dark-mode
Skills:  superpowers:test-driven-development, superpowers:subagent-driven-development
Lessons (top 3):
  1. Don't reimplement what plugins already do
  2. Spec framework assumes greenfield — rework projects need more file types
  3. Design system docs break the product/tech separation — and that's correct
-------------------
```

When neutral:
```
--- shards-code ---
Phase:   (neutral)
Suggested: /code:strategy (no specs found)
-------------------
```

### When typing a prompt that mismatches the active phase

```
[shards-code] You're in feature:DESIGN:SPEC:dark-mode but the prompt asks for implementation.
              Either finish the SPEC + PLAN, or run /code:amend to adjust scope.
```

### When trying to write a hard-blocked file

```
[shards-code] BLOCKED: lessons.md may only be written during feature:IMPL:COMPOUND.
              Current phase: feature:IMPL:WORK:dark-mode.
```

### At end of turn (occasional smell warning)

```
[shards-code] WORK turn changed source without touching tests. TDD?
```

---

## The Three Hard Blocks

The hook layer hard-blocks exactly three categories of write. Each is justified by a structural invariant whose violation would corrupt downstream tooling.

| Block | Why it's hard, not soft |
|---|---|
| **`.spec/lessons.md` outside `feature:IMPL:COMPOUND`** | Lessons are append-only, session-end records. Mid-flight edits invite scratchpad pollution and undermine the lesson's role as durable wisdom. |
| **Global specs (`product.md`, `tech.md`, `plan.md`, `product-*.md`, `tech-*.md`) outside `strategy:DESIGN:SPEC` or `feature:IMPL:COMPOUND`** | Cross-cutting decisions only land in globals through bootstrap or the COMPOUND merge. Allowing ad-hoc edits during WORK turns globals into a feature scratchpad and defeats the global/feature spec separation. |
| **Direct edit to `.spec/.phase`** | The state file is structurally privileged; bypassing `bin/set-phase.sh` skips validation and corrupts state. The block exists so commands can rely on `.phase` being well-formed. |

Anything else (writing source during DESIGN, source mid-strategy, etc.) is a warning.

---

## What the Hooks Do NOT Do

- Block any tool other than `Edit`/`Write`/`NotebookEdit`. v1 doesn't gate `Bash`, `Read`, `Grep`, etc. Adding a Bash matcher is a v1.1 consideration only on evidence of misuse (e.g. `rm -rf .spec/`).
- Inject content into Claude's context. Suggestions go to stderr only. Skill auto-loading is a v1.1 question.
- Run tests, linters, or formatters. Stop hook does turn-end smell checks, but never executes test suites.
- Block the session from continuing. The worst hook outcome is a single tool call denied with a clear message.

---

## Non-Goals

- A configurable hook list. Four hooks, hard-coded. v1.
- Per-project hook overrides. The same four hooks, with the same routing logic, fire in every shards-code project.
- Hook chaining. Each hook is a single bash script. If a hook needs to do multiple things, it does them in one script.
- Web UI / dashboard for hook activity. Stderr is the UI.

---

## Open Questions

1. **PreToolUse on `Bash` tool.** Should we also gate `Bash` to catch `rm -rf .spec/` etc.? Default: no in v1 — too broad. Add only on evidence of real misuse.
2. **Stop-hook turn counter location.** `.spec/.phase-turns` (gitignored file) or session tmpfile? Default: gitignored file (simpler, persists across restarts).
3. **Phase-fight detection sensitivity.** UserPromptSubmit detects "you're in DESIGN but asking for code". Tunable false-positive rate. Default: only suggest, never block; users will tell us if it's noisy.
4. **Hook installation scope.** Per-project (under `<project>/.claude/`) is the default. Power users may want a global install at `~/.claude/`. v1.1 if needed.
