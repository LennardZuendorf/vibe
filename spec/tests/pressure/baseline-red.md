# Pressure-test RED baseline

Run command: `bash spec/tests/pressure/run.sh --sp both`

RED ran on 2026-10-14 against the pre-rewrite skill (commit e7c496b), with HEAD's harness copied into a scratch worktree. Model: `claude-sonnet-5-5`. Superpowers 6.4.1, with its SessionStart hook active in `present` runs.

Isolation: there is no user CLAUDE.md, no user settings and no user plugins. Four org-managed plugins load from managed settings (`cc-plugin-sec-default`, `cc-plugin-agents-md`, `cc-plugin-telemetry`, `cc-plugin-plugin-authoring`). They cannot be disabled per session. A first skill taken from them counts as FAIL.

Result: `pressure: 5/12 passed`.

| scenario | sp | first-skill | agent's words (verbatim) |
|---|---|---|---|
| flag | present | none | "The spec files are unfilled templates, so there are no conventions to follow. The script is a single line, so I'm writing the plan directly." |
| flag | absent | none | "I'll start by reading the file directly, since it's a small targeted change." |
| refactor | absent | none | (no text; Bash → Bash → Write) |
| quick | present | none | (no text; Bash → Write) |
| quick | absent | none | "I'll start by looking at the file, since plan mode only allows read-only actions." |
| howbuild | absent | none | "The repo has nothing to build auth on yet, so a few decisions come first." (answered without routing; dispatched an Agent first) |
| archnote | absent | none | (no text; Bash → Bash → Write) |

Passing runs: prd present → `spec`; prd absent → `spec` (only after Bash → Bash, i.e. exploration first); refactor present → `superpowers:brainstorming`; howbuild present → `superpowers:brainstorming`; archnote present → `spec`.

## GREEN

Run 1 (rewritten skill, commit 839c748): `pressure: 9/12 passed`. FAIL: flag/absent, quick/present, quick/absent. The agent made no text before acting (Read/Bash → Write).

REFACTOR: the description now names "one-line fixes, typos and new flags", says "'Quick' and 'just fix it' still route here first", and says to invoke before reading files. Two Red Flags rows were added from the RED words above.

Run 2:

```
PASS flag present first-skill=spec
PASS flag absent first-skill=spec
PASS prd present first-skill=spec
PASS prd absent first-skill=spec
PASS refactor present first-skill=spec
PASS refactor absent first-skill=spec
PASS quick present first-skill=spec
PASS quick absent first-skill=spec
PASS howbuild present first-skill=spec
PASS howbuild absent first-skill=spec
PASS archnote present first-skill=spec
PASS archnote absent first-skill=spec
pressure: 12/12 passed
```

Run 3: identical, `pressure: 12/12 passed`.
