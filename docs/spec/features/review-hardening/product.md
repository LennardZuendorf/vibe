---
type: feature-product
feature: review-hardening
sibling: tech.md
parent: ../../product.md
updated: 2026-10-24
---

# Feature: Review Hardening — Product

Confirmed code-review findings in the shipped JS/bash harness share one theme. A
guard, installer step or writer claims a safety property and does not deliver it
on an input the author did not picture: a symlink, a path with a space, a
project below the git root, a one-line manifest, a missing temp directory, a
`MultiEdit` call. The
fixes were written once on the pre-v0.4 tree (tag `rh-on-54e5983`). This feature
ports the live ones onto the v0.4 tree, extends them to code that v0.4 added, and
fixes the docs and comments that overclaim. Users are people who install vibe
into a repo, run its hooks, or open an untrusted repo with vibe wired in.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | `install.sh`; `flow/engine/commands/hook.mjs`, `flow/engine/content.mjs`, `flow/engine/commands/render.mjs`, `flow/engine/commands/state.mjs`, `flow/engine/cursor.mjs`, `flow/engine/policy.mjs`; `flow/hooks-fallback/stop-gate.sh`; `flow/scripts/{merge-agents,regen-active-rules,set-state,detect-context}.sh` and the new `flow/scripts/lib/resolve-target.sh`; `build-plugin.sh` and the generated `plugin/hooks/session-start.sh`; `flow/scripts/merge-settings.sh` and this repo's `.claude/settings.json` (the flow guard matcher); `spec/hooks/run-hook.sh`; the matching tests (including the named blocks of `flow/tests/run.sh`), `CHANGELOG.md` bullets, `flow/setup.md`, and every comment the fixes make false |
| **Does not own** | Anything under `oracles/` (frozen, MANIFEST-pinned); the Rust ports of the stop gate (F6) and of content and render (F11); new install flags; new flow states or policy rules; the spec-hook wiring of `install.sh` targets and its `doctor` rows; de-duplicating the vibe-spec router at SessionStart; install-time writes through a symlinked skill dir; the doc text that states the guard matcher (skill-revision) |

---

## Requirements

Each requirement maps to one review finding. R1 to R10 keep the IDs of the
pre-v0.4 feature. R11 to R13 are new on v0.4.

### Requirement: Uninstall never deletes the vibe source or anything outside the target (R1)

The installer MUST refuse install and uninstall when the target is the vibe
source repo (compared by physical path) or another vibe checkout. Every uninstall
delete MUST resolve inside the physical target and outside the physical source,
and MUST NOT follow a symlinked directory out of the target. `--yes` MUST NOT
delete the cursor, receipts or evidence through a symlinked skill directory.

#### Scenario: uninstall inside the source repo

- **Given** a copy of the vibe repo
- **When** `install.sh <copy> --uninstall --yes` runs, with or without `--dry-run`
- **Then** it exits 1 with an uninstall message and `spec/` and `flow/` keep every file

#### Scenario: target is a symlink to the source

- **Given** `link -> <source copy>`
- **When** `install.sh link --uninstall --yes` runs
- **Then** it exits 1 and nothing is deleted

#### Scenario: skill directory symlinked outside the target

- **Given** a target whose `.agents/skills/spec` and `.agents/skills/vibe` link to an outside tree that holds same-named files, a `state.json` and an `evidence/` folder
- **When** `install.sh <target> --uninstall --yes` runs
- **Then** every outside file survives and the output carries a `WARN` that names the skipped link

#### Scenario: target links into a vendored vibe checkout

- **Given** a target holding `vendor/vibe` with `.claude -> vendor/vibe/.claude`
- **When** `vendor/vibe/install.sh <target> --uninstall --yes` runs
- **Then** every file under `vendor/vibe` survives and a warning names the vibe source

#### Scenario: control

- **Given** a fresh install into a plain target
- **When** `--uninstall --yes` runs
- **Then** it exits 0 and the shipped skill files are gone

### Requirement: The Stop receipt gate reads every changed path correctly (R2)

The gate MUST decode git's changed-path list without quoting loss, so a path with
a space, non-ASCII bytes, a newline or a rename is judged like any other path.
This applies to the engine gate and to the no-node bash fallback.

#### Scenario: awkward names go stale

- **Given** a `*.verify` cursor and a receipt older than a changed file named `my file.txt`, `café.txt`, `a\nb.txt`, or a staged rename to `moved file.txt`
- **When** the Stop hook runs, through the engine and through the fallback
- **Then** it blocks with `the evidence receipt is stale` and names the path

#### Scenario: control

- **Given** the same fixture with a plain name `file.txt`
- **When** the Stop hook runs
- **Then** it blocks the same way, and a receipt newer than every file passes

### Requirement: The gate works for a project below the git root (R3)

The gate MUST judge only the project directory's changes and MUST resolve them
relative to that directory, in the engine and in the bash fallback. Both MUST NOT
block on the project's own evidence, warnings log or inject marker. The engine
gate MUST also block when it cannot stat a changed path for any reason except
"not found". The bash fallback keeps its `-e` test.

#### Scenario: nested project

- **Given** the project is `<repo>/sub` and a file under `sub/` changed after the receipt
- **When** the Stop hook runs
- **Then** it blocks; a newer change outside `sub/` does not block; the project's own evidence, warnings log and inject marker never block

#### Scenario: unreadable path

- **Given** a changed path whose stat fails with `ELOOP`
- **When** the Stop hook runs through the engine
- **Then** it blocks and names the path and the error code

### Requirement: Content reads stay inside the repo, physically (R4)

Every config-named or walked content path MUST resolve through symlinks and stay
inside the repo. This covers `.vibe/blocks`, shipped blocks, `blocks.*.file`,
`sources.lessons` and the legacy lessons fallback. An in-repo symlink MUST stay
allowed. A refused read MUST surface as an error that `render --check` reports.
The engine's safety comment MUST state exactly what is guaranteed.

#### Scenario: symlink escapes

- **Given** `.vibe/blocks/x.md`, a symlinked `.vibe/blocks/dir`, a `blocks.*.file` or a `sources.lessons` that resolves outside the repo
- **When** content loads on a hook turn, or `render --check` runs
- **Then** the outside text never reaches a prompt, and `render --check` exits non-zero and names the path

#### Scenario: legacy lessons fallback escapes

- **Given** no `docs/spec/lessons.md` and a `.spec/lessons.md` symlink to an outside file <!-- literal -->
- **When** a `{{lessons:TAG}}` placeholder renders
- **Then** no outside heading is injected and the render errors name the path

#### Scenario: allowed cases

- **Given** an in-repo symlink, or a repo reached through a symlinked temp directory (macOS `/tmp`)
- **When** content loads
- **Then** the content is read with no error

### Requirement: Adapter linking never replaces a user symlink (R5)

`merge-agents.sh link` MUST refuse, exit 1, and leave a symlink that points
elsewhere untouched. There MUST be no force option. The installer MUST print a
warning that names the symlink's target and MUST continue.

#### Scenario: foreign symlink

- **Given** `CLAUDE.md -> docs/other.md`
- **When** `merge-agents.sh link CLAUDE.md <target>` or `install.sh <target> --adapters claude` runs
- **Then** the link is unchanged, the script exits non-zero, and the install output names `docs/other.md`

### Requirement: Managed-block writes keep a symlinked document a symlink (R6)

Every managed write MUST write through a symlink that resolves inside the repo
and MUST refuse with a warning and a non-zero exit when the link dangles or
resolves outside. The temp file MUST sit beside the resolved file, with an
unpredictable name created exclusively.

#### Scenario: write-through

- **Given** `AGENTS.md` is a symlink to `docs/AGENTS.real.md`
- **When** `merge-agents.sh`, `regen-active-rules.sh` or `cli.mjs render agents-md --write` runs
- **Then** the link survives and the real file carries the update

#### Scenario: refusal

- **Given** the link dangles, or points outside the repo, or a planted symlink sits at the old temp-file name
- **When** the same commands run
- **Then** nothing outside is written, a warning names the link, and the exit code is non-zero

### Requirement: The cursor writer emits only valid state (R7)

`set-state.sh` MUST validate the state name against a strict charset before any
lookup. It MUST reject a feature name that contains a control character, `"` or
`\`, with and without `jq`, and MUST drop an invalid carried feature with a
warning. The engine writer (`vibe state set`) MUST apply the same rule. Both MUST
treat an empty feature argument as no feature argument.

#### Scenario: hostile input

- **Given** state `idle|x`, or a feature `a"b`, or a feature that holds a newline
- **When** `set-state.sh` runs with and without `jq`
- **Then** it exits 1, names the bad argument, and the cursor stays untouched and valid JSON

### Requirement: Plugin retirement reads the right manifest key (R8)

The jq-less retirement path MUST treat a manifest as vibe's only when
`"name":"vibe"` is its first key. Otherwise it MUST keep the manifest and
`.claude/hooks/hooks.json`.

#### Scenario: one-line manifests

- **Given** `{"name":"other","author":{"name":"vibe"}}` and `{"name":"vibe","author":{"name":"x"}}`, each on one line, and no `jq`
- **When** install runs
- **Then** the first manifest and `.claude/hooks/hooks.json` are kept, and the second is retired

### Requirement: The no-node policy fallback decides like the engine (R9)

`detect-context.sh decide` without node MUST normalise a path as the engine does
before it matches, including a trailing newline.

#### Scenario: spelling aliases

- **Given** `docs/spec//lessons.md`, `docs/spec/./plan.md`, `docs/spec/x/../lessons.md` and `./docs/spec/lessons.md` in `idle`
- **When** `decide` runs with and without node
- **Then** both answer `block:` the same way, and unrelated paths still `allow`

### Requirement: Doctrine is emitted once per session, and only where the flow is installed (R10)

The plugin SessionStart hook MUST stay silent in a repo without the flow
(no `.agents/skills/vibe/state-machine.json`), even when it has a spec root. It
MUST also stay silent when the repo's settings wire
`session-start-doctrine.sh` and the repo hook can answer: `node` is on PATH, the
engine's `cli.mjs` is present, and `hook session-start-doctrine` prints. It MUST
keep emitting in a flow repo that holds the script unwired, and in a wired repo
where the engine prints nothing.

#### Scenario: three repos

- **Given** a spec-only `docs/spec` repo, a repo-local install, and a flow repo that holds the script without settings wiring
- **When** the plugin hook runs in each
- **Then** it prints the doctrine only in the third

#### Scenario: wired install without node

- **Given** a repo-local install whose settings wire `session-start-doctrine.sh`, and a PATH without `node`
- **When** the plugin hook runs
- **Then** it prints the doctrine

### Requirement: Install continues past a refused AGENTS.md merge (R11)

The installer MUST NOT abort when `merge-agents.sh` refuses a write. It MUST
print a warning and run every later step.

#### Scenario: dangling AGENTS.md

- **Given** a target whose `AGENTS.md` is a symlink to `/nonexistent`
- **When** `install.sh <target>` runs
- **Then** the output carries `AGENTS.md not merged`, the hooks wiring and banner steps still run, and the exit code is 0

### Requirement: The spec hook shim never loses a guard block (R12)

`spec/hooks/run-hook.sh` MUST pass a genuine `blocked:` exit 2 from
`vibe-spec hook guard` through as exit 2 even when it cannot create a temp file.
It MUST leave no temp file behind.

#### Scenario: no temp directory

- **Given** `TMPDIR` names a directory that does not exist, and a fake binary that prints one `blocked: ...` line and exits 2
- **When** `run-hook.sh guard` runs
- **Then** it exits 2 and prints the `blocked:` line

#### Scenario: control

- **Given** the same fake binary with a working `TMPDIR`
- **When** `run-hook.sh guard` runs
- **Then** it exits 2, and a crashing binary still maps to exit 0

### Requirement: The flow guard sees every file tool (R13)

The flow write-invariant guard MUST receive `MultiEdit` calls as it receives
`Edit`, `Write` and `NotebookEdit`. A settings file that holds the older matcher
MUST be upgraded in place by the settings merge, and a second merge MUST change
nothing.

#### Scenario: MultiEdit to a guarded path

- **Given** an installed target at `idle`
- **When** a `MultiEdit` call targets `docs/spec/lessons.md`
- **Then** the guard blocks it, through the engine and through the no-node fallback

#### Scenario: control

- **Given** the same target
- **When** a `MultiEdit` call targets `docs/notes.md`
- **Then** the guard allows it

---

## Non-Goals

- No new flags (`--force`, `--allow-source`) on install or link.
- No change to the pinned top-level `src/` TDD-predicate quirk, and no `-z` for the fallback's TDD-predicate read (`stop-gate.sh:114-130`): it only asks whether anything under `src/` changed.
- No stat of rename origins, and no `-uall` in the bash fallback.
- No edit under `oracles/`.

## Open Questions

1. **Fix the live JS and bash gate now, or only in Rust (R2, R3).** See [plan.md](plan.md) Open Questions. Default: fix the live code now and pin each new case as an expected divergence against the frozen oracles.
2. **Feature-name charset (R7).** Default: reject C0 and DEL control characters, `"` and `\`; permit everything else.
3. **Invalid UTF-8 file names (R2).** Default: a changed path that cannot round-trip through UTF-8 and cannot be stat'ed blocks.
4. **Plugin doctrine in spec-only repos (R10).** See [plan.md](plan.md) Open Question 4. Default: silent without `.agents/skills/vibe/state-machine.json`.
