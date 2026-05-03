---
type: feature-tech
feature: commands
sibling: product.md
parent: ../../tech.md
updated: 2026-05-03
---

# Feature: Commands — Architecture

How the three commands are implemented as markdown files that orchestrate skills via the routing JSON.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)

---

## File Layout

```
commands/
├── code-quick.md           # /code:quick <task>          ~50 LOC
├── code-strategy.md        # /code:strategy              ~80 LOC
├── code-feature.md         # /code:feature <name>        ~150 LOC
└── code-amend.md           # v1.1                        ~60 LOC

bin/
└── merge-feature.sh        # called from feature COMPOUND ~60 LOC
```

`install.sh` symlinks `commands/*.md` into `~/.claude/commands/code-*.md` so they're invocable as `/code:quick`, `/code:strategy`, `/code:feature`.

---

## Command File Structure

Each `commands/code-*.md` follows the same skeleton:

```markdown
---
description: <one-line>
allowed-tools: Read, Bash(bash $PWD/bin/...), Skill(...)
---

# /code:<name>

<prompt body — instructions to Claude>

## Phase: <name>
<bash invocations of detect-context.sh + set-phase.sh>
<delegation to skills via Skill() tool>
```

The body is mostly imperative prose: "Run `bin/detect-context.sh feature`. Parse the JSON. Verify the skills in `skills_to_load`. Set the phase to `feature:DESIGN:RESEARCH:<name>`. Delegate to `compound-engineering:ce-ideate`..." Claude reads it as instructions; the bash invocations are the only deterministic side effects.

---

## State Transitions

The three commands cause exactly the following `.spec/.phase` transitions, all via `bin/set-phase.sh` (direct edits are hard-blocked):

```
/code:quick:    no transition (stateless)

/code:strategy: ""  →  strategy:DESIGN:RESEARCH
                  →  strategy:DESIGN:DISCUSS
                  →  strategy:DESIGN:SPEC
                  →  strategy:DESIGN:PLAN  (optional)
                  →  ""

/code:feature:  ""  →  feature:DESIGN:RESEARCH:<name>
                  →  feature:DESIGN:DISCUSS:<name>
                  →  feature:DESIGN:SPEC:<name>
                  →  feature:DESIGN:PLAN:<name>
                  → [HUMAN GATE]
                  →  feature:IMPL:VERIFY:<name>
                  →  feature:IMPL:WORK:<name>
                  →  feature:IMPL:REVIEW:<name>
                  → [HUMAN GATE]
                  →  feature:IMPL:SHIP:<name>
                  →  feature:IMPL:COMPOUND:<name>
                  →  ""
```

Each command reads `.spec/.phase` on start; if non-neutral and matching the command's workflow, it resumes from that phase rather than restarting.

---

## Skill Delegation Tables

### `/code:strategy`

| Sub-phase | Default skill | Fallback |
|---|---|---|
| `DESIGN:RESEARCH` | `compound-engineering:ce-strategy` + `ce-ideate` (parallel) | built-in Explore agents |
| `DESIGN:DISCUSS` | `superpowers:brainstorming` | built-in `AskUserQuestion` loop |
| `DESIGN:SPEC` | `/spec` | hard-required, command errors if missing |
| `DESIGN:PLAN` | `compound-engineering:ce-plan` | built-in plan template |

### `/code:feature`

| Sub-phase | Default skill | Override | Fallback if missing |
|---|---|---|---|
| `DESIGN:RESEARCH` | `compound-engineering:ce-ideate` | — | built-in Explore agents |
| `DESIGN:DISCUSS` | `superpowers:brainstorming` | — | built-in `AskUserQuestion` loop |
| `DESIGN:SPEC` | `/spec` | — | hard-required |
| `DESIGN:PLAN` | `compound-engineering:ce-plan` + `ce-deepen-plan` | — | built-in plan template |
| `IMPL:VERIFY` | built-in (drift check) | — | — |
| `IMPL:WORK` | `superpowers:test-driven-development` + `:subagent-driven-development` | "ralph it" → `ce-work` | direct implementation |
| `IMPL:REVIEW` | `compound-engineering:ce-code-review` | — | superpowers review (if available) |
| `IMPL:SHIP` | `compound-engineering:ce-commit-push-pr` | — | direct git commands |
| `IMPL:COMPOUND` | `compound-engineering:ce-compound` + built-in merger | — | built-in merger only |

### `/code:quick`

| Condition | Skills loaded |
|---|---|
| trivial task (≤2 lines, no logic change) | none |
| non-trivial | `superpowers:test-driven-development` (always), `superpowers:brainstorming` (if "design"/"plan"/"how should" in prompt) |

---

## `bin/merge-feature.sh`

Called only during `feature:IMPL:COMPOUND`. Approximate flow:

```bash
1. Read .spec/features/<name>/tech.md
2. Identify cross-cutting sections:
     - Frontmatter `merge: true`
     - OR explicit `<!-- merge -->` ... `<!-- /merge -->` blocks
3. For each section, generate unified diff against .spec/tech.md
4. Show diff to user, ask: "Merge this into global tech.md? y/n/skip"
5. On yes: apply with `patch` or in-place sed
6. Repeat for product.md, design.md if marked
7. After all merges resolved:
     mv .spec/features/<name>/ .spec/archive/<name>/
8. Print summary: N sections merged, K skipped, archived to <path>
```

~60 LOC. Conflicts (overlapping section headers, ambiguous insertion points) are flagged for the user; never auto-resolved.

---

## Verification (`feature:IMPL:VERIFY`)

Built-in only; no external skill. Implements the drift-check from prior-art (see `archive/engineering-agent/insights.md` § VERIFY phase contract):

```
1. Read .spec/features/<name>/{product.md, tech.md}
2. Scan codebase for files referenced in spec:
     - All paths exist? (file-existence check)
     - Exported interfaces / signatures match spec? (grep-based)
     - Dependencies still in package.json / Cargo.toml / requirements.txt?
3. If clean → set-phase.sh feature:IMPL:WORK:<name>
4. If targeted drift → suggest /code:amend (v1.1) or print proposed amendment
5. If major drift → flag, suggest mini DESIGN re-entry
```

No test runs (per Q1 in archive/insights.md); WORK will run them anyway.

---

## Performance Contract

Commands are markdown prompts; they don't have a hard latency budget. The bash invocations they make do:

| Invocation | Target |
|---|---|
| `bin/detect-context.sh <workflow>` | ≤ 50ms |
| `bin/set-phase.sh <state>` | ≤ 20ms (validation + atomic write) |
| `bin/merge-feature.sh` | bounded by user diff review |

---

## Open Questions

1. **`/code:amend` mid-IMPL.** Currently v1.1. The amend flow needs a careful contract — should it be a sub-command of `/code:feature` (`/code:feature --amend <name>`) or its own command? Default: own command, simpler discoverability.
2. **Merge marker syntax.** `merge: true` in section frontmatter, or `<!-- merge -->` HTML comments, or both? Default: both, to keep the spec format flexible. Validator catches unknown markers.
3. **Skill missing during a command.** Today the command checks `skills_to_load` and degrades. Should it ask the user before degrading ("ce-code-review not available, use built-in checklist? y/n"), or do it silently with a stderr note? Default: silent + stderr note.
4. **Re-entry after abort.** If the user kills a session mid-`feature:IMPL:WORK`, re-running `/code:feature <name>` finds the saved phase. Should it resume or ask "resume / restart phase / abort"? Default: ask once at top of resume.
