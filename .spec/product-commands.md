---
type: branch
parent: product.md
scope: commands
covers: command UX, sub-phases, gates, output paths, escape hatches, error states
updated: 2026-05-03
---

# Commands

The three commands are the entire user surface of shards-code. This doc specifies their behavior in detail: when to use them, what they do step by step, what state transitions they cause, and how they degrade.

**Parent:** [product.md](product.md)
**Tech implementation:** Each command is a markdown file at `commands/code-*.md` that delegates to skills and `bin/` scripts. See [tech.md](tech.md) for the file layout and [tech-detect-context.md](tech-detect-context.md) for the routing JSON they consume.

---

## 1. `/code:quick <task>`

**Use when:** The change is small (≤5 files touched), there's no architecture decision, and you don't need the spec lifecycle. Bug fixes, copy tweaks, refactors-in-place, dependency bumps.

**Use _instead_ of `/code:feature` when** the task can be described in one sentence and verified by running tests once.

### Behavior

```
1. Run bin/detect-context.sh quick
2. Read STRATEGY-equivalent globals (product.md, tech.md, lessons.md) — skim only
3. Estimate task size:
     • Trivial (≤2 lines literal text, single file, no logic change)
         → implement directly
     • Non-trivial (any logic change, multiple files)
         → write .spec/.quick-plan.md
         → confirm: "Plan written. Proceed?"
         → on yes, implement
4. Run tests
5. Light review (NOT ce-code-review — too heavy)
6. Done
```

### State

Quick is **stateless**. It does not write `.spec/.phase`. If a feature workflow is already active, quick interrupts it without changing the phase — the user can run another `/code:quick` or pick the feature back up.

### Outputs

- Source code changes
- `.spec/.quick-plan.md` (gitignored, ephemeral) for non-trivial tasks
- No spec writes, no compound, no ship ceremony

### Escape hatches

- **Scope balloons.** If quick detects >5 files touched or scope creep ("while we're here, let me also..."), it stops, summarizes what's been done, and suggests `/code:feature <name>` to continue properly.
- **User cancels mid-plan.** Drop `.spec/.quick-plan.md`, don't touch source.

### What `/code:quick` deliberately does NOT do

- Write specs (product.md, tech.md, plan.md)
- Update lessons.md (only `IMPL:COMPOUND` does that)
- Run ce-code-review or ce-compound
- Push or open PRs (commit only on explicit user request)

---

## 2. `/code:strategy`

**Use when:** Bootstrapping a new project, or the project has drifted enough from its specs that a refocus is warranted. Re-runnable.

### Behavior

```
1. Run bin/detect-context.sh strategy
2. set-phase.sh strategy:DESIGN:RESEARCH
3. Check existing state:
     • No .spec/                 → run /spec setup
     • .spec/ exists with specs  → confirm refocus before overwriting
4. RESEARCH:
     • Delegate to ce-strategy + ce-ideate (parallel subagents)
     • Output: .spec/research/strategy-notes.md
     • set-phase.sh strategy:DESIGN:DISCUSS
5. DISCUSS:
     • Delegate to superpowers:brainstorming (Socratic, one question at a time)
     • set-phase.sh strategy:DESIGN:SPEC
6. SPEC:
     • Delegate to /spec skill
     • Writes: product.md, tech.md, optionally product-design-language.md
     • Run validate.sh
     • set-phase.sh strategy:DESIGN:PLAN
7. PLAN (optional):
     • Ask: "Want a top-level roadmap? y/n"
     • yes → ce-plan → plan.md
     • no  → skip
8. set-phase.sh "" (neutral)
9. Print summary, suggest /code:feature <name> for first feature
```

### State transitions

```
"" or any → strategy:DESIGN:RESEARCH → strategy:DESIGN:DISCUSS
        → strategy:DESIGN:SPEC → strategy:DESIGN:PLAN (optional) → ""
```

Strategy is fully resumable: re-running `/code:strategy` reads `.spec/.phase` and continues from the saved phase.

### Outputs

- `.spec/product.md`, `.spec/tech.md` (always)
- `.spec/product-design-language.md` (when project has UI)
- `.spec/plan.md` (only if user opts in to PLAN sub-phase)
- `.spec/research/strategy-notes.md` (RESEARCH artifact)

### Refocus semantics

When `.spec/` already exists, strategy asks before overwriting global specs. Branch docs (`product-*.md`, `tech-*.md`) the user wrote by hand are **never** auto-deleted. The SPEC sub-phase produces a diff and the user approves, edits, or skips each section.

### What strategy deliberately does NOT do

- Touch source code
- Write feature specs (`features/<name>/`) — that's `/code:feature`
- Update lessons.md
- Ship or commit

---

## 3. `/code:feature <name>`

**Use when:** Building a real, named feature with non-trivial scope. The full lifecycle is heavyweight on purpose — this is the discipline command.

### Behavior

```
1. Run bin/detect-context.sh feature
2. Validate prerequisites:
     • product.md and tech.md exist?  No → error "Run /code:strategy first"
     • <name> in .spec/archive/?       Yes → confirm reactivation
3. mkdir -p .spec/features/<name>/ if missing
4. set-phase.sh feature:DESIGN:RESEARCH:<name>

[ DESIGN sub-cluster — heavy, user-involved ]

5.  RESEARCH:  ce-ideate scoped to feature → .spec/features/<name>/research.md
                set-phase.sh feature:DESIGN:DISCUSS:<name>
6.  DISCUSS:   superpowers:brainstorming
                set-phase.sh feature:DESIGN:SPEC:<name>
7.  SPEC:      /spec skill → features/<name>/{product.md, tech.md, design.md if UI}
                run validate.sh
                set-phase.sh feature:DESIGN:PLAN:<name>
8.  PLAN:      ce-plan + ce-deepen-plan → features/<name>/plan.md
9.  HUMAN GATE: "Plan ready. Approve?" — block on user yes
                set-phase.sh feature:IMPL:VERIFY:<name>

[ IMPL sub-cluster — autonomous between gates ]

10. VERIFY:    built-in drift check
                  • Read feature spec
                  • Scan codebase for referenced files / interfaces / dependencies
                  • Drift found?
                       targeted → suggest /code:amend
                       major    → flag, may need DESIGN re-entry
                  set-phase.sh feature:IMPL:WORK:<name>
11. WORK:      Default: superpowers:tdd + superpowers:subagent-driven-development
                Override: user says "ralph it" → ce-work (less ceremony)
                Tests fail 3+ consecutive times → suggest superpowers:systematic-debugging
                set-phase.sh feature:IMPL:REVIEW:<name>
12. REVIEW:    ce-code-review
                  • P1 issues  → back to WORK
                  • P2/P3      → continue (logged in plan.md)
                  set-phase.sh feature:IMPL:SHIP:<name>
13. HUMAN GATE: "Ready to ship?"
14. SHIP:      ce-commit-push-pr
                set-phase.sh feature:IMPL:COMPOUND:<name>
15. COMPOUND:  ce-compound + built-in
                  • Append session learnings to .spec/lessons.md
                  • merge-feature.sh: cross-cutting from features/<name>/tech.md → global tech.md
                  • Move .spec/features/<name>/ → .spec/archive/<name>/
                  set-phase.sh ""
16. Print summary, suggest next command
```

### State transitions

```
"" → feature:DESIGN:RESEARCH:<name> → ...DESIGN:DISCUSS → ...DESIGN:SPEC → ...DESIGN:PLAN
   → [HUMAN GATE]
   → feature:IMPL:VERIFY → ...IMPL:WORK → ...IMPL:REVIEW
   → [HUMAN GATE]
   → feature:IMPL:SHIP → feature:IMPL:COMPOUND → ""
```

Resumable: re-running `/code:feature <same-name>` reads `.spec/.phase` and continues.

### Outputs

- `.spec/features/<name>/{product.md, tech.md, plan.md, research.md, design.md}` (created in DESIGN, read-only after)
- Source code (during WORK)
- A commit, push, and PR (during SHIP)
- Updated `.spec/lessons.md` (during COMPOUND, only)
- Updated global `.spec/tech.md` and friends (during COMPOUND, via `merge-feature.sh`)
- `.spec/archive/<name>/` (after COMPOUND)

### The two human gates

The lifecycle is autonomous _between_ gates. The user is asked to approve at exactly two points:

1. **After PLAN, before VERIFY.** Plan must be reviewed before code is written.
2. **After REVIEW, before SHIP.** A clean review is necessary but not sufficient — the user authorizes the actual push.

Everything else is autonomous, including state transitions, skill delegation, and test runs.

### Skill delegation rules

| Sub-phase | Default skill | Override | Fallback if missing |
|---|---|---|---|
| DESIGN:RESEARCH | `compound-engineering:ce-ideate` | — | built-in Explore agents |
| DESIGN:DISCUSS | `superpowers:brainstorming` | — | built-in `AskUserQuestion` loop |
| DESIGN:SPEC | `/spec` | — | hard-required, command errors |
| DESIGN:PLAN | `compound-engineering:ce-plan` + `ce-deepen-plan` | — | built-in plan template |
| IMPL:VERIFY | built-in | — | — |
| IMPL:WORK | `superpowers:test-driven-development` + `subagent-driven-development` | "ralph it" → `ce-work` | direct implementation |
| IMPL:REVIEW | `compound-engineering:ce-code-review` | — | `superpowers:` review (if available), else built-in checklist |
| IMPL:SHIP | `compound-engineering:ce-commit-push-pr` | — | direct git commands |
| IMPL:COMPOUND | `compound-engineering:ce-compound` + built-in merger | — | built-in merger only |

### Escape hatches

- **`/code:amend` (v1.1)** when DESIGN drifted from reality and IMPL needs targeted spec amendment without rewinding.
- **Manual phase override.** Power users can edit `.spec/.phase`... no they can't, that's hard-blocked. Use `bin/set-phase.sh feature:<phase>:<name>` instead.
- **Abort.** `/code:feature --abort <name>` (v1.1) — wipe `.spec/features/<name>/`, reset phase to "". Until v1.1, the user does this manually.

---

## 4. `/code:amend` (v1.1)

**Use when:** Mid-IMPL, the codebase reality has diverged from the feature spec enough that targeted amendment is needed without abandoning the work. Smaller than re-entering DESIGN.

### Behavior (sketch)

```
1. Read current .spec/.phase
2. Reject if not feature:IMPL:*
3. Open the feature's product.md and tech.md
4. Diff against current codebase reality
5. Propose targeted patches (not rewrites)
6. User approves each patch
7. Resume IMPL:WORK
```

Specced for v1.1; not built in v1. Documented here so the surface area is visible.

---

## 5. Cross-Command Behavior

### What every command does at start

1. Call `bin/detect-context.sh <workflow>` and parse the JSON.
2. Read `warnings` array; surface non-empty warnings to the user.
3. Read `skills_to_load`; verify each is available; degrade gracefully on missing.
4. Read `global_context`; load only those specs (progressive disclosure).

### What every command does at end

1. Call `bin/set-phase.sh <next-phase>` to advance state (or `""` to clear).
2. Print one-line summary of what changed.
3. Suggest the next command, if applicable.

### What no command may do

- Write `.spec/.phase` directly. Always go through `bin/set-phase.sh`.
- Write `.spec/lessons.md` outside `feature:IMPL:COMPOUND`. Hard-blocked by PreToolUse.
- Write global specs (`.spec/product.md`, `.spec/tech.md`, etc.) outside `feature:IMPL:COMPOUND` or `strategy:DESIGN:SPEC`. Hard-blocked by PreToolUse.
- Skip the human gates in `/code:feature`.

---

## 6. Open Questions

1. **Quick threshold.** "Any logic change → plan" feels right but might be friction for one-line behavioral fixes. Watch real usage and tune. Default: stricter for v1.
2. **Strategy refocus diff UX.** When refocusing, do we present the full proposed diff in one block, or section by section with approval gates? Sectioned feels less overwhelming but takes longer.
3. **Feature reactivation.** When `<name>` is in `.spec/archive/`, do we copy it back to `features/<name>/` and resume from `feature:DESIGN:SPEC:<name>`, or treat it as fresh DESIGN? Default: copy + resume from SPEC, user can rewind further if needed.
4. **`ce-work` "ralph it" override syntax.** Plain English ("just code it", "ralph it", "skip TDD") parsed by command, or explicit flag (`/code:feature <name> --no-tdd`)? Default: detect natural language, document the phrases.
5. **`/code:amend` priority.** v1.1 is the default. Promote to v1 only if real drift becomes a frequent pain in the first project.
