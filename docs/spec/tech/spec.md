---
type: tech
applies: "**"
updated: 2026-10-19
---
<!-- vibe-spec: allow SF31 -->

# Spec Framework — Tech

[tech.md](../tech.md)

Branch doc for the spec tool: root discovery and migration, OpenSpec
interchange, the strictness ratchet, adherence checks, and the four
subagents. **Shipped today:** bash scripts (`validate.sh`, `list-specs.sh`,
`lessons-for.sh`, `scan-merges.sh`, `promote.sh`, `check-drift.sh`) run
cwd-relative or install-relative; see the note at the end.

## Root and discovery

Default root `docs/spec/`; `docs/spec/` is legacy and stays supported.
Discovery, first hit wins:

1. `--root` flag
2. `$VIBE_SPEC_ROOT`
3. `.vibe/spec.json` key `root`
4. `docs/spec/`
5. `docs/spec/`
6. repeat 4–5 per parent directory, up to the one holding `.git`
7. nothing found → exit 3 (no spec tree; see [tech-contracts.md](contracts.md))

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
`root` into `.vibe/spec.json`; rewrite `docs/spec/` literals across an
explicit file set (the new root, `.vibe/*.json`, `.claude/**`,
`.agents/**`, `tests/**`, root `*.md`, template files), tagging exempted
lines `<!-- literal -->`; then a **population assertion** — literals
before ≥1, literals after ==0, files scanned ≥ a floor — or the migration
reverts and aborts rather than leaving a half-migrated tree.
`oracles/` and `tests/parity/corpus/` hold `docs/spec/` literals by design
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

| vibe | OpenSpec |
|---|---|
| a root document with blocks under `## Requirements` (`product-auth.md`, `concepts/invoice.md`) | `openspec/specs/<path without .md>/spec.md`; `## Purpose` is generated |
| fallback feature `product.md` | `changes/<f>/proposal.md` + one delta spec for its target's capability (plain `## Requirements` blocks export as ADDED) |
| feature `plan.md` units | `changes/<f>/tasks.md` (`- [ ] 1.<n> <title>`, `[x]` when DONE) |
| the five feature files (`product\|tech\|design\|plan\|research.md`); any other file in the folder is not exported (stderr note `export: <f>/<file>: not exported`) | carried byte for byte in `changes/<f>/design.md` as `## vibe:<file> <len>` blocks after `<!-- vibe:export v1 -->` |

Requirements are keyed by their full header name, normalized as OpenSpec
does (closing `#` run dropped, trimmed, case-sensitive). `RENAMED` is not
supported: write REMOVED + ADDED. A REMOVED block carries a `**Reason**:`
line. `promote <f>` applies a fallback feature's delta sections to its
`parent:` document, or to `<root>/product.md` when there is no `parent:`
(REMOVED, then MODIFIED, then ADDED; all or nothing). `<!-- merge -->`
blocks in the feature's `tech.md` append to `<root>/tech.md`, or to
`--target <file>`.

`vibe-spec export --openspec <dir>` refuses an existing `<dir>/openspec/`
and a MODIFIED that drops a scenario its target still has. `import
--openspec <dir> [--feature <name>]` writes fallback feature folders only,
never root documents. It restores carried files byte for byte, but refuses
when the change's requirements were edited on the OpenSpec side. It refuses
`RENAMED` and changes that touch more than one capability. A change without
carried files is imported as a new feature. Round trip: export -> import is
byte-identical. `openspec validate --strict` runs on the exported directory
in CI (`tests/openspec/run.sh`), never on the tree itself.

**Lossy / not adopted.** The round trip is lossless for requirements,
scenarios, deltas and tasks. It is lossy for: `updated:` dates on a foreign
import (stamped with today's date), branch docs, lessons, drift state, feature
files that are not carried (a stderr note names each), and `proposal.md` or
`design.md` prose edited on the OpenSpec side (import prints `import: <id>:
<file> edits are not imported`). An edited `tasks.md` is refused instead:
edit `plan.md` in vibe. Not adopted: the `openspec/` root as the source of
truth, archive semantics, and `RENAMED`. `promote <f> --done` deletes the
bound superpowers artifacts permanently (a filesystem remove; untracked files
included).

**Safety.** Today's behaviour of `promote`, `export`, `import` and
`check-drift`:

- Every read that feeds a write fails closed: a read error other than "not
  found" exits 1 and writes nothing (`promote`, `export`, `import`), also
  under `--dry-run`.
- `promote` writes are all-or-nothing: originals are kept in memory and
  restored if a later write fails. `--dry-run` runs the same plan and
  refuses exactly like the real run.
- `promote <f> --done` refuses (deletes nothing) if `docs`,
  `docs/superpowers`, `docs/superpowers/plans` or `docs/superpowers/specs`
  is a symlink. It binds artifacts by exact slug.
- `export` refuses a destination inside the spec root, a symlinked `<dir>`
  and an existing `<dir>/openspec/`. On a mid-write failure it removes only
  what it created.
- `promote` refuses to write through a symlinked target (also under
  `--dry-run`) and keeps the original file mode. A merge target and a delta
  target that are the same file under two paths (compared by identity) get
  one write.
- `promote <f> --done` also refuses in a shallow clone (no trustworthy branch
  base) and when a bound artifact is a symlink. `check-drift` reports a
  symlink under `docs/superpowers` as an artifact finding.
- `import` never overwrites an existing `features/<id>/` and never writes
  root documents. It refuses any symlink under the source `specs/` and a
  symlinked `features/`. Rollback removes only what it created.
- `check-drift --pre-merge` fails closed on an unreadable
  `docs/superpowers` subtree.
- `--json`: exit 0 always prints one JSON object. `--json` without a spec
  root exits 3 with empty stdout.
- Deliberate divergences from the bash oracles: root discovery walks up from
  the cwd; a symlinked `docs/spec` root is used; read errors fail closed where
  the oracle sometimes proceeded.

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

Real platform agents on Claude Code and OpenCode. The sources live in
`spec/agents/`. `build-plugin.sh` maintains the symlink `plugin/agents ->
../spec/agents`. This repo registers them as `.claude/agents/<name>.md`
symlinks into `spec/agents/`. OpenCode files come from `vibe-spec setup
--opencode`.

| Agent | Model | Tools | Writes |
|---|---|---|---|
| spec-promoter | sonnet | Read, Write, Edit, Bash | only under `vibe-spec root`; runs `vibe-spec promote <feature> --dry-run`, then the real `promote` and `--done` only after an explicit yes |
| spec-health | haiku | Read, Bash | nothing: `Read-only: never write files.` |

`spec-interviewer` and `spec-tracer` are retired (D36): feature
interviews and plans run through `superpowers:brainstorming` and
`superpowers:writing-plans`.

Each agent file is at most 40 lines: frontmatter `name/description/tools/model`,
a fallback rule (`vibe-spec` missing: reply "vibe-spec binary missing" and
stop), and for the promoter the line `Write only under the directory printed
by \`vibe-spec root\`.`

## Hooks

Spec hooks need no flow. Hooks run through the shim `spec/hooks/run-hook.sh
<event>`. It resolves the binary from `$VIBE_SPEC_BIN`, then `vibe-spec` on
`PATH`, then `$CLAUDE_PROJECT_DIR/target/{release,debug}/vibe-spec`. With no
binary it prints one stderr line and exits 0. It passes exit codes 0 and 2
through. Any other code becomes a warning line and exit 0.

Wiring: `build-plugin.sh` writes `plugin/hooks/hooks.json`, and this repo
carries `.claude/settings.json`. Both wire the same four events with
`"timeout": 10`. The plugin manifest does not register hooks. Flow's guard and
the spec tooth are independent: both must allow.

| Event | Matcher | Command |
|---|---|---|
| SessionStart | `startup\|clear\|compact` | `hook session-start` |
| PreToolUse | `Edit\|Write\|MultiEdit\|NotebookEdit\|Bash` | `hook guard` |
| PostToolUse | `Edit\|Write\|MultiEdit` | `hook validate`: `validate --file`, 500 ms cap |
| Stop | none | `hook validate`: `validate --changed`, 2 s cap |

`validate --changed` keeps the findings of changed spec files from `git status
--porcelain=v1 -z --untracked-files=all`, falls back to mtime against
`.vibe/spec-session`, and keeps tree-level findings. `validate --file <path>`
keeps one file's findings. `--format hook` prints hook JSON. Validate hooks
always exit 0.

Router: `hook session-start` prints the `using-spec` skill inside
`<EXTREMELY_IMPORTANT>`, shaped for Claude, Cursor or generic hosts. It
touches `.vibe/spec-session` and never writes through a symlink.

Tooth (D47): `hook guard` exits 2 with one stderr line when the target is
under the spec root, is `.vibe/spec.json`, or is under
`docs/superpowers/specs/` or `docs/superpowers/plans/` (other paths under
`docs/superpowers/` are not guarded), and the transcript holds no load of the
matching skill. Spec root and `.vibe/spec.json` need `spec`; `specs/` needs
`superpowers:brainstorming` or `spec`; `plans/` needs
`superpowers:writing-plans` or `spec`. A load is a `Skill`
tool call or a `<command-name>` record, in the main transcript or the
subagent's own. Prose and tool results never count. It fails open (exit 0, one
stderr line) on malformed input, an unreadable transcript, no spec root or a
target outside the project. `.vibe/spec.json` `"tooth": "warn"` downgrades a
block to a warning. A Bash command that writes a target gets a warning, not a
block.

Setup: `vibe-spec setup [--git-hooks] [--opencode] [--agents-md] [--json]`
needs at least one flag and no spec root. Every file it writes carries a
marker. It refuses to replace a file without the marker, replaces its own
idempotently, and restores on failure. `--git-hooks` writes `pre-commit`
(`# vibe-spec pre-commit v1`) under `git rev-parse --git-path hooks`. The hook
runs `validate --changed --strict` with `VIBE_SPEC_STRICTNESS='{"SF29":"error"}'`
and is the only hook that exits 1. `VIBE_SPEC_STRICTNESS` is a JSON object in
the `strictness` schema, merged over the config. `--opencode` writes
`.opencode/agents/spec-promoter.md`, `.opencode/agents/spec-health.md` and
`.opencode/plugins/vibe-spec.js`, a `tool.execute.after` shim that runs `hook
validate`. `--agents-md` adds or refreshes the router block between
`<!-- vibe-spec:router:start -->` and `<!-- vibe-spec:router:end -->`.

Known limits:
- `install.sh` does not wire spec hooks into targets (P5). They reach targets through the plugin.
- The OpenCode shim has not run against live OpenCode.
- A hooks dir outside the project (a linked worktree or an external `core.hooksPath`) skips the symlink walk.

## Commands (v1)

`root`, `init`, `validate [--changed|--file|--strict|--format text|json|hook]`,
`check-drift [--pre-merge] [--json]`,
`promote <feature> [--dry-run] [--target <file>] [--done] [--base <ref>] [--json]`,
`scan-merges`, `lessons-for <tags>`, `list-specs`, `trace`,
`migrate --to docs/spec [--layout layered] [--dry-run]`,
`export|import --openspec <dir>`, `hook {session-start,guard,validate}`,
`setup [--git-hooks|--opencode|--agents-md]`, `doctor`.

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
F17 adds `validate --strict`, `--format text|json` (`--json`), the check registry and strictness config, and `trace`. `--changed`, `--file` and `--format hook` arrive with F19 (see Hooks). The fixes (R-ID prefix, RFC 2119 boundary, Scope scoping, zero-requirement short-circuit) land in Rust only; `spec/scripts/validate.sh` keeps the oracle's behaviour until the binary ships.
F18 adds `check-drift` (`--pre-merge`, `--json`), `promote` (deltas, the superpowers checklist, `--done`, `--json`), `export --openspec` and `import --openspec` to `vibe-spec`, with byte parity for the two ports over the corpus. Skills, CI and flow still call `spec/scripts/*.sh` until F21-F22 (P3c-R2). The OpenSpec gate runs in its own CI job (`tests/openspec/run.sh`, network).
F19 adds `hook {guard,validate,session-start}`, `validate --changed|--file|--format hook`, `setup`, the shim and wiring, and the `spec-promoter` and `spec-health` agents. See Subagents and Hooks.
