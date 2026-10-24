---
type: feature-plan
feature: review-hardening
sibling: tech.md
parent: ../../plan.md
updated: 2026-10-24
---

# Feature: Review Hardening — Implementation Plan

This plan ports the review-hardening fixes from tag `rh-on-54e5983` onto the
v0.4 tree and adds the v0.4-only items. Unit numbers keep the old IDs 1 to 14,
so each unit maps to its old diff. Unit 8 is dropped as moot. Units 2 and 16 are
folded into unit 1 (their tests keep their IDs). Units 15 to 19 are new. Step 0
puts the feature on the root plan. Tracks own disjoint files, except the shared
files named under Tracks.

> For agentic workers: use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Every unit is TDD: write the failing test first, run the stated command, and see the stated failure before the fix. Subagent models: implementers and per-unit reviewers `sonnet`; the whole-branch review in unit 13 `opus`. Set `model` on every dispatch.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** none. This is branch work on the live JS/bash harness, outside the F1–F25 binary gates. Step 0 adds the root row before any unit starts.

---

## Problem Frame

v0.4 rebuilt the spec half in Rust. It left `install.sh`, the flow engine, the
bash scripts and the fallbacks as they were, so ten of the twelve old findings
are still live. v0.4 also added code the old fixes never covered: manifest-driven
and source-walk uninstall deletes, the legacy lessons read, the spec hook shim,
a plugin doctrine that fires in every `docs/spec` repo, and a flow guard matcher
without `MultiEdit`. Most old hunks apply cleanly; `build-plugin.sh` and the
`lessonsFor` hunk need a hand-merge. The riskiest track is the gate (units 9 and
10), because the live fallback is still the parity oracle until unit 9 repoints it.

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | [Uninstall never deletes the source or outside the target](product.md#requirement-uninstall-never-deletes-the-vibe-source-or-anything-outside-the-target-r1) | review-hardening/1, review-hardening/15 |
| R2 | [Gate reads every changed path](product.md#requirement-the-stop-receipt-gate-reads-every-changed-path-correctly-r2) | review-hardening/9, review-hardening/10 |
| R3 | [Gate works below the git root](product.md#requirement-the-gate-works-for-a-project-below-the-git-root-r3) | review-hardening/9, review-hardening/10 |
| R4 | [Content reads stay inside the repo](product.md#requirement-content-reads-stay-inside-the-repo-physically-r4) | review-hardening/11, review-hardening/12 |
| R5 | [Adapter linking never replaces a symlink](product.md#requirement-adapter-linking-never-replaces-a-user-symlink-r5) | review-hardening/3 |
| R6 | [Writes keep a symlinked document a symlink](product.md#requirement-managed-block-writes-keep-a-symlinked-document-a-symlink-r6) | review-hardening/4, review-hardening/12, review-hardening/18 |
| R7 | [Cursor writer emits only valid state](product.md#requirement-the-cursor-writer-emits-only-valid-state-r7) | review-hardening/5 |
| R8 | [Plugin retirement reads the right key](product.md#requirement-plugin-retirement-reads-the-right-manifest-key-r8) | review-hardening/1 (section review-hardening/2) |
| R9 | [No-node policy fallback matches the engine](product.md#requirement-the-no-node-policy-fallback-decides-like-the-engine-r9) | review-hardening/6 |
| R10 | [Doctrine emitted once, only where the flow is installed](product.md#requirement-doctrine-is-emitted-once-per-session-and-only-where-the-flow-is-installed-r10) | review-hardening/7 |
| R11 | [Install continues past a refused merge](product.md#requirement-install-continues-past-a-refused-agentsmd-merge-r11) | review-hardening/1 (section review-hardening/16), review-hardening/4 |
| R12 | [Spec hook shim never loses a block](product.md#requirement-the-spec-hook-shim-never-loses-a-guard-block-r12) | review-hardening/17 |
| R13 | [Flow guard sees every file tool](product.md#requirement-the-flow-guard-sees-every-file-tool-r13) | review-hardening/19 |

---

## Key Technical Decisions

1. **Oracles stay frozen.** No edit under `oracles/`. Unit 9 repoints `GATE_ORACLE` at the existing frozen copy. `GUARD_ORACLE` is not repointed, because its frozen copy differs from the live guard.
2. **Fix the live JS and bash now, pin divergences** (default of Open Question 1). The new cases carry forward into the F6 and F11 corpora.
3. **`predicateTdd` stays byte-compatible.** `gitPorcelain` rebuilds the legacy `text` from decoded entries. The TDD predicate's plain-porcelain read in `stop-gate.sh` (:114-130) stays as it is (ruling in [tech.md](tech.md) R2/R3).
4. **`confineRealWhy` is the one capability gate.** `confinePath` stays lexical.
5. **All `install.sh` hunks belong to track A**, so tracks D and E never touch it. The whole old `install.sh` diff lands once, in unit 1, after the tests of units 1, 2 and 16 are RED.
6. **Hunk subsets without `filterdiff`** (not installed here): split a diff on `^@@` with awk and apply chosen hunks (Global Constraints). Used only for `merge-agents.sh` (unit 4 versus unit 3).

---

## Global Constraints

- Bash: `set -euo pipefail` (the hook shims keep their `-uo`), shellcheck-clean, bash 3.2-safe (no `declare -A`, `readlink -f`, `mapfile`, `${var,,}`).
- Node 18+, ESM, zero dependencies. `content.mjs` may not spell an upward path segment.
- Never edit `oracles/**`. Never hand-edit `plugin/hooks/session-start.sh`. Never edit `docs/spec/lessons.md` or root specs outside step 0 and compound (unit 14).
- Literal sweep (`bash tests/literals/run.sh`, `migrate --check`): a new line that spells the legacy root needs a trailing `# literal` or `// literal` tag, unless its file carries `vibe-spec: literal-file`. `adapters/run.sh`, `hook.test.mjs`, `parity.test.mjs` and `policy.test.mjs` carry the marker. `render.test.mjs`, `state.test.mjs`, `detect-context.sh` and `flow/tests/run.sh` do not.
- Repo copies in tests use `tar -cf - --exclude=.git --exclude=./target -C "$REPO_ROOT" .`. A local cargo `target/` can be large. Use a fresh copy per destructive case.
- Git fixtures (lesson "Sandbox git repos set `commit.gpgsign false`"): every sandbox `git init` sets `user.email`, `user.name`, `commit.gpgsign false` and `tag.gpgsign false` in that repo, and checks every setup commit's exit status. A nested-project fixture runs `git init` in the outer directory and sets `GIT_CEILING_DIRECTORIES` to the sandbox's parent, so no enclosing repo leaks in.
- Shell changes (lesson "A new shell script runs the umbrella suite"): a unit that adds or edits a tracked `*.sh` runs, before its commit, `bash tests/run.sh` and the CI command `git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck`. Run `git add -N <new file>` first, so `git ls-files` sees a new script.
- Hunk split (no `filterdiff`): `git diff 5ccb99b rh-on-54e5983 -- <path> > "$S/d.patch"`, then `awk -v d="$S" '/^@@/{n++} {f = n ? sprintf("%s/h%02d", d, n) : d "/head"; print > f}' "$S/d.patch"`, then `cat "$S/head" "$S/h01" … | patch -p1 -F3 --dry-run` before the real apply. `$S` is a scratch directory outside the repo.
- Each unit adds one `### Fixed` bullet under `## [Unreleased]` in `CHANGELOG.md`, and fixes every comment or doc it makes false.
- Test IDs: bash sections use `pass`/`fail` id `review-hardening/N` under a `# review-hardening/N` header. JS test names start with `review-hardening/N:`.
- Commands: engine `node flow/engine/tests/run.mjs "<filter>"`; adapters `bash flow/tests/adapters/run.sh 2>&1 | grep -E "FAIL|review-hardening/N"`; flow `bash flow/tests/run.sh`; spec hooks `bash spec/tests/hooks.sh`; plugin `bash spec/tests/plugin-validate.sh` (skips without `claude`).
- Commits only on explicit request: `fix(<area>): rh/N <subject>`, ≤50 characters, no body. The full ID `review-hardening/N` lives in test names.

---

## Tracks and merge order

| Track | Units (in order) | Files owned |
|---|---|---|
| 0 setup | step 0 | `docs/spec/plan.md` (the `review-hardening` row), `docs/spec/features/review-hardening/**` |
| A installer | 1 (with 2, 16) → 15 | `install.sh` |
| B gate | 9 → 10 | `hook.mjs`, `hook.test.mjs`, `stop-gate.sh`, `parity.test.mjs` |
| C content | 11 → 12 | `content.mjs`, `render.mjs`, `content.test.mjs`, `render.test.mjs`, `primitives.test.mjs` |
| D writers | 4 → 3 | `merge-agents.sh`, `regen-active-rules.sh`, `lib/resolve-target.sh`, `flow/setup.md` |
| E scripts | 5, 6, 7, 19 (independent) | `set-state.sh`, `state.mjs`, `cursor.mjs`, `state.test.mjs`; `detect-context.sh`, `policy.mjs`, `policy.test.mjs`; `build-plugin.sh`, `plugin/hooks/session-start.sh`; `merge-settings.sh`, `.claude/settings.json` |
| F spec hook | 17, 18 (optional) | `spec/hooks/run-hook.sh`, `spec/tests/hooks.sh`; `crates/vibe-spec/tests/setup.rs` |

Shared files (each unit adds its own section; on conflict keep all sections, all bullets, both sides):

| File | Writers | Rule |
|---|---|---|
| `flow/tests/adapters/run.sh` | A, D, units 5, 7, 19 | own `# review-hardening/N` section right before the `=== results` footer; unit 19 also edits the matcher pin at :300 |
| `CHANGELOG.md` | every unit; skill-revision | own bullet; never touch the `### skill-revision` subsection |
| `flow/tests/run.sh` | unit 4 (regen sandboxes :690, :759, :1875), unit 7 (`spec-v04/p4` pin :1255-1263), unit 19 (matcher block :1270-1290); skill-revision Track D (units 18-24) | every port unit that edits it lands on `villain/vibe` before skill-revision Track D merges; Track D rebases onto it and keeps both sides |
| `flow/README.md` | unit 4 (the "temp + rename / never clobber" line); skill-revision unit 27 | keep both sides |
| `AGENTS.md` | unit 14 (`regen-active-rules.sh` rewrites the `vibe:active-rules` block); skill-revision units 25-27 and 31 | the block is generated: on conflict take either side, then re-run `bash .agents/skills/vibe/scripts/regen-active-rules.sh` |
| `docs/spec/plan.md` | step 0, unit 14; skill-revision units 1 and 31 | the first writer creates `## Branch work`, the other appends its row; keep both rows on conflict |

- Merge order: **0, A, D, E, C, B, F**. D needs A first: unit 4 makes `merge-agents.sh` exit 1 on a refused write, and only the R11 hunk that unit 1 applies stops that from aborting the install. B goes last because it has the most parity churn.
- Each track runs its own suites, `bash tests/run.sh` and the CI shellcheck command green before it merges. Unit 13 runs everything once after all merges.

---

## Unit IDs

Units are `review-hardening/n`, assigned once and never renumbered. Unit 8 is
dropped: it is moot upstream (see [tech.md](tech.md) Dropped), and its one
remaining step is the first step of unit 9. Units 2 and 16 are folded into
unit 1: their code is part of the one `install.sh` apply, and their test
sections keep the IDs `review-hardening/2` and `review-hardening/16`.

---

### Step 0 — Root row

**Goal:** the root plan names this feature before any unit starts, so `check-drift` and validate SF28 pass while the folder exists.

**Files:** `docs/spec/plan.md` (one row; bump `updated:`), `docs/spec/features/review-hardening/{product,tech,plan}.md`.

**Steps:**

- [ ] The orchestrator creates a side worktree: `git worktree add -b rh/setup "$SCRATCH/wt/rh-setup" villain/vibe`. Never move the main worktree's cursor for this step. Every later command runs in that worktree.
- [ ] `bash flow/scripts/set-state.sh strategy.brainstorm`, then `bash flow/scripts/set-state.sh strategy.spec`. Check `bash flow/scripts/detect-context.sh decide docs/spec/plan.md` prints `allow`. Invoke `Skill(spec)`.
- [ ] `grep -n '^## Branch work' docs/spec/plan.md`. No hit: append the section with this header and row. A hit (skill-revision landed first): append the row only.

  ```
  ## Branch work

  | Feature | Scope | Status |
  |---|---|---|
  | review-hardening | Port of the pre-v0.4 review-hardening fixes onto the live JS/bash harness | IN PROGRESS |
  ```

  `IN PROGRESS` is the token `check-drift` reads (`drift.rs` `row_status`). Bump `updated:`. Copy the three spec files into `docs/spec/features/review-hardening/`.
- [ ] `cargo run -q --locked -p vibe-spec -- validate --strict` (expected: no errors; `plan.md` stays under the 80-line hub budget) and `-- check-drift` (expected: a note that `review-hardening` is IN PROGRESS, no error).
- [ ] `bash flow/scripts/set-state.sh idle`. Commit `docs(spec): add review-hardening row`. Merge into `villain/vibe`; remove the worktree.

**Verification:** validate and check-drift output pasted into the merge request.

---

### review-hardening/1 — Installer port: uninstall guard, symlink-safe removal, retire key, merge warning

**Goal:** refuse install and uninstall into a vibe checkout; never delete through a symlink or into the source; classify a jq-less manifest by its first key (R8, section `review-hardening/2`); continue past a refused AGENTS.md merge (R11, section `review-hardening/16`).

**Requirements:** R1, R8, R11

**Dependencies:** step 0

**Files:**

```
install.sh                    # the whole old diff, all 11 hunks (map below); --yes and last-inject wraps
flow/tests/adapters/run.sh    # sections review-hardening/1, review-hardening/2, review-hardening/16
```

Hunk map of `git diff 5ccb99b rh-on-54e5983 -- install.sh` (traceability; applied together):

| Hunk | Content | Test section |
|---|---|---|
| `@@ -45` | physical `SRC` | 1 |
| `@@ -101` | jq-less first-key classify; `inside_target` around the two retire `rm` calls (:129) | 2 (and 1 for the rm) |
| `@@ -139` | `inside_target` | 1 |
| `@@ -156` | `remove_shipped` physical checks | 1 |
| `@@ -247` | `unregister_skill` | 1 |
| `@@ -451`, `@@ -460`, `@@ -527`, `@@ -562` | physical `TARGET`, guard, CLAUDE_OURS gate, uninstall branches | 1 |
| `@@ -788` | merge failure is a WARN | 16 |
| `@@ -830` | link failure names the held link | 16 (asserted at install level in unit 3) |

**Interfaces:**

- Consumes: —
- Produces: physical `SRC`/`TARGET`; `inside_target PATH`; `remove_shipped SRC DST [EXCLUDE...]` keeps its signature; the install contract "merge exit 1 is a warning", which unit 4 relies on.

**Steps:**

- [ ] Write the tests of all three sections first.
  - `# review-hardening/1`: port the tag's section with `rh_copy` (add `--exclude=./target`) and `rh_count`. Cases: `install.sh "$SRCA" --uninstall --yes`, the same with `--dry-run`, and a symlink to the copy: each exits 1, the output contains `uninstall`, and `find $SRCA/spec $SRCA/flow -type f | wc -l` is unchanged. Outside sentinel: `T/.agents/skills/spec -> $OUT` and `T/.agents/skills/vibe -> $OUT2`, where `$OUT2` holds a `state.json` and `evidence/x.md`. Run `--uninstall --yes --only spec`, then again with `--only flow`: every file in `$OUT` and `$OUT2` survives, and the output contains `WARN`. Vendored case: `$RV/vendor/vibe` from `rh_copy`, with `.claude -> vendor/vibe/.claude`: every file under `vendor/vibe` survives. Overlap case: uninstall into another vibe checkout exits 1. **Control:** fresh install into `$T2`, then `--uninstall --yes`: exit 0, `$T2/.agents/skills/spec/SKILL.md` gone.
  - `# review-hardening/2` (R8), under `PATH="$(mkshim jq)"`, using the fixture pattern of the legacy-manifest retirement tests (platform-adapters section, near run.sh :330), each with a `.claude/hooks/hooks.json` sentinel: `{"name":"other","author":{"name":"vibe"}}` keeps both files; `{"name":"vibe","author":{"name":"x"}}` removes both; a vibe manifest whose first key is not `name` is kept. **Controls:** a pretty-printed vibe manifest is removed; the jq-present variants of the first two give the same verdicts.
  - `# review-hardening/16` (R11): a target whose `AGENTS.md` has reversed markers (`<!-- vibe:instructions:end -->` on line 1, `<!-- vibe:instructions:start -->` on line 2). Today's `merge-agents.sh` already exits 1 on it (`die`, merge-agents.sh :76), so the case does not depend on unit 4. `install.sh <target>` exits 0, the output contains `AGENTS.md not merged`, and the hooks-wiring and banner lines are present. **Control:** a target with a real-file `CLAUDE.md` and `--adapters claude` still prints a `not linked` warning.
- [ ] Run `bash flow/tests/adapters/run.sh 2>&1 | grep -E "FAIL.*review-hardening/(1|2|16)"`. Expected FAIL: section 1 — exit 0 not 1, the spec file count drops, outside, `$OUT2/state.json` and vendored files are deleted; section 2 — the first two one-line manifests get the wrong verdict; section 16 — install exits non-zero at step 6 (`set -e`) and the later lines are missing. Every control passes.
- [ ] Implement: `git diff 5ccb99b rh-on-54e5983 -- install.sh | patch -p1 -F3 --dry-run` (fuzz expected, all 11 hunks apply), then the same without `--dry-run`. Check `head -c 20 plugin/.claude-plugin/plugin.json` still shows `"name"` first. Then wrap the `--yes` deletes (:1016-1019) in `[[ ! -L "$TARGET/.agents/skills/vibe" ]] && inside_target "$TARGET/.agents/skills/vibe/state.json"`, and wrap the last-inject rm and rmdir (:1023-1024) in `inside_target "$TARGET/.vibe/last-inject"`.
- [ ] Run the adapters suite. Expected: no `FAIL`. Run `bash tests/run.sh` and the CI shellcheck command. Expected: all suites PASS, no shellcheck output.
- [ ] Docs: one CHANGELOG bullet each for R1, R8, R11. Rewrite the guard comment ("Global mode is fine from here") to cover uninstall.

**Test scenarios:**

- Source copy, symlink to source, `--dry-run`, another checkout: exit 1, nothing deleted.
- Symlinked skill dirs and a vendored `.claude` are left alone with a warning, and so are the outside cursor and evidence under `--yes`.
- A nested `name` never wins; a vibe manifest that is not first-keyed is kept.
- A merge refusal is a warning; later install steps still run.

**Verification:** adapters suite green; the spec file count of the copy is unchanged after the reproduction.

---

### review-hardening/2 — Folded into unit 1

The jq-less retirement fix is hunk `@@ -101` of the one `install.sh` apply. Its
tests are section `review-hardening/2`, written in unit 1's RED step. No separate
work remains.

---

### review-hardening/16 — Folded into unit 1

The merge-warning and held-link hunks (`@@ -788`, `@@ -830`) land in unit 1. The
reversed-marker case is section `review-hardening/16`, written in unit 1's RED
step. The dangling-`AGENTS.md` install case belongs to unit 4, and the held-link
install case to unit 3, because each fails first only before that unit's code.

---

### review-hardening/15 — Every uninstall delete stays inside the target

**Goal:** the uninstall deletes that v0.4 added, and the ones the old diff never saw, cannot delete through a symlinked ancestor.

**Requirements:** R1

**Dependencies:** review-hardening/1

**Files:**

```
install.sh                    # every site marked "unit 15" in tech.md R1 deletion-site table
flow/tests/adapters/run.sh    # section review-hardening/15
```

**Interfaces:**

- Consumes: `inside_target` from unit 1.
- Produces: —

**Steps:**

- [ ] Run `grep -n 'rm -f\|rm -rf\|-delete\|mv -f\|rmdir' install.sh` on the unit-1 result. Every hit must have a row in the [tech.md](tech.md) R1 deletion-site table. A new hit gets a row in this unit before any code.
- [ ] Write tests. Install with `--platform opencode,codex,droid` into `$T`, then:
  - (a) manifest file: move one host file that `remove_host_file` deletes to `$OUT` and symlink its directory back; (b) manifest package: move one host package dir the manifest lists to `$OUT` and symlink it back;
  - (c) no-manifest fallback (`remove_host_package`, source walk, :678 and :686): delete `.vibe/install-manifest.json`, move `.opencode` to `$OUT` and symlink it back;
  - (e) manifest record (:1147): move `.vibe` to `$OUT3` and symlink it back;
  - (f) `.gitignore` (:1044): replace `.gitignore` with a symlink to `$OUT/gi`;
  - (g) prune and rmdir sites (:1063-1064, :1124-1125, :1140, :1148, :1162-1163): a vendored target (`.claude -> vendor/vibe/.claude`, as in unit 1) whose `vendor/vibe/.claude/skills` and `.claude/hooks` are empty, and `plugins -> $OUT4` holding an empty `vibe/`: `find vendor/vibe $OUT4 -type d | wc -l` is unchanged after uninstall.
  - Then `--uninstall --yes`: every file and link in `$OUT`, `$OUT3` survives byte-for-byte, the `.gitignore` symlink is still a symlink, and the output contains `WARN` for each. Run (c) with `--dry-run` too: the output shows the skip. **Control:** the same install with no symlink uninstalls cleanly, exits 0, and removes every recorded file.
  - The adapter-link rm (:1077) gets no case: `flow/reference/adapters.json` lists only `CLAUDE.md` and `WARP.md`, both at the target root, so no ancestor can be a symlink (table row in [tech.md](tech.md)).
- [ ] Run the adapters suite. Expected FAIL: (a), (b), (c), (e) delete files outside the target; (f) replaces or deletes the link; (g) removes empty directories outside the target.
- [ ] Implement per the table: guard each "unit 15" delete site with `inside_target` and `err "WARN: <path> is outside the target; left alone"` on a miss; `dry_remove_rel` prints the same skip; run each prune or rmdir only when `inside_target "<dir>/."` passes (silently, since an empty-dir prune is best-effort). `.gitignore`: when `[[ -L "$GI" ]]`, skip the rewrite with a WARN (Open Question 5).
- [ ] Run the adapters suite, `bash tests/run.sh` and the CI shellcheck command. CHANGELOG bullet.

**Test scenarios:**

- Symlinked host file, host package, no-manifest package, `.vibe` and `.gitignore` are all kept with a WARN; empty directories behind a symlinked ancestor survive; the plain uninstall is unchanged.

**Verification:** adapters suite green; the grep has no hit without a table row.

---

### review-hardening/4 — resolve_target and write-through for bash writers

> Track D note: the install-level R11 case (AGENTS.md -> /nonexistent exits 0, prints `AGENTS.md not merged`) is deferred until track A (unit 1) lands that message in `install.sh`.

**Goal:** a symlinked managed document stays a symlink; dangling or outside links are refused.

**Requirements:** R6, R11

**Dependencies:** track A merged

**Files:**

```
flow/scripts/lib/resolve-target.sh   # NEW, from the tag
flow/scripts/merge-agents.sh         # every hunk of the old diff except @@ -159 (link_adapter, unit 3)
flow/scripts/regen-active-rules.sh   # all 4 old hunks (apply at -F0)
flow/tests/adapters/run.sh           # section review-hardening/4 (rh4_regen_sandbox); install case for R11
flow/tests/run.sh                    # copy lib/resolve-target.sh into the sandboxes at :690, :759, :1875
flow/README.md                       # the "temp + rename / never clobber" line, if it is now false
```

**Interfaces:**

- Consumes: the R11 install contract from unit 1.
- Produces: `resolve_target FILE ROOT`.

**Steps:**

- [ ] Write tests: port the tag's `# review-hardening/4` section. (a) `T/AGENTS.md -> docs/AGENTS.real.md` with user prose: merge and regen keep the link, and the real file carries the block and the prose. (b) dangling link: non-zero, a warning names the link, nothing created. (c) link to an outside file: non-zero, the outside file is unchanged (`cmp`). (d) unmerge on a symlinked AGENTS.md keeps the link. **Control:** a regular AGENTS.md behaves as today. Update `rh4_regen_sandbox` to copy `lib/resolve-target.sh`, and give it `docs/spec/lessons.md`. Install case (R11, owned here): a target with `AGENTS.md -> /nonexistent` installs with exit 0, prints `AGENTS.md not merged`, and `AGENTS.md` is still the dangling link.
- [ ] Run the adapters suite. Expected FAIL: (a) the link is replaced by a regular file; (c) the outside file is clobbered; the install case replaces the dangling link and prints no warning.
- [ ] Implement:
  - `git show rh-on-54e5983:flow/scripts/lib/resolve-target.sh > flow/scripts/lib/resolve-target.sh`; `git add -N` it.
  - `merge-agents.sh`: split the old diff (Global Constraints). It has 9 hunks; hunk 4 is `@@ -159` (`link_adapter`). Apply `head` plus hunks 1-3 and 5-9. Hunk 5 (`@@ -190`) is labelled `link_adapter()` by git's function context, but it edits `seed_rules_block`, so it belongs here.
  - `regen-active-rules.sh`: `git diff 5ccb99b rh-on-54e5983 -- flow/scripts/regen-active-rules.sh | patch -p1 -F0`. Check that the result keeps `REPO_ROOT`, the v0.4 lessons lookup, every `# literal` tag, `REFUSED=0` and the end-of-script `exit 1`.
  - `flow/tests/run.sh`: after each `cp … regen-active-rules.sh` at :690 and :759, add `mkdir -p <dir>/lib; cp "$SCRIPTS/lib/resolve-target.sh" <dir>/lib/` as the tag does; in the `spec-v04/p4` loop at :1874, add the same for `$dr/pkg/flow/scripts/lib`.
- [ ] Run the adapters suite and `bash flow/tests/run.sh` (the three regen sandboxes must pass); `bash tests/run.sh`; the CI shellcheck command; `./build-plugin.sh --check` (expected `OK`); `bash spec/tests/plugin-validate.sh`. Install into a temp target, then check that `.agents/skills/vibe/scripts/lib/resolve-target.sh` exists, and that it is gone after `--uninstall`.
- [ ] Docs: the comment above `strip_managed_block`; `grep -n "temp + rename\|never clobber" flow/README.md` and fix each false line; CHANGELOG bullet.

**Test scenarios:**

- Write-through, dangling refusal, outside refusal, unmerge, plain-file control, install continues past a dangling link.

**Verification:** adapters and flow suites green, shellcheck clean.

---

### review-hardening/3 — link_adapter refuses a foreign symlink

> Track D note: the install-level assertion that output names `docs/other.md` and omits `real file?` is deferred until unit 1 lands the held-link message; the current test only checks the CLAUDE.md symlink is unchanged.

**Goal:** `link` never rewrites a symlink that points elsewhere, and the install names the held link.

**Requirements:** R5

**Dependencies:** review-hardening/4 (same file)

**Files:**

```
flow/scripts/merge-agents.sh    # hunk @@ -159 of the old diff (link_adapter :156-172)
flow/setup.md                   # :44 "relinks a wrong target"
flow/tests/adapters/run.sh      # section review-hardening/3 (script and install level)
```

**Interfaces:**

- Consumes: the `@@ -830` held-link message from unit 1.
- Produces: `merge-agents.sh link` exits 1 on any existing non-matching file or symlink.

**Steps:**

- [ ] Write tests: `T/CLAUDE.md -> other.md`: `bash "$MERGE" link CLAUDE.md "$T"` exits 1, `readlink` is still `other.md`, stderr names `other.md`. Install level (owned here): target with `CLAUDE.md -> docs/other.md` and `--adapters claude`: exit 0, the output names `docs/other.md` and not `real file?`, and the link is unchanged. **Controls:** a correct link is skipped (exit 0); a real file is refused (exit 1); an absent file is linked. The existing link tests (the agent-instructions unit-5 tests, run.sh :203-209) stay as they are.
- [ ] Run the adapters suite. Expected FAIL: the link is rewritten to `AGENTS.md` with exit 0, and the install prints no warning.
- [ ] Implement: apply `head` plus hunk 4 from unit 4's split. Edit `flow/setup.md` to "refuses to replace a symlink or a real file".
- [ ] Run the adapters suite, `bash tests/run.sh` and the CI shellcheck command. CHANGELOG bullet.

**Test scenarios:**

- A foreign symlink is untouched, exit non-zero, and the install message names the target.

**Verification:** adapters suite green; `grep -n "relinks a wrong target" flow/setup.md` prints nothing.

---

### review-hardening/5 — Cursor writers validate their arguments

**Goal:** neither cursor writer emits invalid JSON or accepts a pattern as a state.

**Requirements:** R7

**Dependencies:** —

**Files:**

```
flow/scripts/set-state.sh             # validate_args, valid_feature, FEATURE_GIVEN, carried-feature drop
flow/engine/cursor.mjs                # validFeature
flow/engine/commands/state.mjs        # runSet refusal and drop
flow/engine/tests/state.test.mjs      # review-hardening/5 tests from the tag
flow/tests/adapters/run.sh            # section review-hardening/5, beside the set-state parity section
```

**Interfaces:**

- Consumes: the 4-space state-key indentation of `flow/state-machine.json` (the jq-less parser, `set-state.sh:62-68`). skill-revision unit 22 keeps that indentation.
- Produces: `validFeature(name) -> boolean`.

**Steps:**

- [ ] Write tests: port the tag's `state.test.mjs` additions and the adapters section. With jq and under `mkshim jq`: state `idle|x`; feature `a"b`; a feature holding a backslash; a feature holding a newline. Each exits 1, the cursor bytes are unchanged, and `jq -e .` parses the cursor. **Controls:** an empty feature argument carries the current feature; `feature.impl demo`; feature `$(touch x)` is written literally and no file `x` appears; `quick.fix`.
- [ ] Run `node flow/engine/tests/run.mjs state.test` and the adapters suite. Expected FAIL: `idle|x` and the control-character features without jq (the cursor is invalid or rewritten); the engine accepts them.
- [ ] Implement: apply `git diff 5ccb99b rh-on-54e5983 -- flow/scripts/set-state.sh flow/engine/commands/state.mjs flow/engine/cursor.mjs` (clean). Check the jq-less 4-space-indent match: `grep -cE '^    "[a-z]+\.[a-z]+"[[:space:]]*:[[:space:]]*\{' flow/state-machine.json` prints 13.
- [ ] Run both suites, `bash tests/run.sh` and the CI shellcheck command. Replace the `json_string` comment with the enforced charset. CHANGELOG bullet.

**Test scenarios:**

- Hostile names are rejected on both paths, with and without jq; legal names are unchanged.

**Verification:** both suites green.

---

### review-hardening/6 — detect-context.sh normalises like the engine

**Goal:** `decide_bash` blocks every spelling of a guarded path that the engine blocks.

**Requirements:** R9

**Dependencies:** —

**Files:**

```
flow/scripts/detect-context.sh        # normalise_path; decide_bash head (old hunk at -F0)
flow/engine/policy.mjs                # globToRegExp `s` flag
flow/engine/tests/policy.test.mjs     # differential matrix
```

**Interfaces:**

- Consumes: —
- Produces: `normalise_path PATH`.

**Steps:**

- [ ] Write tests: port the tag's `policy.test.mjs` additions. Extend `guardedPathsFor` so each literal pattern also yields `.//p`, a `/./` insertion, `x/../p`, `//p` and `./p/`, and include the v0.4 `docs/spec/*` and `docs/superpowers/specs/*` patterns. Add `review-hardening/6: alias spellings decide identically with and without node` for `docs/spec//lessons.md`, `docs/spec/./plan.md`, `docs/spec/x/../lessons.md` in `idle`, each starting `block:`. Add a trailing-newline path. **Control:** `docs/lessons.md` allows. Raise the `paths.length` and `engineRows.length` floors to the new derived counts.
- [ ] Run `node flow/engine/tests/run.mjs "differential matrix"`. Expected FAIL: `branches diverged`, listing the alias rows (engine `block:`, bash `allow`).
- [ ] Implement: `git diff 5ccb99b rh-on-54e5983 -- flow/scripts/detect-context.sh | patch -p1 -F0` (applies cleanly). Tag every added line that spells the legacy root with a trailing `# literal` (the file has no `literal-file` marker); then `bash tests/literals/run.sh`. Add the `s` flag in `policy.mjs`.
- [ ] Run the engine suite, `bash tests/literals/run.sh`, `bash tests/run.sh` and the CI shellcheck command. CHANGELOG bullet.

**Test scenarios:**

- Aliases block identically; unrelated paths and controls still allow; floors raised.

**Verification:** engine and literal suites green.

---

### review-hardening/7 — Plugin SessionStart hook: only where the flow is installed, and only once

**Goal:** the plugin hook emits the doctrine only in a repo that has the flow installed, and stays silent when the repo-local hook already emits it.

**Requirements:** R10

**Dependencies:** —

**Files:**

```
build-plugin.sh                  # heredoc template and comments (hand-merge: 1 of 2 old hunks fails)
plugin/hooks/session-start.sh    # GENERATED by ./build-plugin.sh
flow/tests/adapters/run.sh       # section review-hardening/7
flow/tests/run.sh                # spec-v04/p4 pin :1255-1263 flips to "silent without the flow"
```

**Interfaces:**

- Consumes: —
- Produces: a silent exit 0 under the probe in [tech.md](tech.md) R10.

**Steps:**

- [ ] Write tests running `CLAUDE_PLUGIN_ROOT="$REPO_ROOT/plugin" CLAUDE_PROJECT_DIR=<dir> bash "$REPO_ROOT/plugin/hooks/session-start.sh" </dev/null`: (a) a plain repo with only `docs/spec/` prints nothing (flow not installed); (a2) the same with the legacy root prints nothing; (b) a real `install.sh` target prints nothing (repo hook answers); (c) a repo with `.agents/skills/vibe/state-machine.json` and the doctrine script but no settings prints the doctrine (**control**); (d) a wired `settings.json` with no node on PATH prints the doctrine (**control**); (e) a wired `settings.local.json` prints nothing. Also `grep -c "this project's cursor" plugin/hooks/session-start.sh` is 0. Flip the `spec-v04/p4` loop at `flow/tests/run.sh:1255-1263` to assert silence and exit 0 for both roots, and add one sandbox with `.agents/skills/vibe/state-machine.json` that asserts `vibe flow`.
- [ ] Run the adapters suite and `bash flow/tests/run.sh`. Expected FAIL: (a), (a2), (b), (e), the flipped p4 pin and the comment check.
- [ ] Implement: hand-merge `git diff 5ccb99b rh-on-54e5983 -- build-plugin.sh` into the v0.4 heredoc (the clean hunk applies; the failing one is merged by hand). Replace the `docs/spec/` and legacy-root test line with `[ -f "$PROJECT/.agents/skills/vibe/state-machine.json" ] || exit 0` (Open Question 4). Run `./build-plugin.sh`.
- [ ] Run `./build-plugin.sh --check` (expected `OK`), the adapters and flow suites, `bash tests/run.sh`, the CI shellcheck command and `bash spec/tests/plugin-validate.sh`. CHANGELOG bullet.

**Test scenarios:**

- A spec-only repo stays silent; an install target stays silent; a flow repo without wiring, and a node-less wired repo, emit.

**Verification:** `--check` green; adapters and flow suites green.

---

### review-hardening/19 — The flow guard matcher covers MultiEdit

**Goal:** `MultiEdit` writes reach the flow write-invariant guard, as `Edit` and `Write` do.

**Requirements:** R13

**Dependencies:** —

**Files:**

```
flow/scripts/merge-settings.sh     # matcher at :13 (comment), :43, :73; upgrade of an older matcher
.claude/settings.json              # :16 this repo's flow guard matcher
flow/tests/run.sh                  # review-fix block :1270-1290 (substring pin :1276; legacy-matcher fixture :1287)
flow/tests/adapters/run.sh         # exact matcher pin :300; section review-hardening/19
flow/hooks-fallback/pre-tool-use-guard.sh   # header comment :30 only (no code change)
```

**Interfaces:**

- Consumes: the engine guard decides any non-Bash tool by `tool_input.file_path` (`hook.mjs:555`); the bash fallback routes every non-Bash tool to the path policy (`pre-tool-use-guard.sh:116-133`) and reads `file_path` (:137-139). No guard code changes.
- Produces: shipped matcher `Edit|Write|MultiEdit|NotebookEdit|Bash`.

**Steps:**

- [ ] Write tests (section `review-hardening/19`): a fresh `merge-settings.sh` run writes the new matcher; a settings file holding the old `Edit|Write|NotebookEdit|Bash` matcher ends with exactly one vibe group carrying the new matcher (the script strips vibe groups by command path and re-adds), and a second run changes nothing (idempotent); in an installed target at `idle`, a `{"tool_name":"MultiEdit","tool_input":{"file_path":"docs/spec/lessons.md"}}` payload through the engine guard and through the fallback guard exits 2. **Control:** the same payload with `Edit` exits 2 today; `docs/notes.md` with `MultiEdit` exits 0. Change the exact pin at `adapters/run.sh:300` and the substring pin at `flow/tests/run.sh:1276` to the new matcher.
- [ ] Run the adapters and flow suites. Expected FAIL: the matcher pins, and the upgrade case (the old matcher stays).
- [ ] Implement: the matcher in `merge-settings.sh` (three sites) and `.claude/settings.json`; fix the header comment of the fallback guard. The upgrade needs no new code.
- [ ] Run the adapters and flow suites, `node flow/engine/tests/run.mjs parity` (expected unchanged: parity runs behaviour, and the fallback's code is untouched), `bash tests/run.sh` and the CI shellcheck command. CHANGELOG bullet. The doc lines that state the matcher (`AGENTS.md`, `README.md`, `flow/README.md`) belong to skill-revision unit 27; its sweep (unit 29) derives the list from `merge-settings.sh`.

**Test scenarios:**

- New and upgraded settings carry `MultiEdit`; a MultiEdit write to a guarded path blocks on both guard paths; an unguarded path allows.

**Verification:** adapters and flow suites green.

---

### review-hardening/11 — Physical confinement for content reads

**Goal:** every config-named or walked content read resolves physically and stays in the repo, including the legacy lessons read.

**Requirements:** R4

**Dependencies:** —

**Files:**

```
flow/engine/content.mjs                  # walkMarkdown, loadBlocksDir, SECURITY comment, confineRealWhy, applyConfigBlocks, loadContent, lessonsFor, buildResolver
flow/engine/tests/content.test.mjs       # makeEscapeFixture, composedAll, legacy-lessons cases
flow/engine/tests/primitives.test.mjs    # WAIVERS :920, REASONS :864
```

**Interfaces:**

- Consumes: —
- Produces: `confineRealWhy`, `confineReal` (exported).

**Steps:**

- [ ] Write tests: port the tag's review-hardening/11 tests with `makeEscapeFixture()` and `composedAll(sb)` onto `makeContentSandbox`. Add three legacy-lessons tests next to the existing "falls back to the legacy root" test: (a) no `docs/spec/lessons.md` and a legacy lessons symlink to outside: no secret in the output, and the `renderChannel` errors name it; (b) **control:** an in-repo legacy symlink is read; (c) **control:** `docs/spec/lessons.md` present while the legacy file links outside: no error. Each legacy path literal in the test carries `// literal`, or the file already has the marker.
- [ ] Run `node flow/engine/tests/run.mjs content.test`. Expected FAIL: the secret is injected with no error; the SECURITY-comment pin fails on `symlink-free`.
- [ ] Implement: apply the old `content.mjs` hunks except `lessonsFor`; hand-merge `lessonsFor` so both reads use `confineRealWhy`; update the WAIVERS line text and the REASONS text in the same commit.
- [ ] Run `node flow/engine/tests/run.mjs content.test`, then `primitives`, then the full engine suite. Expected PASS; the symlink-cycle test stays green. CHANGELOG bullet.

**Test scenarios:**

- Escapes via a file, a directory, a config file, configured lessons and legacy lessons; in-repo and symlinked-root controls; the comment pin.

**Verification:** engine suite green; `git diff --stat oracles` is empty.

---

### review-hardening/12 — render --write confinement and write-through

**Goal:** `render --write` writes through in-repo symlinks and refuses dangling or outside ones.

**Requirements:** R4, R6

**Dependencies:** review-hardening/11

**Files:**

```
flow/engine/commands/render.mjs      # writeTarget :62-77; write block :134-163
flow/engine/tests/render.test.mjs    # makeOutside, docSandbox
```

**Interfaces:**

- Consumes: `confineRealWhy` from unit 11.
- Produces: —

**Steps:**

- [ ] Write tests: port the tag's review-hardening/12 tests. (a) an in-repo link keeps the link and updates the real file; (b) an outside link: exit 1, outside bytes unchanged, stderr contains `escapes`; (c) a planted link at the old temp name is not followed; (d) a dangling link: exit 1, nothing created; (e) an in-repo dir linked outside is refused. **Controls:** a regular file; a missing file in an existing dir; a symlinked root.
- [ ] Run `node flow/engine/tests/run.mjs render.test`. Expected FAIL: (a) the link is replaced; (b)–(e) write outside or follow the planted link.
- [ ] Implement: apply `git diff 5ccb99b rh-on-54e5983 -- flow/engine/commands/render.mjs` (clean).
- [ ] Run the render tests and the full engine suite. Expected PASS. CHANGELOG bullet.

**Test scenarios:**

- Write-through, outside, planted temp, dangling, symlinked directory, controls.

**Verification:** engine suite green.

---

### review-hardening/9 — Engine gate: NUL output, project scope, fail closed

**Goal:** the engine gate judges every changed path of the project and blocks when it cannot judge.

**Requirements:** R2, R3

**Dependencies:** —

**Files:**

```
flow/engine/tests/parity.test.mjs   # first: GATE_ORACLE :74 -> oracles copy, GATE_LIVE, banner test :1529-1541; realPorcelain argv; pins for existing fixtures the engine change splits
flow/engine/commands/hook.mjs       # gitPorcelain :707-725; loop :744-835; runGateHook :845-889; comments :82-126, :654-705; message :796
flow/engine/tests/hook.test.mjs     # nulPorcelain, fakeGit, makeNestedSandbox (new); realPorcelain :1287
```

**Interfaces:**

- Consumes: —
- Produces: `GATE_ORACLE`, `GATE_LIVE`; `gitPorcelain(root, opts) -> { determined, noGit?, text, entries, why? }`; test helpers `nulPorcelain(rows)`, `fakeGit(rows, { prefix })`, `makeNestedSandbox()` (outer `git init`, signing off per Global Constraints, `GIT_CEILING_DIRECTORIES` = the sandbox parent).

**Steps:**

- [ ] Behaviour-neutral first step: repoint `GATE_ORACLE`, add `GATE_LIVE`, and retarget the banner test at the oracle path. Run `node flow/engine/tests/run.mjs parity`. Expected PASS with no change, because the two files are byte-identical. `GUARD_ORACLE` stays.
- [ ] Write tests with the real git binary via `makeRealGitFixture` (signing off, setup exit status checked): untracked `my file.txt`, `café.txt`, `a\nb.txt`, and a staged rename to `moved file.txt` each block with `the evidence receipt is stale` and the path. **Controls:** plain `file.txt` blocks; a receipt newer than every file passes. **Floor:** each case asserts that the real porcelain gives at least one entry. Nested: `sub/src.txt` modified blocks; a newer `outer/other.txt` passes; `sub/.vibe/last-inject` and `sub/.agents/skills/vibe/warnings.log` never block. Stat error: a `fakeGit` row `?? loop` with `loop -> loop` blocks and names `loop` and `ELOOP`; a row for a missing path passes. The TDD quirk test stays unchanged.
- [ ] Run `node flow/engine/tests/run.mjs hook.test`. Expected FAIL: the awkward-name, rename, nested and ELOOP cases exit 0 instead of 2. The controls pass.
- [ ] Implement per [tech.md](tech.md) R2/R3 engine gate.
- [ ] Run `node flow/engine/tests/run.mjs parity` right after the engine change. `GATE_FIXTURES` (:905-922) still compare the engine against the frozen oracle. Any existing fixture that now diverges gets a `KNOWN DIVERGENCE` pin in this unit, with the finding as reason and a plain-name control where both agree. No existing row may be deleted.
- [ ] Run `node flow/engine/tests/run.mjs hook.test`, the full suite, and the full suite on the no-jq PATH. Expected PASS. New three-way rows for the new cases go in unit 10. CHANGELOG bullet.

**Test scenarios:**

- Four awkward names, nested project, own-runtime exclusion, ELOOP fail-closed, controls and floors; existing parity rows green or pinned.

**Verification:** engine suite green; `bash oracles/check.sh` passes.

---

### review-hardening/10 — Live bash fallback decodes NUL output and scopes to the project

**Goal:** the no-node Stop gate matches the fixed engine; the oracle stays frozen.

**Requirements:** R2, R3

**Dependencies:** review-hardening/9

**Files:**

```
flow/hooks-fallback/stop-gate.sh     # banner :1-27; git_changed_z; loop :172-198 (own-runtime exclusions); comment :163-171
flow/engine/tests/parity.test.mjs    # three-way rows after GATE_FIXTURES :710-724
```

**Interfaces:**

- Consumes: `GATE_LIVE`, `GATE_ORACLE`, `makeNestedSandbox` from unit 9.
- Produces: `git_changed_z` (internal).

**Steps:**

- [ ] Add a `GATE_LIVE` leg (jq and no-jq) to the existing `GATE_FIXTURES` loop (:905-922): every existing fixture must give the same rc for engine and live, with each existing `KNOWN DIVERGENCE` pin carried to the live leg while the live file still behaves like the oracle. Expected PASS before any code change (live and frozen are byte-identical); it is the regression net for the rewrite below.
- [ ] Write tests modelled on the "untracked directory" pin (:789): run the engine, `GATE_LIVE` (jq and no-jq via `oracleEnv`) and `GATE_ORACLE` on one fixture. Rows: `café.txt` (quoted name), a staged rename to `moved file.txt`, and the nested project with a fresh `sub/.vibe/last-inject` and `sub/.agents/skills/vibe/warnings.log` beside a stale `sub/src.txt`. Expect engine rc 2 and live rc 2, and pin frozen rc 0 as `KNOWN DIVERGENCE` with the finding as reason. Nested own-runtime row: with only the two runtime files fresh, engine and live pass (rc 0). **Control:** a plain name where all three give rc 2. **Floor:** the three rows plus the control, each running all three legs. Banner test: the live file no longer carries `FROZEN PARITY ORACLE` and names the oracle path.
- [ ] Run `node flow/engine/tests/run.mjs parity`. Expected FAIL: live rc 0 on the awkward and nested cases; live rc 2 on the own-runtime row; the banner assertion fails.
- [ ] Implement per [tech.md](tech.md) R2/R3 bash fallback, including the two own-runtime `case` arms.
- [ ] Run the full engine suite on both jq legs, `bash tests/run.sh` and the CI shellcheck command; run the nested fixture under `/bin/bash` (3.2 on macOS). Expected PASS. CHANGELOG bullet.

**Test scenarios:**

- Three-way table with a control and floors; own-runtime exclusion in the fallback; banner change.

**Verification:** parity green, including the live leg over every existing gate fixture; `bash oracles/check.sh` passes.

---

### review-hardening/17 — run-hook.sh keeps a guard block without a temp file

**Goal:** a real `blocked:` exit 2 survives a missing temp directory.

**Requirements:** R12

**Dependencies:** —

**Files:**

```
spec/hooks/run-hook.sh    # :33-52 stderr capture
spec/tests/hooks.sh       # new cases next to the fake() tests
```

**Interfaces:**

- Consumes: —
- Produces: —

**Steps:**

- [ ] Write tests with `fake block.sh 2 'blocked: invoke Skill(spec) first\n'`: `TMPDIR="$SCRATCH/none" VIBE_SPEC_BIN="$SCRATCH/block.sh" bash "$SHIM" guard` exits 2 and stderr is the `blocked:` line. **Controls:** the same with a working `TMPDIR` exits 2; a crash fake (exit 3) exits 0; no `vibe-spec-hook.*` file is left in `$TMPDIR`.
- [ ] Run `bash spec/tests/hooks.sh`. Expected FAIL: the missing-TMPDIR case exits 0 with `hook guard failed (exit 2) — allowing`.
- [ ] Implement per [tech.md](tech.md) R12.
- [ ] Run `bash spec/tests/hooks.sh`, `./build-plugin.sh --check`, `bash spec/tests/plugin-validate.sh`, `bash tests/run.sh` and the CI shellcheck command. Expected PASS, `OK`, no shellcheck output. CHANGELOG bullet.

**Test scenarios:**

- Block kept with no temp dir; crash still allows; no temp left behind.

**Verification:** `spec/tests/hooks.sh` green.

---

### review-hardening/18 — Rust setup refuses a symlinked AGENTS.md (optional)

**Goal:** pin the upstream refuse-all behaviour with a test. Skip it unless the owner opts in.

**Requirements:** R6

**Dependencies:** —

**Files:**

```
crates/vibe-spec/tests/setup.rs    # new cases beside the symlinked .git/hooks test
```

**Steps:**

- [ ] Write `#[cfg(unix)]` tests: `AGENTS.md` as an in-repo symlink and as a symlink to an outside file; `vibe-spec setup --agents-md` exits 1, the message contains `symlink; not writing through it`, and the target bytes are unchanged.
- [ ] Run `cargo test -q --locked -p vibe-spec --test setup`. Expected: PASS at once (behaviour pin, not a fix). Plant a mutant in `existing()` that skips the symlink check, confirm with `git diff --numstat`, see the new cases FAIL, then revert.
- [ ] Run `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings`.

**Verification:** `cargo test -p vibe-spec` green; the mutant check is recorded.

---

### review-hardening/13 — Final verification

**Goal:** prove the whole branch after all merges.

**Requirements:** R1 to R13

**Dependencies:** review-hardening/1, /3 to /7, /9 to /12, /15, /17, /19

**Files:**

```
(no edits; evidence only, in .agents/skills/vibe/evidence/feature-review-hardening.md)
```

**Steps:**

- [ ] `bash tests/run.sh`. Expected: every suite PASS (spec, flow, adapters, engine, oracles, contracts, literals, and skills once skill-revision unit 1 has merged).
- [ ] `bash spec/tests/hooks.sh` and `bash spec/tests/agents.sh`. Expected: `0 failed`.
- [ ] `bash spec/tests/plugin-validate.sh` (units 4, 7 and 17 change what ships in `plugin/`). Expected: `PASS`, or the one `SKIP` line when `claude` is absent; record which.
- [ ] `/bin/bash tests/run.sh` on macOS. Expected: PASS under bash 3.2.
- [ ] `CLAUDE_PROJECT_DIR="$PWD" bash tests/run.sh`. Expected: PASS (hermeticity leg).
- [ ] No-jq leg: build the shim dir as in `.github/workflows/ci.yml` (the "jq stripped from PATH" step), then `PATH="$NOJQ_BIN" VIBE_NO_JQ=1 node flow/engine/tests/run.mjs`. Expected: PASS.
- [ ] Spaced-path leg: tar the tree into `"$(mktemp -d)/with space/vibe"` and run `node flow/engine/tests/run.mjs` there. Expected: PASS.
- [ ] `cargo run -q --locked -p vibe-spec -- validate --strict`. Expected: no errors.
- [ ] `cargo run -q --locked -p vibe-spec -- check-drift`. Expected: the `check-drift: OK` line, and a note that the root plan names this feature as IN PROGRESS.
- [ ] `cargo run -q --locked -p vibe-spec -- migrate --to docs/spec --check`. Expected: exit 0, no literals.
- [ ] `bash oracles/check.sh` and `bash contracts/run.sh`. Expected: PASS; `git diff --stat oracles` is empty.
- [ ] `git ls-files '*.sh' | grep -Ev '^oracles/(bash|js)/' | xargs shellcheck`. Expected: no output.
- [ ] `./build-plugin.sh --check`. Expected: `OK`.
- [ ] Rust, only if unit 18 landed: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`.
- [ ] Reproduce finding 1 on a tar copy: `install.sh <copy> --uninstall --yes` exits 1 and the spec file count is unchanged.
- [ ] `grep -n 'rm -f\|rm -rf\|-delete\|mv -f\|rmdir' install.sh` matches the [tech.md](tech.md) R1 table row for row.
- [ ] `grep -rn "relinks a wrong target\|this project's cursor\|symlink-free\|FROZEN PARITY ORACLE" flow/setup.md flow/engine/content.mjs flow/hooks-fallback/stop-gate.sh build-plugin.sh plugin/hooks`. Expected: no output. (`pre-tool-use-guard.sh` keeps its banner, because it stays the guard oracle.)
- [ ] Whole-branch review: one `opus` reviewer over `git diff <step-0 base>..HEAD -- <port-owned files>`; fix must-fix findings in their unit's files and re-run the affected rows above.

**Verification:** each command above, with its real output pasted into the verify receipt.

---

### review-hardening/14 — Compound: lessons, promotion, root row

**Goal:** record lessons and wrap up this folder. This runs in `feature.compound` only.

**Requirements:** R1 to R13

**Dependencies:** review-hardening/13 and the ship approval

**Files:**

```
docs/spec/lessons.md                         # compound only, append
docs/spec/tech/rust/rust-parity-pins.md      # promotion target for both merge blocks (bump updated:)
docs/spec/plan.md                            # row -> DONE (bump updated:)
AGENTS.md                                    # vibe:active-rules block, regenerated
```

**Steps:**

- [ ] Invoke `Skill(spec)`. Add tagged lessons (Pattern, Rule, Tags, Date): path guards check physical paths; parse git with `-z` scoped to the project; port fixes onto a moved base by applying the old diff after all its tests are RED, then re-check every deletion site the new base added; a shim that captures stderr must not depend on a temp file.
- [ ] Promote: both `<!-- merge -->` blocks state what the Rust ports (F6, F11) must keep, so they go to `docs/spec/tech/rust/rust-parity-pins.md`. Run `cargo run -q --locked -p vibe-spec -- promote review-hardening --dry-run --target docs/spec/tech/rust/rust-parity-pins.md`, show it, apply on yes. If the note would pass 60 lines, create `docs/spec/tech/rust/rust-hardening-pins.md` (`type: tech`, `updated:`), list it in the `children:` of `docs/spec/tech/rust.md`, and target it instead. Bump `updated:` on every note touched.
- [ ] Run `bash .agents/skills/vibe/scripts/regen-active-rules.sh` (rewrites the `AGENTS.md` `vibe:active-rules` block; see the shared-file rule).
- [ ] Set the root row to DONE and bump `updated:` in `docs/spec/plan.md` (a status edit is a spec edit). `git mv docs/spec/features/review-hardening docs/spec/archive/review-hardening`. Run `cargo run -q --locked -p vibe-spec -- validate --strict` and `-- check-drift` until both exit 0. Ask to delete the archive.

**Verification:** validate and check-drift output; `git diff docs/spec AGENTS.md` shows only lessons, the promotion, the row and the regenerated block.

---

## Open Questions

1. **Fix the live JS and bash gate now, or only in Rust (units 9 and 10).** `oracles/README.md` says a defect found during parity is "fixed in Rust only", and the Rust plan deletes the JS engine once parity is green. But no Rust stop gate exists, and the JS and bash gates run on every install today. **Default:** fix the live code now, pin each new case as `KNOWN DIVERGENCE` against the frozen oracles, and add the same cases to the F6 receipt-and-git-state corpus. **Alternative:** skip units 9 and 10 and carry the cases into F6 only. This is a human-gate decision at plan approval.
2. **Feature-name charset (unit 5).** Default: reject control characters, `"` and `\`. Stricter alternative: `^[A-Za-z0-9._-]+$`.
3. **Unit 18.** Default: skip. Take it only if Rust test coverage of the refuse-all policy is wanted now rather than in F11.
4. **Plugin doctrine in spec-only repos (unit 7, flow:V-09).** **Default:** emit only when `.agents/skills/vibe/state-machine.json` exists, because a repo without the flow gets no orders or level line, so the doctrine misleads. A plugin-only flow user loses the doctrine too; today that user also gets no orders. **Alternative:** keep emitting in every `docs/spec` repo (today's behaviour, pinned at `flow/tests/run.sh:1260`).
5. **Symlinked `.gitignore` at uninstall (unit 15).** Default: leave it alone and print a WARN naming the vibe lines to remove by hand. Alternative: write through the link when it resolves inside the target.
