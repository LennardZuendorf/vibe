---
name: vibe-setup
description: |
  Install or repair the vibe workflow harness in a repo. Read-only audit
  (detect) then write/merge bootstrap (apply) — never clobbers existing content.
  Writes the constitution block, scaffolds .agents/flow + .spec, preflights
  required plugins. Trigger on: set up vibe, bootstrap the harness, repair
  the workflow, install, onboard a repo, or user says setup.
user-invocable: true
argument-hint: ""
allowed-tools: Read, Edit, Write, Bash
compatibility: Requires bash + jq. macOS and Linux.
metadata:
  author: lennarddib
  version: "1.0"
---

# vibe-setup — bootstrap & repair

Brings a fresh (or drifted) repo under the harness. Two states:
`setup.detect` (read-only audit) → `setup.apply` (write/merge) → `idle`.

## 1. Detect (read-only, caveman lite)

`bash .agents/flow/scripts/set-state.sh setup.detect`. Audit only — write nothing.
Report present vs missing:

- `.agents/flow/state-machine.json`, `state.example.json`, and the four scripts
  (`set-state.sh`, `validate-state.sh`, `detect-context.sh`, `regen-active-rules.sh`).
- `.agents/skills/vibe-*` shims and the bundled `spec` skill.
- Constitution block in `CLAUDE.md` / `AGENTS.md` (markers
  `<!-- vibe:constitution:start -->` / `:end`).
- `.spec/lessons.md` and the root specs.

**Plugin preflight** — verify required skills are installed; warn + list any
missing, never hard-fail:

- `spec` (bundled), `superpowers:*`, feature-dev subagents (`code-explorer`,
  `code-architect`, `code-reviewer`), `caveman` (optional → fall back to the
  1-line caveman definitions in the constitution if absent).

## 2. Apply (write/merge, caveman lite)

`bash .agents/flow/scripts/set-state.sh setup.apply`. Bootstrap without
clobbering:

1. **Constitution block.** Write/merge the managed block (see template below)
   into `CLAUDE.md` and `AGENTS.md` between
   `<!-- vibe:constitution:start -->` and `:end`. Content outside markers is
   user-owned — diff and ask on any divergence.
2. **Flow scaffold.** Ensure `.agents/flow/` files exist. Seed the cursor only if
   missing: `cp .agents/flow/state.example.json .agents/flow/state.json`. Add
   `.agents/flow/state.json` to `.gitignore`.
3. **Spec scaffold.** If `.spec/` is bare, run `bash .agents/skills/spec/scripts/setup.sh`.
   Ensure `.spec/lessons.md` exists with the `Tags:`/`Pinned-by:` format.
4. **Active rules.** Run `bash .agents/flow/scripts/regen-active-rules.sh` to seed
   the managed active-rules block.

Finish at `idle`. Report what was created, merged, skipped, and any missing
plugins.

## Constitution block template

```md
<!-- vibe:constitution:start -->
## vibe Constitution

Three flows exist — **strategy** (direction, no code), **feature** (build loop),
**quick** (small fixes). Plus **setup** and the **amend** modifier.

**Read state:** `.agents/flow/state.json` = `{flow, phase, feature, updated}`.
The compound `<flow>.<phase>` keys into `.agents/flow/state-machine.json`, which
holds each state's skill, delegates, caveman level, read/write surface, frozen
inject, and legal `next`.

**Transition:** only via `bash .agents/flow/scripts/set-state.sh <flow.phase>`.
Never edit `state.json` by hand. Transitions are agent-suggested — name the next
state and confirm before moving.

**Canonical paths:** durable specs in `.spec/**`; runtime state in
`.agents/flow/**`; workflow shims in `.agents/skills/vibe-*`. Delegated skills
(`spec`, `superpowers:*`, subagents) must be told the exact `.spec/` path to
write.

**Caveman density** (output compression only — never reasoning; code/paths/
commands byte-exact; security + irreversible-action confirmations always normal
prose): `lite` = full sentences, no filler (strategy/setup/design/compound/
amend); `full` = drop articles, fragments OK (impl/verify/quick.*); `ultra` =
arrows, one word where one does (compound receipts + subagent summaries only,
never triage).

**Follow the inject literally.** It names the one skill, the write surface, the
path, the caveman level, and the next legal state.

The active-rules block below is generated from `.spec/lessons.md` by
`regen-active-rules.sh`; to change it, edit `lessons.md` during compound.
<!-- vibe:constitution:end -->
```

## Rules

- Never overwrite user content; merge inside markers, diff + ask outside.
- Missing plugin = warn and degrade, never hard-fail.
- Caveman lite throughout.
