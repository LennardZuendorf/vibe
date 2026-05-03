---
type: branch
parent: tech.md
scope: hooks
covers: hook events, matchers, stdin format, exit codes, hard blocks, stderr conventions
updated: 2026-05-03
---

# Hooks

Four hooks turn shards-code's policy into mild enforcement. Three emit warnings to stderr and never block. The fourth (`pre-tool-use.sh`) has exactly two structural blocks plus the `.phase` write block. Together they make the workflow visible and resistant to footguns without being adversarial.

**Parent:** [tech.md](tech.md) | **Sibling:** [tech-detect-context.md](tech-detect-context.md)

---

## 1. Hook Event Map

| Hook script | Trigger event | Matcher | Hard blocks? |
|---|---|---|---|
| `hooks/session-start.sh` | `SessionStart` | `startup` | no |
| `hooks/user-prompt-submit.sh` | `UserPromptSubmit` | (all) | no |
| `hooks/pre-tool-use.sh` | `PreToolUse` | `Edit\|Write\|NotebookEdit` | yes — exactly 3 conditions |
| `hooks/stop.sh` | `Stop` | (all) | no |

All four registered in `settings.json` at the project (and optionally user) level. `install.sh` writes the project copy at `<project>/.claude/settings.json`.

### Common bash conventions

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read the hook's JSON payload from stdin
INPUT=$(cat)

# Resolve detect-context.sh location, falling back across known paths
SCRIPT=""
for candidate in \
  "$PWD/bin/detect-context.sh" \
  "$HOME/.claude/shards-code/bin/detect-context.sh" \
  "$(git rev-parse --show-toplevel 2>/dev/null)/bin/detect-context.sh"; do
  [ -f "$candidate" ] && SCRIPT="$candidate" && break
done

# Graceful degradation: if the keystone is missing, do nothing
[ -z "$SCRIPT" ] && exit 0

CONTEXT=$(echo "$INPUT" | bash "$SCRIPT" none)
```

The `for candidate in ...` pattern comes from the prior `check-phase.sh` (see archive insights) — it makes hooks portable across machines without env-var configuration.

---

## 2. `hooks/session-start.sh`

**Goal:** Make the current state visible at session start. Pure visibility; never blocks.

**Matcher:** `startup` (only fires once per session).

**stdin payload:** the SessionStart event JSON. Mostly unused — the script reads files directly.

**What it prints (to stderr):**

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

When `workflow=none`:

```
--- shards-code ---
Phase:   (neutral)
Suggested: /code:strategy (no specs found) OR /code:quick (specs found, no active feature)
-------------------
```

**Reads:**
- `.spec/.phase`
- `.spec/lessons.md` (top 3 entries by `### ` heading)
- `bin/detect-context.sh` output

**Exit:** always `0`.

---

## 3. `hooks/user-prompt-submit.sh`

**Goal:** Nudge the user toward the right command before they go off-track. Stderr-only; never blocks.

**Matcher:** none (fires on every prompt).

**Skip conditions:**
- Prompt starts with `/` (already a command)
- Prompt is empty
- Active phase is already a non-neutral feature workflow (don't second-guess in-progress work)

**Routing keywords** (case-insensitive, hard-coded in `detect-context.sh`):

| Trigger | Suggestion |
|---|---|
| "fix", "tweak", "small change", "rename", "typo" | `/code:quick <task>` |
| "build", "implement", "feature", "add ... to" | `/code:feature <name>` |
| "plan", "architect", "strategy", "refocus", "what should we build" | `/code:strategy` |

**Phase-fight detection:** if the active phase is `*:DESIGN:*` and the prompt contains code-action keywords ("write the function", "implement", "fix the bug"), suggest `/code:amend` (v1.1) or breaking out via `bin/set-phase.sh`. Same in reverse for `*:IMPL:*` + design keywords.

**What it prints (to stderr):**

```
[shards-code] Looks like a small fix — consider /code:quick "<paraphrased task>"
```

or

```
[shards-code] You're in feature:DESIGN:SPEC:dark-mode but the prompt asks for implementation.
              Either finish the SPEC + PLAN, or run /code:amend to adjust scope.
```

**Exit:** always `0`.

---

## 4. `hooks/pre-tool-use.sh`

**Goal:** Phase-aware write gate. Three hard blocks, otherwise advisory.

**Matcher:** `Edit|Write|NotebookEdit`.

**stdin payload:** Claude Code's PreToolUse JSON, including `tool_name` and `tool_input.file_path`.

### Decision rules (in order)

```
let path = tool_input.file_path

# ----- HARD BLOCKS (exit 2 + stderr) -----

if path == ".spec/.phase":
    BLOCK("`.spec/.phase` is structurally privileged. Use `bin/set-phase.sh <state>` instead.")

if path matches ".spec/lessons.md":
    if workflow != "feature" or phase != "IMPL:COMPOUND":
        BLOCK("`lessons.md` may only be written during `feature:IMPL:COMPOUND`. Current: <phase>.")

if path matches one of [.spec/product.md, .spec/tech.md, .spec/plan.md, .spec/product-*.md, .spec/tech-*.md]:
    if workflow == "strategy" and phase startswith "DESIGN:SPEC":
        ALLOW
    elif workflow == "feature" and phase == "IMPL:COMPOUND":
        ALLOW
    else:
        BLOCK("Global specs may only be written during `strategy:DESIGN:SPEC` or `feature:IMPL:COMPOUND`. Current: <phase>.")

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

### Exit codes

| Code | Meaning |
|---|---|
| `0` | Allow. Optional stderr warning. |
| `2` | Block. stderr message shown to Claude as an error. |
| (other) | treated by Claude Code as block — avoid. |

### Source-code-like detection

For warnings only, the heuristic is: anything outside `.spec/`, outside `.git/`, and not matching `*.md` at the project root, OR matching a known source extension (`.ts`, `.tsx`, `.js`, `.jsx`, `.py`, `.go`, `.rs`, `.dart`, etc.). Heuristic is fine — it's a warning, not a block.

---

## 5. `hooks/stop.sh`

**Goal:** Catch turn-end smells. Stderr only.

**Matcher:** none (fires after every assistant turn).

**stdin payload:** Claude Code's Stop event JSON. Used minimally; the script mostly reads files.

### Checks

| Smell | Warning |
|---|---|
| `IMPL:WORK` finished a turn that touched source but no test files | `"WORK turn changed source without touching tests. TDD?"` |
| `IMPL:SHIP` phase but no review artifact present (e.g., no `review.json` or recent ce-code-review note) | `"SHIP phase entered without a recorded review. Did REVIEW finish cleanly?"` |
| `.spec/.quick-plan.md` exists and is older than 24 hours | `"`.spec/.quick-plan.md` is stale (>24h). Stale quick-task or forgotten cleanup?"` |
| Same `.phase` value for >N consecutive turns (N=10 default) | `"Phase has been `<phase>` for 10+ turns — stuck or done?"` |

The "same phase for N turns" check requires lightweight session-local counting. v1 stores it in `.spec/.phase-turns` (single integer, gitignored). Bumped on every Stop hook invocation, reset by `set-phase.sh`.

**Exit:** always `0`.

---

## 6. settings.json (project copy)

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

`install.sh` writes this verbatim to `<project>/.claude/settings.json`. If the file already exists, install.sh prints a unified diff and asks the user to merge manually — never auto-overwrites.

---

## 7. The Two Hard Blocks (Justification)

shards-code blocks exactly two file-write conditions plus one structural privilege. Each is here because the alternative is structurally bad in a way that warnings can't fix.

| Block | Why it's hard, not soft |
|---|---|
| `lessons.md` outside `feature:IMPL:COMPOUND` | Lessons are append-only, session-end records. Allowing mid-flight edits invites scratchpad pollution and undermines the lesson's role as durable wisdom. |
| Global specs outside `strategy:DESIGN:SPEC` or `feature:IMPL:COMPOUND` | Cross-cutting decisions only land in globals through the COMPOUND merge. Allowing ad-hoc edits during WORK turns globals into a feature scratchpad and defeats the global/feature spec separation. |
| Direct edits to `.spec/.phase` | The state file is structurally privileged; bypassing `set-phase.sh` skips validation and corrupts state. The block exists so commands can rely on `.phase` being well-formed. |

Anything else (writing source during DESIGN, writing globals as a typo, etc.) is a warning. The premise: warnings make the workflow legible, blocks should be reserved for invariants whose violation breaks downstream tools.

---

## 8. Performance Contract

| Hook | Target latency | Notes |
|---|---|---|
| SessionStart | ≤ 100ms | One-shot per session, can afford lessons.md head read |
| UserPromptSubmit | ≤ 30ms | Fires on every prompt — must be fast |
| PreToolUse | ≤ 30ms | Fires on every Edit/Write — most sensitive |
| Stop | ≤ 50ms | Fires every turn |

All four delegate to `detect-context.sh`. If the keystone hits its 50ms target, the hooks add <20ms of bash overhead and are well within budget.

---

## 9. Open Questions

1. **Stop-hook turn counter.** `.spec/.phase-turns` (gitignored integer) or session-scoped temp file? Filesystem keeps it simple but writes on every turn; tmpfile is cleaner but resets on session restart. Default: gitignored file, reset by set-phase.sh.
2. **Phase-fight keyword list.** When DESIGN-phase prompts contain code-action verbs, we suggest `/code:amend`. The keyword list is shared with UserPromptSubmit — do we centralize in one bash array, or duplicate? Default: centralize in `detect-context.sh`.
3. **Hook location.** Under `<project>/hooks/` (where `install.sh` puts them) or `~/.claude/hooks/shards-code/`? Per-project keeps the policy with the project; global avoids re-symlinking. Default: per-project, with the option to symlink globally for power users.
4. **PreToolUse on non-file tools.** Currently only matches `Edit|Write|NotebookEdit`. Should we also match `Bash` to catch `rm -rf .spec/` etc.? Default: no in v1 — too broad, too many false positives. Add if real misuse appears.
