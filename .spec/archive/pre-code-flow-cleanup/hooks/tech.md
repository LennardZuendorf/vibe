---
type: feature-tech
feature: hooks
sibling: product.md
parent: ../../tech.md
updated: 2026-05-03
---

# Feature: Hooks — Architecture

Per-hook trigger, matcher, decision rules, exit codes, and the settings.json registration. All four hooks delegate to `bin/detect-context.sh` and operate on its JSON output.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)

---

## Files

```
hooks/
├── session-start.sh        # SessionStart hook         ~40 LOC
├── user-prompt-submit.sh   # UserPromptSubmit hook     ~50 LOC
├── pre-tool-use.sh         # PreToolUse hook           ~100 LOC
└── stop.sh                 # Stop hook                 ~40 LOC

settings.json               # registers all four        ~30 LOC
```

---

## Common Bash Conventions

Every hook starts with the same skeleton (lifted from prior-art `check-phase.sh`, see `archive/engineering-agent/insights.md`):

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read hook event JSON from stdin
INPUT=$(cat)

# Resolve detect-context.sh location across known paths
SCRIPT=""
for candidate in \
  "$PWD/bin/detect-context.sh" \
  "$HOME/.claude/shards-code/bin/detect-context.sh" \
  "$(git rev-parse --show-toplevel 2>/dev/null)/bin/detect-context.sh"; do
  [ -f "$candidate" ] && SCRIPT="$candidate" && break
done

# Graceful degradation: if keystone is missing, do nothing
[ -z "$SCRIPT" ] && exit 0

CONTEXT=$(echo "$INPUT" | bash "$SCRIPT" none)
```

The `for candidate` pattern makes hooks portable across machines without env-var configuration.

---

## `hooks/session-start.sh`

| Property | Value |
|---|---|
| Trigger | `SessionStart` |
| Matcher | `startup` (only fires once per session) |
| stdin | SessionStart event JSON (mostly unused) |
| Exit | always `0` |
| LOC | ~40 |

**Behavior:** call `detect-context.sh none`, parse the JSON, print to stderr:

```
--- shards-code ---
Phase:   <workflow>:<phase>:<feature>   OR   "(neutral)"
Skills:  <skills_to_load joined>
Lessons (top 3):
  1. <heading 1>
  2. <heading 2>
  3. <heading 3>
Suggested: <next_suggested_command>     # only when neutral
-------------------
```

**Reads:** routing JSON, `.spec/lessons.md` head (top 3 `### ` headings).

---

## `hooks/user-prompt-submit.sh`

| Property | Value |
|---|---|
| Trigger | `UserPromptSubmit` |
| Matcher | (all) |
| stdin | UserPromptSubmit event JSON, including the prompt text |
| Exit | always `0` |
| LOC | ~50 |

**Skip conditions** (exit 0 without doing anything):
- Prompt starts with `/` (already a command)
- Prompt is empty
- Active phase is `feature:IMPL:*` (don't second-guess in-progress work)

**Routing keywords** (case-insensitive, hard-coded in `detect-context.sh`):

| Trigger | Suggestion |
|---|---|
| "fix", "tweak", "small change", "rename", "typo" | `/code:quick <task>` |
| "build", "implement", "feature", "add ... to" | `/code:feature <name>` |
| "plan", "architect", "strategy", "refocus", "what should we build" | `/code:strategy` |

**Phase-fight detection:** if active phase is `*:DESIGN:*` and prompt contains code-action keywords ("write the function", "implement", "fix the bug"), suggest `/code:amend` (v1.1) or breaking out via `bin/set-phase.sh`. Symmetric for `*:IMPL:*` + design keywords.

**Output (to stderr):**
```
[shards-code] Looks like a small fix — consider /code:quick "<paraphrased task>"
```
or
```
[shards-code] You're in feature:DESIGN:SPEC:dark-mode but the prompt asks for implementation.
              Either finish the SPEC + PLAN, or run /code:amend to adjust scope.
```

---

## `hooks/pre-tool-use.sh`

| Property | Value |
|---|---|
| Trigger | `PreToolUse` |
| Matcher | `Edit\|Write\|NotebookEdit` |
| stdin | PreToolUse event JSON with `tool_name`, `tool_input.file_path` |
| Exit | `0` (allow) or `2` (block) |
| LOC | ~100 |

### Decision Rules (in order)

```
let path = tool_input.file_path
let { workflow, phase, active_feature } = detect-context.sh feature

# ----- HARD BLOCKS (exit 2 + stderr) -----

if path == ".spec/.phase":
    BLOCK("`.spec/.phase` is structurally privileged. Use `bin/set-phase.sh <state>` instead.")

if path matches ".spec/lessons.md":
    if not (workflow == "feature" and phase == "IMPL:COMPOUND"):
        BLOCK("`lessons.md` may only be written during feature:IMPL:COMPOUND. Current: <workflow>:<phase>.")

if path matches one of [.spec/product.md, .spec/tech.md, .spec/plan.md, .spec/product-*.md, .spec/tech-*.md]:
    if workflow == "strategy" and phase startswith "DESIGN:SPEC":
        ALLOW
    elif workflow == "feature" and phase == "IMPL:COMPOUND":
        ALLOW
    else:
        BLOCK("Global specs may only be written during strategy:DESIGN:SPEC or feature:IMPL:COMPOUND. Current: <workflow>:<phase>.")

# ----- WARNINGS (exit 0 + stderr) -----

if .phase missing and path startswith ".spec/":
    WARN("No active workflow. Run /code:strategy, /code:feature, or /code:quick first.")

if workflow == "quick" and path matches global specs pattern:
    WARN("Quick mode shouldn't be writing specs. Did you mean /code:feature?")

if workflow == "strategy" and path is source-code-like (not .spec/, not docs):
    WARN("You're in strategy DESIGN — source files shouldn't be edited yet.")

if workflow == "feature" and phase startswith "DESIGN" and path is source-code-like:
    WARN("You're in feature DESIGN. Source files come during IMPL:WORK.")

# Default: ALLOW
ALLOW
```

### Source-Code-Like Detection

Heuristic only (it's a warning, not a block): anything outside `.spec/`, outside `.git/`, and not matching `*.md` at the project root, OR matching a known source extension (`.ts`, `.tsx`, `.js`, `.jsx`, `.py`, `.go`, `.rs`, `.dart`, `.swift`, `.kt`, `.java`, `.c`, `.cpp`, `.h`, `.hpp`, etc.).

### Exit Codes

| Code | Meaning |
|---|---|
| `0` | Allow. Optional stderr warning. |
| `2` | Block. stderr message shown to Claude as an error. |
| any other | Treated by Claude Code as block — avoid. |

---

## `hooks/stop.sh`

| Property | Value |
|---|---|
| Trigger | `Stop` |
| Matcher | (all) |
| stdin | Stop event JSON (mostly unused) |
| Exit | always `0` |
| LOC | ~40 |

### Smell Checks

| Smell | Warning |
|---|---|
| `IMPL:WORK` finished a turn that touched source but no test files | `"WORK turn changed source without touching tests. TDD?"` |
| `IMPL:SHIP` phase entered but no review artifact present | `"SHIP phase entered without a recorded review. Did REVIEW finish cleanly?"` |
| `.spec/.quick-plan.md` exists, mtime > 24h | `"`.spec/.quick-plan.md` is stale (>24h). Stale quick-task or forgotten cleanup?"` |
| Same `.phase` value for >10 consecutive turns | `"Phase has been <phase> for 10+ turns — stuck or done?"` |

### Turn Counter

The "10+ turns" check needs a counter. Implementation: `.spec/.phase-turns` (single integer, gitignored).

```bash
# In stop.sh:
COUNTER_FILE=".spec/.phase-turns"
if [ -f "$COUNTER_FILE" ]; then
  N=$(cat "$COUNTER_FILE")
  echo $((N + 1)) > "$COUNTER_FILE.tmp" && mv "$COUNTER_FILE.tmp" "$COUNTER_FILE"
else
  echo 1 > "$COUNTER_FILE"
fi
```

`bin/set-phase.sh` resets the counter to `0` whenever it changes `.phase`.

---

## settings.json (project copy)

```jsonc
{
  "hooks": {
    "SessionStart": [
      { "matcher": "startup",
        "hooks": [{ "type": "command", "command": "bash $PWD/hooks/session-start.sh" }] }
    ],
    "UserPromptSubmit": [
      { "hooks": [{ "type": "command", "command": "bash $PWD/hooks/user-prompt-submit.sh" }] }
    ],
    "PreToolUse": [
      { "matcher": "Edit|Write|NotebookEdit",
        "hooks": [{ "type": "command", "command": "bash $PWD/hooks/pre-tool-use.sh" }] }
    ],
    "Stop": [
      { "hooks": [{ "type": "command", "command": "bash $PWD/hooks/stop.sh" }] }
    ]
  },
  "permissions": {
    "allow": [
      "Read", "Glob", "Grep", "TodoWrite",
      "Skill(spec)",
      "Skill(compound-engineering)",
      "Skill(superpowers)",
      "Bash(bash bin/detect-context.sh:*)",
      "Bash(bash bin/set-phase.sh:*)",
      "Bash(bash bin/merge-feature.sh:*)"
    ]
  }
}
```

`install.sh` writes this verbatim to `<project>/.claude/settings.json`. If the file already exists, install prints a unified diff and asks the user to merge — never auto-overwrites.

---

## Performance Budget

| Hook | Target | Notes |
|---|---|---|
| SessionStart | ≤ 100ms | One-shot per session, can afford lessons.md head read |
| UserPromptSubmit | ≤ 30ms | Fires on every prompt — most user-visible |
| PreToolUse | ≤ 30ms | Fires on every Edit/Write — sensitive |
| Stop | ≤ 50ms | Fires every turn |

All four delegate to `detect-context.sh`. If the keystone hits its 50ms target, the hooks add <20ms of bash overhead and stay within budget.

---

## Open Questions

1. **`.spec/.phase-turns` lifecycle.** Resets via `set-phase.sh` (which has no awareness of stop.sh's counter). Is the implicit dependency okay, or should the counter live elsewhere? Default: keep the implicit coupling; document it in `set-phase.sh`'s comments.
2. **Phase-fight keyword list.** Shared with `user-prompt-submit.sh`. Centralize in `detect-context.sh` (which already does prompt-keyword routing) or duplicate? Default: centralize.
3. **PreToolUse Bash gate.** v1 doesn't match `Bash`. Adding it would catch `rm -rf .spec/` and friends. False-positive risk is high. Defer to v1.1 unless real misuse appears.
4. **Hook scope: per-project vs global.** `install.sh` defaults to per-project (`<project>/.claude/settings.json`). Global install (`~/.claude/settings.json`) is possible but riskier — it applies to every project. Default: per-project.
