---
type: branch
parent: tech.md
scope: routing
covers: detect-context.sh contract, inputs, outputs, JSON schema, routing rules, fallback behavior
updated: 2026-05-03
---

# `bin/detect-context.sh` — The Keystone

The keystone routing script. Hooks call it. Commands call it. It is the single source of truth about the current workflow, phase, and which skills to load. Everything else in shards-code reads its JSON output.

**Parent:** [tech.md](tech.md) | **Sibling:** [tech-hooks.md](tech-hooks.md)

---

## 1. Contract

### Inputs

```bash
bin/detect-context.sh <command-context>
```

| Argument | Required | Values | Meaning |
|---|---|---|---|
| `command-context` | yes (positional `$1`) | `quick`, `strategy`, `feature`, `none` | Lets the caller assert what command (if any) is currently running. `none` means "no command active — figure it out from `.phase` and project state." |

**Optional stdin:** when called from a hook, the hook's JSON event payload may be piped in. The script may use it for prompt-keyword detection in `UserPromptSubmit`, but routing decisions otherwise rely on `.phase` and the file system, not stdin.

### Outputs

A single JSON object on stdout. Always valid JSON, even on error. Never writes to stdin/stdout other than this. Diagnostic information goes in the `warnings` array, not stderr (the calling hook decides whether to print warnings to stderr).

Exit code 0 on success — and only on success, but "success" includes "the project is in a weird state" (returns warnings, not failure). Exit non-zero only on programmer error (missing `jq`, etc.).

### File system reads

```
.spec/.phase                          # the state file
.spec/STRATEGY.md OR .spec/product.md # global spec presence
.spec/tech.md                         # ditto
.spec/lessons.md                      # for top-N entries
.spec/features/                       # listing of active features
.spec/archive/                        # listing of archived features
~/.claude/skills/<plugin>/...         # to verify skill availability
.agents/skills/spec/                  # to verify bundled spec skill
```

It does not write anything. Ever. State mutations go through `bin/set-phase.sh`.

---

## 2. Output JSON Schema

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

### Field semantics

| Field | Type | Notes |
|---|---|---|
| `workflow` | enum string | Mirrors first segment of `.phase`. `none` when `.phase` is empty or missing. |
| `phase` | string or null | Concatenation of segments 2 and 3 of `.phase` (e.g. `DESIGN:RESEARCH`). `null` for `quick` or neutral. |
| `active_feature` | string or null | Segment 4 of `.phase`. Only non-null when `workflow=feature`. |
| `skills_to_load` | array | `<plugin>:<skill>` strings. May be empty. Missing-but-needed skills are dropped from this array and added to `warnings`. |
| `plan_path` | string or null | Where the relevant plan lives for the current phase. `.spec/.quick-plan.md` for quick, `.spec/plan.md` for strategy, `.spec/features/<name>/plan.md` for feature. |
| `spec_paths.product/tech/design` | string or null | Active feature's specs when `workflow=feature`, else global specs. `design` only set when a design doc exists. |
| `global_context` | array | Paths the command should read for context. Always includes `lessons.md` when present. Includes globals for `quick` and feature `IMPL:*`; includes nothing for `strategy:DESIGN:RESEARCH` (it's writing them). |
| `warnings` | array | Human-readable advisory strings. Hooks may print these to stderr. |
| `next_suggested_command` | string or null | Used by `UserPromptSubmit` and SessionStart to nudge the user. |

---

## 3. Routing Rules

The script implements the following decision table. Each row shows the inputs (workflow + phase) and the resulting output fields that depend on routing.

### `workflow=none` (no `.phase`, no command context)

```
no .spec/                      → skills:[], suggest /code:strategy
.spec/ exists, no specs        → skills:[], suggest /code:strategy
.spec/ exists, specs present   → skills:[], suggest /code:quick OR /code:feature based on prompt keywords (if stdin)
```

### `workflow=quick`

```
skills_to_load:
  • superpowers:brainstorming           (only if non-trivial — heuristic: prompt has "design"/"plan"/"how should")
  • superpowers:test-driven-development (always, when available)
plan_path:      .spec/.quick-plan.md
spec_paths:     null (quick doesn't write specs)
global_context: [lessons.md, product.md, tech.md]   (skim only)
phase:          null
```

### `workflow=strategy`

| Phase | `skills_to_load` |
|---|---|
| `DESIGN:RESEARCH` | `compound-engineering:ce-strategy`, `compound-engineering:ce-ideate` |
| `DESIGN:DISCUSS` | `superpowers:brainstorming` |
| `DESIGN:SPEC` | `/spec` |
| `DESIGN:PLAN` | `compound-engineering:ce-plan`, `compound-engineering:ce-deepen-plan` |

```
spec_paths:     { product: .spec/product.md, tech: .spec/tech.md, design: .spec/product-design-language.md if present else null }
plan_path:      .spec/plan.md
global_context: [lessons.md] only — strategy is writing the rest
```

### `workflow=feature`

| Phase | `skills_to_load` |
|---|---|
| `DESIGN:RESEARCH` | `compound-engineering:ce-ideate` |
| `DESIGN:DISCUSS` | `superpowers:brainstorming` |
| `DESIGN:SPEC` | `/spec` |
| `DESIGN:PLAN` | `compound-engineering:ce-plan`, `compound-engineering:ce-deepen-plan` |
| `IMPL:VERIFY` | `[]` (built-in only) |
| `IMPL:WORK` | `superpowers:test-driven-development`, `superpowers:subagent-driven-development` |
| `IMPL:REVIEW` | `compound-engineering:ce-code-review` |
| `IMPL:SHIP` | `compound-engineering:ce-commit-push-pr` |
| `IMPL:COMPOUND` | `compound-engineering:ce-compound` |

```
spec_paths:     {
                  product: .spec/features/<name>/product.md,
                  tech:    .spec/features/<name>/tech.md,
                  design:  .spec/features/<name>/design.md if present else null
                }
plan_path:      .spec/features/<name>/plan.md
global_context:
  DESIGN:*  →  [lessons.md, product.md, tech.md]
  IMPL:*    →  [lessons.md, product.md, tech.md]   (read-only during IMPL)
```

---

## 4. Fallback Behavior

| Condition | Behavior |
|---|---|
| `.phase` missing | `workflow=none`, no warning (this is the neutral state) |
| `.phase` corrupted (doesn't match grammar) | `workflow=none`, warning: `".spec/.phase is malformed: <content>; treating as neutral"` |
| `.phase` says `feature:*` but no `.spec/features/<name>/` directory | warning: `"feature <name> has no spec directory; was it archived?"`, `active_feature` still set, command should error explicitly |
| Skill in `skills_to_load` not installed | drop from array, add `warnings: "skill <plugin>:<skill> not installed; falling back"` |
| `jq` not installed | exit 2, error to stderr (this is the only hard error) |
| `.spec/` directory missing entirely | `workflow=none`, `next_suggested_command="/code:strategy"`, no warning (greenfield is normal) |

The script never errors silently. The contract is: **always output valid JSON, never crash a session.** Programmer errors (missing `jq`) are the only exception.

---

## 5. Skill Availability Detection

The script needs to know which `<plugin>:<skill>` entries are actually installed. Approach:

```bash
# Check known install paths for the three external systems
CE_INSTALLED=$([ -d "$HOME/.claude/skills/compound-engineering" ] && echo true || echo false)
SP_INSTALLED=$([ -d "$HOME/.claude/skills/superpowers" ] && echo true || echo false)
SPEC_INSTALLED=$([ -d "$HOME/.agents/skills/spec" ] && echo true || echo false)
```

Per-skill granularity is not needed in v1: if the plugin is present, assume its named skills are available. If a specific skill turns out to be missing, the calling command will surface the error. Per-skill detection is v1.1.

---

## 6. Performance Contract

Target ≤ 50ms per call. The script runs on every PreToolUse, every UserPromptSubmit, every Stop, and at SessionStart. Cumulative latency matters.

Strategies:
- No subprocess except `jq` and a few `[ -f ]` / `[ -d ]` checks.
- No network. No git commands (use `[ -d .git ]` if needed).
- No reading file contents beyond `.spec/.phase` and the lessons.md head (for SessionStart).
- `jq` is invoked once at the end to assemble the JSON.

---

## 7. Worked Example

Input:
```bash
echo '' | bin/detect-context.sh feature
```

Filesystem state:
```
.spec/.phase  →  "feature:IMPL:WORK:dark-mode\n"
.spec/features/dark-mode/  →  exists, contains product.md, tech.md, plan.md
~/.claude/skills/superpowers  →  exists
~/.claude/skills/compound-engineering  →  missing
.spec/lessons.md  →  exists
.spec/product.md  →  exists
.spec/tech.md  →  exists
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
  "global_context": [
    ".spec/lessons.md",
    ".spec/product.md",
    ".spec/tech.md"
  ],
  "warnings": [],
  "next_suggested_command": null
}
```

If `compound-engineering` were needed (e.g., during `IMPL:REVIEW`) and missing, the corresponding skill would be dropped and a warning appended.

---

## 8. Open Questions

1. **Per-skill detection for v1.1.** Today we trust that a present plugin has its named skills. What's the lightest-weight way to verify a specific skill (`grep` the SKILL.md for the name? read the plugin's manifest)?
2. **Caching.** If the script becomes a bottleneck at SessionStart we could cache the JSON for the session — but caching across hook invocations is risky because `.phase` changes mid-turn. Defer until measured.
3. **Stdin prompt parsing.** UserPromptSubmit may pipe the prompt in for keyword routing. Where does the keyword list live (hard-coded, a config file)? Default: hard-coded in `detect-context.sh` for v1. Move to config in v1.1.
4. **Per-project skill overrides.** Could a project pin different skills for a phase via `.shards/config.json`? Yes — that's exactly what `.shards/config.json` is for in v1.1. Out of scope for v1.
