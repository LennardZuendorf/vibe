---
type: feature-tech
feature: routing
sibling: product.md
parent: ../../tech.md
updated: 2026-05-03
---

# Feature: Routing — Architecture

The keystone (`bin/detect-context.sh`) plus the state writer (`bin/set-phase.sh`). One emits JSON describing the current state; the other validates and writes the state file. Together they're the single source of routing truth.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)

---

## Files

```
bin/
├── detect-context.sh       # KEYSTONE — read state, emit JSON  ~150 LOC
└── set-phase.sh            # only sanctioned .phase writer     ~30 LOC
```

---

## `bin/detect-context.sh`

### Contract

```bash
bin/detect-context.sh <command-context>
```

| Input | Required | Values | Meaning |
|---|---|---|---|
| `command-context` (`$1`) | yes | `quick`, `strategy`, `feature`, `none` | Lets the caller assert what's running. `none` = "figure it out from `.phase` + project state." |
| stdin | optional | hook event JSON | Used for prompt-keyword detection in `UserPromptSubmit`. |

**Reads:** `.spec/.phase`, `.spec/product.md` and `.spec/tech.md` existence, `.spec/features/` listing, `.spec/lessons.md` (head only), `~/.claude/skills/<plugin>/` for plugin presence, `.agents/skills/spec/` for bundled skill.

**Writes:** nothing. Ever.

**Exit:** `0` on success (including degraded states with warnings). Non-zero only on programmer error (missing `jq`).

### Output Schema

```json
{
  "workflow":   "feature | strategy | quick | none",
  "phase":      "DESIGN:RESEARCH | DESIGN:DISCUSS | DESIGN:SPEC | DESIGN:PLAN | IMPL:VERIFY | IMPL:WORK | IMPL:REVIEW | IMPL:SHIP | IMPL:COMPOUND | null",
  "active_feature": "<name> | null",
  "skills_to_load": ["<plugin>:<skill>", "..."],
  "plan_path":  "<path> | null",
  "spec_paths": {
    "product": "<path> | null",
    "tech":    "<path> | null",
    "design":  "<path> | null"
  },
  "global_context": ["<path>", "..."],
  "warnings": ["<message>", "..."],
  "next_suggested_command": "<command> | null"
}
```

| Field | Notes |
|---|---|
| `workflow` | Mirrors first segment of `.phase`. `none` when empty/missing. |
| `phase` | Concatenation of segments 2 + 3 of `.phase`. `null` for `quick` and neutral. |
| `active_feature` | Segment 4 of `.phase`. Only non-null when `workflow=feature`. |
| `skills_to_load` | `<plugin>:<skill>` strings. Missing-but-needed skills dropped + added to `warnings`. |
| `plan_path` | `.spec/.quick-plan.md` for quick, `.spec/plan.md` for strategy, `.spec/features/<name>/plan.md` for feature. |
| `spec_paths.*` | Active feature's specs when `workflow=feature`, else global specs. `design` only when a design doc exists. |
| `global_context` | Paths to read for context. Always includes `lessons.md` when present. |
| `warnings` | Human-readable advisory strings. Hooks may print to stderr. |
| `next_suggested_command` | Used by `UserPromptSubmit` and `SessionStart` to nudge the user. |

### Routing Rules

`workflow=none` (no `.phase`, no command context):
```
no .spec/                      → skills:[], suggest /code:strategy
.spec/ exists, no specs        → skills:[], suggest /code:strategy
.spec/ exists, specs present   → skills:[], suggest /code:quick or /code:feature based on prompt keywords
```

`workflow=quick`:
```
skills_to_load:
  • superpowers:brainstorming           (only if prompt suggests non-trivial)
  • superpowers:test-driven-development (always when available)
plan_path:      .spec/.quick-plan.md
spec_paths:     null
global_context: [lessons.md, product.md, tech.md]
phase:          null
```

`workflow=strategy` — phase determines skills:

| Phase | `skills_to_load` |
|---|---|
| `DESIGN:RESEARCH` | `compound-engineering:ce-strategy`, `:ce-ideate` |
| `DESIGN:DISCUSS` | `superpowers:brainstorming` |
| `DESIGN:SPEC` | `/spec` |
| `DESIGN:PLAN` | `compound-engineering:ce-plan`, `:ce-deepen-plan` |

```
spec_paths: { product: .spec/product.md, tech: .spec/tech.md, design: .spec/product-design-language.md if present else null }
plan_path:  .spec/plan.md
global_context: [lessons.md] only — strategy is writing the rest
```

`workflow=feature` — phase determines skills:

| Phase | `skills_to_load` |
|---|---|
| `DESIGN:RESEARCH` | `compound-engineering:ce-ideate` |
| `DESIGN:DISCUSS` | `superpowers:brainstorming` |
| `DESIGN:SPEC` | `/spec` |
| `DESIGN:PLAN` | `compound-engineering:ce-plan`, `:ce-deepen-plan` |
| `IMPL:VERIFY` | `[]` (built-in) |
| `IMPL:WORK` | `superpowers:test-driven-development`, `:subagent-driven-development` |
| `IMPL:REVIEW` | `compound-engineering:ce-code-review` |
| `IMPL:SHIP` | `compound-engineering:ce-commit-push-pr` |
| `IMPL:COMPOUND` | `compound-engineering:ce-compound` |

```
spec_paths: {
  product: .spec/features/<name>/product.md,
  tech:    .spec/features/<name>/tech.md,
  design:  .spec/features/<name>/design.md if present else null
}
plan_path:  .spec/features/<name>/plan.md
global_context: [lessons.md, product.md, tech.md]   # read-only during IMPL
```

### Fallback Rules

| Condition | Behavior |
|---|---|
| `.phase` missing | `workflow=none`, no warning (neutral is normal) |
| `.phase` malformed | `workflow=none` + warning `".spec/.phase is malformed: <content>; treating as neutral"` |
| `feature:*` but no `.spec/features/<name>/` directory | warning `"feature <name> has no spec directory; was it archived?"`, `active_feature` still set, calling command should error explicitly |
| Skill in `skills_to_load` not installed | drop + add `warnings: "skill <name> not installed; falling back"` |
| `jq` not installed | exit 2 + stderr error (only hard error) |
| `.spec/` missing entirely | `workflow=none`, `next_suggested_command="/code:strategy"`, no warning |

### Skill Availability Detection

```bash
CE_INSTALLED=$([ -d "$HOME/.claude/skills/compound-engineering" ] && echo true || echo false)
SP_INSTALLED=$([ -d "$HOME/.claude/skills/superpowers" ] && echo true || echo false)
SPEC_INSTALLED=$([ -d "$HOME/.agents/skills/spec" ] && echo true || echo false)
```

Per-plugin granularity in v1. If the plugin is present, all its named skills are assumed available. Per-skill detection is v1.1.

### Performance Budget

Target ≤ 50ms per call. The script:
- No subprocess except `jq` and a few `[ -f ]` / `[ -d ]` tests.
- No network. No git commands (use `[ -d .git ]` if needed).
- No reading file contents beyond `.spec/.phase` and the `lessons.md` head (only when called from SessionStart).
- `jq` invoked once at the end to assemble JSON.

---

## `bin/set-phase.sh`

### Contract

```bash
bin/set-phase.sh <state>
```

`state` is the new value of `.spec/.phase`. Empty string clears it.

### Validation Grammar

```
state := "" | "quick" | strategy_state | feature_state
strategy_state := "strategy:DESIGN:" research|discuss|spec|plan
feature_state  := "feature:" cluster_state ":" feature_name
cluster_state  := "DESIGN:" research|discuss|spec|plan
                | "IMPL:"   verify|work|review|ship|compound
feature_name   := [a-z][a-z0-9-]*
```

Case-sensitive. `RESEARCH`/`DISCUSS`/`SPEC`/`PLAN`/`VERIFY`/`WORK`/`REVIEW`/`SHIP`/`COMPOUND` are uppercase. `quick`, `strategy`, `feature`, and feature names are lowercase.

### Behavior

```
1. Validate input against grammar
   - Invalid → exit 1 with stderr "invalid state: <value>"
2. Atomic write:
   - echo "$state" > .spec/.phase.tmp
   - mv .spec/.phase.tmp .spec/.phase
3. Exit 0
```

Idempotent for the same value. Writes never overlap because `mv` is atomic on the same filesystem.

### Why a separate script?

PreToolUse hard-blocks any `Edit`/`Write` to `.spec/.phase`. Commands need a way to update state; `set-phase.sh` is that way. The script's existence + the hard block is the structural pattern that keeps state coherent.

---

## Worked Example

Filesystem:
```
.spec/.phase                          → "feature:IMPL:WORK:dark-mode\n"
.spec/features/dark-mode/             → exists, has product.md + tech.md + plan.md
~/.claude/skills/superpowers          → exists
~/.claude/skills/compound-engineering → missing
.spec/lessons.md, product.md, tech.md → exist
```

Call:
```bash
echo '' | bin/detect-context.sh feature
```

Output:
```json
{
  "workflow": "feature",
  "phase": "IMPL:WORK",
  "active_feature": "dark-mode",
  "skills_to_load": [
    "superpowers:test-driven-development",
    "superpowers:subagent-driven-development"
  ],
  "plan_path": ".spec/features/dark-mode/plan.md",
  "spec_paths": {
    "product": ".spec/features/dark-mode/product.md",
    "tech": ".spec/features/dark-mode/tech.md",
    "design": null
  },
  "global_context": [".spec/lessons.md", ".spec/product.md", ".spec/tech.md"],
  "warnings": [],
  "next_suggested_command": null
}
```

If the active phase were `IMPL:REVIEW` (which needs `compound-engineering:ce-code-review`), the missing CE plugin would cause that skill to be dropped and a warning appended.

---

## Open Questions

1. **`jq` minimum version.** Output uses `jq -n --arg` to assemble. Pin a version or assume any modern `jq`? Default: any `jq` 1.6+ (widely available).
2. **Caching.** Re-call frequency is ~1× per hook per turn (4 hooks × N turns). Skip caching in v1; measure latency before optimizing.
3. **State-grammar evolution.** When (not if) we add a new phase, the validator in `set-phase.sh` and the routing rules in `detect-context.sh` must update together. They live in different files. Should we extract to a shared bash include? Default: no in v1, two-file edit is acceptable. Extract if the grammar grows past ~15 phases.
