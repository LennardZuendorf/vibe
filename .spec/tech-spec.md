---
type: tech-topic
parent: tech.md
scope: spec root, OpenSpec interchange, validation
covers: root discovery, migrate, OpenSpec grammar, strictness, adherence checks, trace, subagents, hooks, commands
updated: 2026-09-04
---

# Spec Framework — Tech

[tech.md](tech.md)

Branch doc for the spec tool: root discovery and migration, OpenSpec
interchange, the strictness ratchet, adherence checks, and the four
subagents. **Shipped today:** bash scripts (`validate.sh`, `list-specs.sh`,
`lessons-for.sh`, `scan-merges.sh`, `promote.sh`, `check-drift.sh`) run
cwd-relative or install-relative; see the note at the end.

## Root and discovery

Default root `docs/spec/`; `.spec/` is legacy and stays supported.
Discovery, first hit wins:

1. `--root` flag
2. `$VIBE_SPEC_ROOT`
3. `.vibe/spec.json` key `root`
4. `docs/spec/`
5. `.spec/`
6. repeat 4–5 per parent directory, up to the one holding `.git`
7. nothing found → exit 3 (no spec tree; see [tech-contracts.md](tech-contracts.md))

One function implements this order; every subcommand, hook, and peer call
goes through it. `vibe-spec root [--json]` is the one answer peers call —
if the binary is absent, a peer runs the same order via a vendored
resolution routine, so the order exists in one place in code, never
duplicated in prose.

## Migrate

`vibe-spec migrate --to docs/spec [--dry-run]`: `git mv` the tree; write
`root` into `.vibe/spec.json`; rewrite `.spec/` literals across an
explicit file set (the new root, `.vibe/*.json`, `.claude/**`,
`.agents/**`, `tests/**`, root `*.md`, template files), tagging exempted
lines `<!-- literal -->`; then a **population assertion** — literals
before ≥1, literals after ==0, files scanned ≥ a floor — or the migration
reverts and aborts rather than leaving a half-migrated tree. Inserts
`<!-- vibe-spec: allow SF-nn -->` markers for any newly-warn-eligible
check so nothing appears that the owner cannot act on today. This repo
migrates in one commit, late in the sequence (plan.md, F20).

## OpenSpec interchange

Grammar-compatible, layout-native: vibe adopts OpenSpec's requirement
grammar inside its own document model, not its directory layout.

Adopted verbatim: `### Requirement: <ID> — <name>` (SHALL/MUST),
`#### Scenario:` with bulleted GIVEN/WHEN/THEN, `## ADDED|MODIFIED|REMOVED
Requirements` deltas.

| vibe document | OpenSpec equivalent |
|---|---|
| root `product.md` (+ `product-<topic>.md`) | `openspec/specs/<capability>/spec.md`, one H2 per capability |
| feature `product.md` | `proposal.md` (`## Why`, `## What changes`) + delta specs |
| feature `plan.md` | `tasks.md` (`<name>/n` ↔ `- [ ] n.`) |
| feature `design.md` | `design.md` |
| `tech.md`, `research.md`, frontmatter, `<!-- merge -->`, Requirements Trace | exported as `## vibe:<field>` blocks appended to `design.md`; imported back |

`vibe-spec export --openspec <dir>` / `import --openspec <dir> [--feature
<name>]`: lossless for requirements, scenarios, deltas, and tasks;
documented-lossy for `updated:` dates, branch docs, lessons, and drift
state. Not adopted: the `openspec/` root, capability-per-folder layout,
`config.yaml`, schema profiles, the `/opsx:*` commands, archive
semantics — the split product/tech/plan model and lessons memory are why
vibe exists. `openspec validate` runs on the exported directory, never on
the tree itself.

## Strictness

`.vibe/spec.json` `strictness: {"SF-nn": "warn|error|off"}`, default
`warn` for every check; `--strict` (CI) promotes every non-`off` check to
`error`. A check moves to `error` in this repo's own config only in the
compound that migrates the last live spec still failing it — never
sooner. A `--strict` run that examined zero files is refused, not
reported green: absence of findings is only evidence when presence of
input is proven.

## Adherence checks

All warn-first, one SF-number each: layout allowlist
(`{product,tech,design,plan,lessons}.md`, `{product,tech,plan}-<topic>.md`,
`features/<name>/{product,tech,design,plan,research}.md`,
`archive/<name>/**`, `quick/<slug>.md`, `templates/**` — anything else
fails); required feature files present; frontmatter
`type/scope/children/updated` complete and enum-valid; every `children:`
path exists and every branch doc backlinks its parent; feature folder
name matches its plan row, a row with no folder is drift; `updated:`
older than the file's last commit; grammar (≥1 scenario per requirement,
WHEN and THEN present, SHALL/MUST word-bounded).

`vibe-spec trace [--since <ref>]`: cross-references `<name>/n` unit
citations from `git log` and `tests/**` against the plan, reporting
uncited units and orphan citations — a plan row nothing ever touched, or
a commit that cites a unit that does not exist.

## Subagents

Registered via the plugin manifest, real platform agents on both Claude
Code and OpenCode:

| Agent | Model | Tools | Writes |
|---|---|---|---|
| spec-interviewer | sonnet | Read, Grep, Glob, Write | `features/<name>/product.md` only |
| spec-tracer | sonnet | Read, Grep, Bash(`vibe-spec trace`, `git log`) | `features/<name>/plan.md` Requirements Trace |
| spec-promoter | sonnet | Read, Bash(`vibe-spec promote`) | root docs, only through the engine |
| spec-health | haiku | Read, Bash(`vibe-spec validate`, `check-drift`) | nothing — returns findings |

Each agent file ≤40 lines: frontmatter `name/description/tools/model`, an
explicit write path under `$(vibe-spec root)`, and a fallback rule —
binary missing, report and stop, never guess. OpenCode files under
`.opencode/agent/<name>.md` are generated by `vibe-spec setup --opencode`
from the same source, never hand-duplicated.

## Hooks

Shipped in the spec plugin, no dependency on flow: `Stop` runs `validate
--changed --format hook` (predicate: `git status --porcelain` under root,
mtime fallback), 2 s cap, exit 0 always; `PostToolUse` on
Edit/Write/MultiEdit under root runs `validate --file <path>`,
single-file checks only, 500 ms cap, exit 0. `vibe-spec setup
--git-hooks` installs a `pre-commit` running `validate --changed
--strict` — the only hook allowed to exit 1. OpenCode: the same
predicate and command behind a `tool.execute.after` shim.

## Commands (v1)

`root`, `init`, `validate [--changed|--file|--strict|--format text|json|hook]`,
`check-drift`, `promote [--dry-run]`, `scan-merges`, `lessons-for <tags>`,
`list-specs`, `trace`, `migrate`, `export|import --openspec`, `setup
[--git-hooks|--claude|--opencode]`, `doctor`.

## Shipped today

The bash scripts self-locate inconsistently: `validate.sh` and
`list-specs.sh` are cwd-only; `lessons-for.sh`/`scan-merges.sh` read
`${SPEC_DIR:-.spec}`; only `check-drift.sh` does upward marker search.
`spec/SKILL.md:319`, `spec/agents/spec-promoter/SKILL.md:16,19`,
`spec/agents/spec-interviewer/SKILL.md:15`, and `spec/feature.md:105`
hardcode `.agents/skills/spec/scripts/*.sh` — broken under a plugin
install (no `.agents/` tree) and from any cwd other than the repo root.
F1 (`spec-path-hotfix`, plan.md) fixes the four references to
skill-relative paths and adds self-location plus upward marker search to
every script, ahead of the Rust port.
