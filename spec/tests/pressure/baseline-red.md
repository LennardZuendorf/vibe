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

REFACTOR (commit b2aa4b1): the description now names "one-line fixes, typos and new flags", says "'Quick' and 'just fix it' still route here first", and says to invoke before reading files. Two Red Flags rows were added for the failing scenarios: one answers the quick-fix prompt itself, the other the RED words "small targeted change" and "plan mode only allows read-only actions" (wording aligned with The Rule in 792106e).

Run 2 (b2aa4b1, `claude-sonnet-5-5`):

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

Run 3 (b2aa4b1): identical, `pressure: 12/12 passed`.

Final (792106e, after the scoped review: harness logging fixes and aligned Red Flags rows), `claude-sonnet-5-5`, two consecutive runs: `pressure: 12/12 passed`, `pressure: 12/12 passed`.

Limits: the six scenarios also served to tune the description, so 12/12 shows these prompts route; it does not prove generalisation. Only `claude-sonnet-5-5` was tested.

## skill-revision/2

RED record for the reworked harness: the `vibe-spec init` sandbox (layered
`docs/spec` root), six should-trigger rows and six near-miss rows (accept
`none`). Skill under test: the fd6ce39 text (unchanged by this unit). Command:
`bash spec/tests/pressure/run.sh --sp present` and `--sp absent` (sonnet, run in
parallel), then `PRESSURE_MODEL=haiku … --sp both`. Superpowers 7.0.0.

Isolation (differs from the 2026-10-14 record, so the two are not directly
comparable): this machine has keychain auth only, so a local, uncommitted
`claude` shim dropped the empty `CLAUDE_CONFIG_DIR` and the dummy token and ran
`claude --setting-sources "" --no-session-persistence` instead. No user
settings, user plugins, user hooks or user CLAUDE.md loaded; the four
org-managed plugins still loaded, as before. A one-row smoke run confirmed the
listing (`spec-under-test:spec`, 15 `superpowers:*` skills) and the superpowers
SessionStart hook.

Sonnet (`claude-sonnet-5-5`), `pressure: 12/12 passed` per leg:

```
PASS flag present first-skill=spec
PASS prd present first-skill=spec
PASS refactor present first-skill=spec
PASS quick present first-skill=spec
PASS howbuild present first-skill=spec
PASS archnote present first-skill=spec
PASS nm-explain present first-skill=none (near-miss)
PASS nm-tests present first-skill=none (near-miss)
PASS nm-gitstatus present first-skill=none (near-miss)
PASS nm-lastcommit present first-skill=none (near-miss)
PASS nm-json present first-skill=none (near-miss)
PASS nm-review present first-skill=code-review (near-miss)
PASS flag absent first-skill=spec
PASS prd absent first-skill=spec
PASS refactor absent first-skill=spec
PASS quick absent first-skill=spec
PASS howbuild absent first-skill=spec
PASS archnote absent first-skill=spec
PASS nm-explain absent first-skill=none (near-miss)
PASS nm-tests absent first-skill=none (near-miss)
PASS nm-gitstatus absent first-skill=none (near-miss)
PASS nm-lastcommit absent first-skill=none (near-miss)
PASS nm-json absent first-skill=none (near-miss)
PASS nm-review absent first-skill=none (near-miss)
```

Haiku (`haiku`), `pressure: 23/24 passed`. Every near-miss passed
(`nm-review` → `code-review` in both legs). One should-trigger FAIL:

| scenario | sp | first-skill | agent's words (verbatim) |
|---|---|---|---|
| quick | absent | none | (Bash → Write of a plan file, then) "Once plan mode is off, I'll also invoke the `spec-under-test:spec` skill before editing. Its description says it applies even to one-line fixes." |

Near-miss result: no near-miss routed to `spec` or `vibe` on either model, so
unit 4 has no verbatim near-miss quote to turn into a Red Flags row.

## skill-revision/3

GREEN after the frontmatter change (description 498 characters with no angle
brackets and no literal tag, "how should X be built" in place of "how should we
build X"; dead frontmatter keys removed). Same harness, isolation and
superpowers 7.0.0 as `## skill-revision/2`. Sonnet (`claude-sonnet-5-5`), both
legs `pressure: 12/12 passed`: the six original should-trigger rows route to
`spec` in both `sp` legs (12/12), including `howbuild`, and no near-miss routes
to `spec` or `vibe` (`nm-review` present → `code-review`; every other near-miss
→ `none`). Selection evals: [../evals/RESULTS.md](../evals/RESULTS.md).
