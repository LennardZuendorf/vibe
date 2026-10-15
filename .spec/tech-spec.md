---
type: tech-topic
parent: tech.md
scope: spec root, OpenSpec interchange, validation
covers: root discovery, migrate, OpenSpec grammar, strictness, adherence checks, trace, subagents, hooks, commands
updated: 2026-10-15
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

`docs/superpowers/` is a recognised artifact root (D33): `vibe-spec` reads
`specs/YYYY-MM-DD-<topic>-design.md` and `plans/YYYY-MM-DD-<topic>.md`,
binds them to a feature by topic slug, and deletes them only through
`promote <feature> --done`.

## Migrate

`vibe-spec migrate --to docs/spec [--dry-run]`: `git mv` the tree; write
`root` into `.vibe/spec.json`; rewrite `.spec/` literals across an
explicit file set (the new root, `.vibe/*.json`, `.claude/**`,
`.agents/**`, `tests/**`, root `*.md`, template files), tagging exempted
lines `<!-- literal -->`; then a **population assertion** — literals
before ≥1, literals after ==0, files scanned ≥ a floor — or the migration
reverts and aborts rather than leaving a half-migrated tree.
`oracles/` and `tests/parity/corpus/` hold `.spec/` literals by design
(frozen oracles and legacy-layout fixtures) and are excluded from the
rewrite and from the population count. Inserts
`<!-- vibe-spec: allow SF-nn -->` markers for any newly-warn-eligible
check so nothing appears that the owner cannot act on today. This repo
migrates in one commit, late in the sequence (plan.md, F20).

`--layout layered` runs after the move: `{tech,product,plan}-<topic>.md`
→ `<kind>/<topic>.md`, each D-record → `decisions/dNN-<slug>.md`, hubs
seeded from templates. Splitting oversized prose into kind notes is
agent-led via the spec skill's restructure mode; the gate is
`validate --strict` green.

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

`.vibe/spec.json` `strictness: {"SF-nn": "warn|error|off"}`; a legacy
check keeps its default (errors stay errors), every other check
defaults to `warn`; `--strict` (CI) promotes every non-`off` check to
`error`. A check moves to `error` in this repo's own config only in the
compound that migrates the last live spec still failing it — never
sooner. A `--strict` run that examined zero files is refused, not
reported green: absence of findings is only evidence when presence of
input is proven.

Shipped (F17): every finding names one check in `vibe-spec`'s registry —
SF3, SF4, SF8–SF16 and I1 from the oracle, SF20–SF23 for the families it
left unnumbered (frontmatter, naming, links, tree), SF24–SF36 added by
F17. Keys accept `SF-nn` or `SFnn`; an unknown id warns and is ignored.
A configured level replaces a finding's default (legacy errors stay
errors, everything else warns); `--strict` turns warn into error; an
allow marker `<!-- vibe-spec: allow SF-nn -->` silences that check for
its file before either applies.

## Adherence checks

All warn-first, one SF-number each: layout allowlist (root hubs + `README.md`/`AGENTS.md`/`lessons.md`,
kind folders `areas|flows|concepts|platform|tech|decisions|questions`,
`parts/<part>/` with the same kinds, `features/<name>/{product,tech,design,plan,research}.md`,
`archive/<name>/**`, `quick/<slug>.md`, `templates/**`, and
`docs/superpowers/{specs,plans}/*.md` — anything else fails); budgets
(hub ≤80, note ≤60); link resolution; unique names; typed frontmatter
with `code:` existence on areas and `applies:` on tech; every kind note
reachable from a hub; required feature files present; frontmatter
`type/scope/children/updated` complete and enum-valid; every `children:`
path exists and every branch doc backlinks its parent; feature folder
name matches its plan row, a row with no folder is drift; `updated:`
older than the file's last commit; grammar (≥1 scenario per requirement,
WHEN and THEN present, SHALL/MUST word-bounded).

Shipped (F17): SF24 requirement population, SF25 scenario WHEN/THEN,
SF26 frontmatter values, SF27 children and backlinks, SF28 feature
folder → plan row, SF29 freshness against git, and on layered roots
(a root hub declares `type: hub`) SF30 allowlist, SF31 budgets, SF32
names, SF33 typed notes (`code:`, `applies:`), SF34 hub reachability;
in both layouts SF35/SF36 check `docs/superpowers/` names and each
plan's `**Spec:**` path. `trace` reads units from fallback feature plans
only; superpowers plans carry no unit ids.
`trace` refuses on zero units (exit 1, nothing on stdout). A shallow clone skips commit citations (`history: none`, stderr note) unless `--since` names a reachable ref. Any word-bounded `<feature>/<n>` token counts as a citation, including path fragments under `tests/`.

`vibe-spec trace [--since <ref>]`: cross-references `<name>/n` unit
citations from `git log` and `tests/**` against the plan, reporting
uncited units and orphan citations — a plan row nothing ever touched, or
a commit that cites a unit that does not exist.

## Subagents

Registered via the plugin manifest, real platform agents on both Claude
Code and OpenCode:

| Agent | Model | Tools | Writes |
|---|---|---|---|
| spec-promoter | sonnet | Read, Edit, Bash(`vibe-spec promote`) | layered notes + lessons, only through the promote checklist |
| spec-health | haiku | Read, Bash(`vibe-spec validate`, `check-drift`) | nothing — returns findings |

`spec-interviewer` and `spec-tracer` are retired (D36): feature
interviews and plans run through `superpowers:brainstorming` and
`superpowers:writing-plans`.

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

Router and tooth (D47): `SessionStart` (`startup|clear|compact`, sync)
runs `vibe-spec hook session-start`, which prints the `using-spec`
router inside `<EXTREMELY_IMPORTANT>`. `PreToolUse` on
`Edit|Write|MultiEdit|NotebookEdit` runs `vibe-spec hook guard`: exit 2
when the target is under the spec root or `docs/superpowers/**` and the
session transcript holds no load of the matching skill; exit 0 with one
stderr line on unreadable transcript, missing binary, no spec root, or
malformed input; `.vibe/spec.json` `"tooth": "warn"` downgrades to warn.

## Commands (v1)

`root`, `init`, `validate [--changed|--file|--strict|--format text|json|hook]`,
`check-drift [--pre-merge]`, `promote <feature> [--dry-run|--done]`,
`scan-merges`, `lessons-for <tags>`, `list-specs`, `trace`,
`migrate --to docs/spec [--layout layered] [--dry-run]`,
`export|import --openspec`, `hook {session-start,guard,validate}`,
`setup [--git-hooks|--claude|--opencode]`, `doctor`.

## Shipped today

F1 (`spec-path-hotfix`, plan.md) is delivered. `validate.sh`,
`list-specs.sh`, `lessons-for.sh`, and `scan-merges.sh` walk up from the
cwd to the directory holding `.git`; a relative `SPEC_DIR` resolves
against the root found. `promote.sh`, `setup.sh`, and `check-drift.sh`
are unchanged (`check-drift.sh` already searched upward); run `promote.sh`
and `setup.sh` from the project root. The skill, the spec subagents, and
`feature.md` reference scripts by skill-relative path. F25
(`spec-superpowers-default`) is delivered: the pressure harness
(`spec/tests/pressure/run.sh`) passes 12/12 with and without superpowers,
against a 5/12 baseline for the pre-rewrite skill.
F16 adds the `vibe-spec` binary (dev build: `cargo build -p vibe-spec`).
Every command goes through one discovery function. It offers `root`,
byte-identical ports of `list-specs`, `lessons-for`, `scan-merges` and
`validate`, and the `lessons-for --json`, `plan` and `feature` queries.
`tests/parity/run.sh` proves the ports against the frozen oracles, over the
corpus and live on this repo. Skills, CI and flow keep calling the scripts
until the binary ships (F21–F22).
F17 adds `validate --strict`, `--format text|json` (`--json`), the check registry and strictness config, and `trace`. `--changed`, `--file` and `--format hook` arrive with F19's hooks. The fixes (R-ID prefix, RFC 2119 boundary, Scope scoping, zero-requirement short-circuit) land in Rust only; `spec/scripts/validate.sh` keeps the oracle's behaviour until the binary ships.
