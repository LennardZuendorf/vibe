---
type: feature-plan
feature: vibe-cli
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-03
---

# Feature: vibe-cli — Implementation Plan

Build a fully-loaded skeleton first (unit 1: all package dirs, `__init__.py` stubs, the
frozen `pyproject.toml`, the shared `markers.py`, the complete `conftest.py` fixture set,
and the entire vendored `_assets/` tree), then fan out the leaf modules in dependency
waves, then wire `app.py` and retire the legacy. Parity suites vs the current bash scripts
are the merge gate: bash stays until its Python replacement is byte-identical.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Design (approved):** [research.md](research.md)

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | One-command project setup | vibe-cli/7, /8, /9, /10, /11, /12, /17 |
| R2 | Orientation and legal transitions | vibe-cli/2, /3, /6 |
| R3 | Health report with CI gate | vibe-cli/13 |
| R4 | Surgical, cursor-safe lifecycle | vibe-cli/8, /14 |
| R5 | Spec framework wrapped, not rewritten | vibe-cli/15 |
| R6 | Runtime safety — fast guard, persistent install | vibe-cli/4, /7, /11 |

---

## Key Technical Decisions

1. **Fully-loaded skeleton (unit 1), then leaf-only builders.** Unit 1 is the ONLY unit
   that creates shared surface: every package dir + empty `__init__.py`
   (`vibe/`, `ui/`, `provision/`, `commands/`), the frozen `pyproject.toml` (all runtime
   deps + a recursive `_assets/**/*` package-data glob), `markers.py`, `conftest.py` (the
   full fixture set), and the complete vendored `_assets/` tree. Unit 1 then does the single
   editable install (`uv sync`). Every later builder creates ONLY its own leaf module + test
   file, imports the shared modules, runs ONLY its own test node
   (`uv run pytest cli/tests/test_<x>.py`), and never edits `pyproject.toml`, `app.py`, or
   `conftest.py`, never runs `uv sync`.
2. **Vendor ALL assets in unit 1.** `src/vibe/_assets/` gets: `state-machine.json`,
   `state.example.json`, `deps.json`, `adapters.json`, `templates/AGENTS.md`, and the full
   `skills/{spec,vibe}/` trees (SKILL.md with its `vibe:orders` blocks, phase files, scripts,
   reference/**). Sources: `flow/**`, `spec/**`, `flow/reference/*.json`, `flow/state.example.json`.
   Downstream units READ these; none re-create them. Unit 16 sync-checks the whole tree vs source.
3. **Hot-path import purity (R6).** The per-Edit guard path — `hook.py`, `policy.py`,
   `orders.py`, `cursor.py`, `machine.py`, `errors.py`, `markers.py` — is **stdlib-only at
   import**: no `typer`, `rich`, or `pydantic`. `machine.py` loads+validates
   `state-machine.json` with plain `json` and exposes helpers (`next_of`, `inject_of`,
   `writes_of`, `caveman_of`, compound-key split/join); the rich commands reuse the same
   stdlib layer. `errors.py` stays a plain `Exception`; `typer.Exit` is raised only inside
   command modules. Unit 7 pins this with an import-cost test.
4. **Parity suites are the merge gate.** Units 4, 5, 10 each ship a byte-for-byte parity
   test vs its bash origin (`flow/scripts/detect-context.sh decide`, `orders.sh`,
   `regen-active-rules.sh`) over a fixture matrix. These require `jq`+`bash` in the test env
   (skip-with-message if absent). Each parity test pins WHERE it reads its bash origin and,
   for orders, WHICH `SKILL.md` (bundled `_assets/skills/vibe/SKILL.md` for parity/inject).
   Bash is retired (unit 16) only once parity is green.
5. **One shared `markers.py`.** The strict marker-pairing + reversal guard (ported from
   `merge-agents.sh`, NOT the laxer `regen-active-rules.sh` grep) lives once in unit 1's
   `markers.py`; units 8, 10, 13 import it — the standing "reuse the tested marker guard" lesson.
6. **`register(app)` wiring contract.** Every command module exposes
   `def register(app: typer.Typer) -> None` that adds its command(s) or `app.add_typer(group)`.
   `app.py` (unit 16) imports and calls each `register`; no builder edits `app.py`.
   Symbols: `flow_cmds.register` (6), `init_cmd.register` (11), `plugins_cmd.register` (12),
   `doctor_cmd.register` (13), `lifecycle_cmd.register` (14), `spec_cmd.register` (15),
   `rules_cmd.register` (10).
7. **Wave scheduling on corrected deps.** The orchestrator releases a unit only when its
   (corrected) dependencies are green, not all-at-once:
   - **Wave A** (parallel, dep = unit 1): 2, 8, 9, 10, 12, 17
   - **Wave B** (dep = 2): 3
   - **Wave C** (parallel, dep = 3): 4, 5, 6
   - **Wave D** (parallel): 7 (deps 4,5), 11 (deps 3,8,9,10,12,17), 13 (deps 2,3,9,markers,deps.json), 15 (deps 4,5,spec-tree)
   - **Wave E**: 14 (dep 11)
   - **Wave F**: 16 (all)

---

## Units

### vibe-cli/1 — Fully-loaded skeleton (dirs, pyproject, markers, conftest, assets)

**Goal:** `vibe --help` and `vibe-hook --help` run; the package is import-ready for every
downstream leaf; all shared surface exists so builders never touch it.

**Requirements:** — (infra for all)

**Dependencies:** —

**Files:**
```
cli/pyproject.toml                         # name=vibe-flow; hatchling src-layout;
                                           #   [project.scripts] vibe=vibe.app:main, vibe-hook=vibe.hook:main
                                           #   deps: typer, rich, pydantic; package-data force-include src/vibe/_assets/**/*
cli/src/vibe/__init__.py                   # version
cli/src/vibe/app.py                        # root typer app + @callback + main() (empty registry; wired in u16)
cli/src/vibe/hook.py                       # argparse stub + main() (filled in u7)
cli/src/vibe/ui/__init__.py  ui/console.py  ui/theme.py
cli/src/vibe/provision/__init__.py         # empty (package marker only)
cli/src/vibe/commands/__init__.py          # empty (package marker only)
cli/src/vibe/errors.py                     # VibeError(Exception), plain — no typer
cli/src/vibe/markers.py                    # strict pairing/reversal guard (from merge-agents.sh)
cli/tests/conftest.py                      # shared fixtures (below)
cli/tests/test_smoke.py  cli/tests/test_markers.py
cli/src/vibe/_assets/**                    # FULL vendored tree (Decision 2)
```
`conftest.py` fixtures (enumerated so no builder edits it): `home_sandbox` (monkeypatch
`Path.home` → tmp), `target_project` (builds a provisioned-or-bare sandbox target),
`bash_ref` (locates repo `flow/scripts/*.sh`, runs them; skips if no `jq`/`bash`),
`sample_lessons` and `machine_data` sample fixtures.

**Test scenarios:**
- `CliRunner().invoke(app, ["--help"])` exit 0; `vibe-hook --help` exit 0
- `markers.py`: replace-region round-trips; reversed markers raise (not mangle)
- `conftest` imports NO unit module at collection time

---

### vibe-cli/2 — machine.py (stdlib load of canonical state-machine.json)

**Goal:** stdlib-only load+validate of the bundled `state-machine.json`; helpers for lookup,
`next`, `writes`, `inject`, `caveman`, compound-key split/join. No pydantic (Decision 3).

**Requirements:** R2

**Dependencies:** vibe-cli/1

**Files:** `cli/src/vibe/machine.py`, `cli/tests/test_machine.py`

**Test scenarios:**
- Loads all 15 states; `flows`/`phases`/`modifiers` present; missing/invalid file raises typed error
- Compound-key: `idle` bare; `feature.impl` splits; unknown key rejected; `amend` present but not a legal cursor target
- Importing `machine` pulls no `typer`/`rich`/`pydantic`

---

### vibe-cli/3 — cursor.py (read + atomic write, set-state semantics)

**Goal:** read `state.json`; atomic write (temp + `os.replace`); port `set-state.sh` —
feature carry-forward, `idle` clears feature, `amend` rejected, `updated` stamped. Stdlib-only import.

**Requirements:** R2

**Dependencies:** vibe-cli/2

**Files:** `cli/src/vibe/cursor.py`, `cli/tests/test_cursor.py`

**Test scenarios:**
- `feature.impl foo` then `feature.verify` (no feature) preserves `foo`; `idle` clears to null
- Atomic write (no partial file on simulated failure); `amend` target rejected
- Importing `cursor` pulls no typer/rich/pydantic

---

### vibe-cli/4 — policy.py (allow/warn/block + 3 hard blocks) + parity

**Goal:** stdlib-only `decide(path, state)` → `allow`/`warn:<r>`/`block:<r>`; 3 hard blocks
verbatim; current-state resolved via `cursor.py` when state omitted.

**Requirements:** R6

**Dependencies:** vibe-cli/2, vibe-cli/3

**Files:** `cli/src/vibe/policy.py`, `cli/tests/test_policy.py`, `cli/tests/test_parity_policy.py`

**Test scenarios:**
- lessons.md blocked outside `*.compound`; root specs blocked outside `strategy.spec`/`feature.compound`/`setup.apply`; state.json direct-edit blocked; src/tests warn outside impl; else allow
- **Parity:** state × path matrix, `policy.decide` == `detect-context.sh decide` byte-for-byte (skips if no jq/bash)
- Importing `policy` pulls no typer/rich/pydantic

---

### vibe-cli/5 — orders.py (D12 resolution) + parity

**Goal:** resolve orders: bundled `_assets/skills/vibe/SKILL.md` `vibe:orders:<state>` block →
`machine.inject` → generic fallback; `<feature>` interpolation only. Stdlib-only import.

**Requirements:** R2

**Dependencies:** vibe-cli/2, vibe-cli/3, unit-1 vendored vibe skill tree

**Files:** `cli/src/vibe/orders.py`, `cli/tests/test_orders.py`, `cli/tests/test_parity_orders.py`

**Test scenarios:**
- Marker block extracted + feature-interpolated; machine-inject fallback for skill-less states; generic last resort
- Call-site read location pinned: bundled `_assets` copy for parity/inject; document target `.claude/skills/vibe/SKILL.md` at hook time
- **Parity:** output == `orders.sh` for every state (± feature) (skips if no jq/bash)

---

### vibe-cli/6 — status / next / go commands

**Goal:** `vibe status` (rich panel: flow/phase/feature + legal next), `vibe next` (list),
`vibe go <state> [--feature F]` (legality-checked via cursor.py). Exports `register(app)`.

**Requirements:** R2

**Dependencies:** vibe-cli/2, vibe-cli/3

**Files:** `cli/src/vibe/commands/flow_cmds.py`, `cli/tests/test_flow_cmds.py`

**Test scenarios:**
- status at `feature.impl` shows cursor + legal next (R2-S1)
- `vibe go feature.verify` from `feature.design` refused, legal options named, cursor unchanged (R2-S2)
- legal `go` transitions and stamps

---

### vibe-cli/7 — hook.py entry (vibe-hook inject|guard|gate) + self-location

**Goal:** stdlib `argparse` second entry. `inject` prints orders; `guard` reads stdin JSON →
`policy.decide` → **exit 2 on block**; `gate` warn-only. Project self-located via
`CLAUDE_PROJECT_DIR` then upward `.spec`/`.git` marker walk (no `CLAUDE_PLUGIN_ROOT`).

**Requirements:** R6, R1 (the binary the hooks invoke)

**Dependencies:** vibe-cli/4, vibe-cli/5

**Files:** `cli/src/vibe/hook.py` (fill stub), `cli/tests/test_hook.py`, `cli/tests/test_hook_importcost.py`

**Test scenarios:**
- `guard` on a hard-block path exits 2 (reason on stderr); on allow exits 0 silent; `inject` emits orders; `gate` exit 0
- **Self-location:** `CLAUDE_PROJECT_DIR` set → resolved from it; unset → marker walk finds root; **path-parity** — byte-identical verdict via real path and via a symlinked invocation (standing lesson)
- **Import-cost:** importing `vibe.hook` loads NO `typer`/`rich`/`pydantic` (assert via `sys.modules`)

---

### vibe-cli/8 — provision/agents_md.py (AGENTS.md marker-merge) + adapter link

**Goal:** port `merge-agents.sh` using shared `markers.py`: 5 merge cases, `unmerge`, adapter
`link`. Preserve user prose outside markers.

**Requirements:** R1, R4

**Dependencies:** vibe-cli/1 (markers.py, bundled template + adapters.json)

**Files:** `cli/src/vibe/provision/agents_md.py`, `cli/tests/test_agents_md.py`

**Test scenarios:**
- No file → copy template; markers → replace; legacy `vibe:constitution` → migrate; no-markers+divergent → append (warn, no clobber)
- `unmerge` removes block, prose survives; reversed markers → error
- **link:** fresh symlink created; already-correct → no-op; existing real file → refused (not clobbered)

---

### vibe-cli/9 — provision/settings.py (.claude/settings.json hook merge)

**Goal:** idempotent merge of the three hook entries (keyed by event + command); never
rewrite unrelated user hooks; inverse `unmerge`.

**Requirements:** R1

**Dependencies:** vibe-cli/1

**Files:** `cli/src/vibe/provision/settings.py`, `cli/tests/test_settings.py`

**Test scenarios:**
- **Full contract (byte-for-byte per tech.md):** all three events (UserPromptSubmit/PreToolUse/Stop), PreToolUse matcher `Edit|Write|NotebookEdit`, exact commands `vibe-hook inject|guard|gate`, `timeout:10`
- Re-merge is a no-op (idempotent); pre-existing unrelated user hook preserved; `unmerge` removes only vibe's entries

---

### vibe-cli/10 — rules.py (active-rules digest) + vibe rules + parity

**Goal:** port `regen-active-rules.sh` using shared `markers.py`: top-5 lessons, pinned-first
then recent, into `vibe:active-rules` of CLAUDE.md/AGENTS.md; symlink-aware, dedup. Thin
`vibe rules` command exporting `register(app)`.

**Requirements:** R1, R4

**Dependencies:** vibe-cli/1 (markers.py)

**Files:** `cli/src/vibe/rules.py`, `cli/src/vibe/commands/rules_cmd.py`, `cli/tests/test_rules.py`, `cli/tests/test_parity_rules.py`

**Test scenarios:**
- Digest capped at 5, pinned first, 📌; block replaced in place; symlinked CLAUDE.md→AGENTS.md written once
- **Parity:** block output == `regen-active-rules.sh` for a sample lessons.md (skips if no bash)

---

### vibe-cli/17 — provision/skills.py (copy engine)

**Goal:** copy the bundled `_assets/skills/{spec,vibe}/` trees into a target's
`.claude/skills/{spec,vibe}/` (leaving `.agents/` canonical untouched); the per-file inverse
`remove` for uninstall (surgical). Reusable leaf, independently tested.

**Requirements:** R1, R4

**Dependencies:** vibe-cli/1

**Files:** `cli/src/vibe/provision/skills.py`, `cli/tests/test_skills.py`

**Test scenarios:**
- Fresh target gets both skill trees under `.claude/skills/`; re-copy idempotent
- `remove` deletes only the shipped files (per-file inverse), prunes emptied dirs, leaves a co-located user file

---

### vibe-cli/11 — vibe init (orchestration only)

**Goal:** one command orchestrating the leaves: PATH prereq check (+ ephemeral warn), call
`skills`(17)/`settings`(9)/`agents_md`(8)/`rules`(10)/`plugins`(12), seed cursor via
`cursor.py` (3) + gitignore, print summary. `--yes`, `--only spec`, `--dry-run`. Exports `register(app)`.

**Requirements:** R1, R6

**Dependencies:** vibe-cli/3, /8, /9, /10, /12, /17

**Files:** `cli/src/vibe/commands/init_cmd.py`, `cli/tests/test_init.py`

**Test scenarios:**
- Fresh `--yes` fully provisioned: skills, 3 settings.json hooks, AGENTS.md block, gitignored cursor, summary (R1-S1)
- **Ephemeral** invocation (binaries not on persistent PATH) → warns, no silent no-op (R6-S2)
- `--only spec`: spec skill only, no hooks/cursor/plugins
- **`--dry-run`** on fresh target: ZERO filesystem writes, summary still printed
- **Hook-contract proxy (R1-S2):** the written settings.json command strings equal the subcommands `hook.py` implements

---

### vibe-cli/12 — provision/plugins.py + vibe plugins

**Goal:** orchestrate `claude plugin marketplace add` + `claude plugin install <p>@<mkt>
--scope project` from bundled `deps.json`; graceful when `claude` absent (print manual
commands). `plugins_cmd.py` exports `register(app)` for `vibe plugins list|install|add`.

**Requirements:** R1

**Dependencies:** vibe-cli/1 (deps.json asset)

**Files:** `cli/src/vibe/provision/plugins.py`, `cli/src/vibe/commands/plugins_cmd.py`, `cli/tests/test_plugins.py`

**Test scenarios:**
- `claude` present (mocked): install invoked per dep with correct args
- `claude` absent: no crash; prints the manual commands
- `vibe plugins list` reads deps.json

---

### vibe-cli/13 — doctor.py + vibe doctor

**Goal:** rich health — skills, `.claude/settings.json` hook entries (NOT legacy hooks.json),
cursor validity, AGENTS.md block (via markers.py), dependency plugins (via deps.json data) —
each degraded check paired with a fix hint. Warn-only default; `--exit-code` nonzero on
failure. Exports `register(app)`.

**Requirements:** R3

**Dependencies:** vibe-cli/2, /3, /9, markers.py, deps.json (data)

**Files:** `cli/src/vibe/doctor.py`, `cli/src/vibe/commands/doctor_cmd.py`, `cli/tests/test_doctor.py`

**Test scenarios:**
- Missing dep → warn names dep + degrade + fix cmd, exit 0; `--exit-code` nonzero (R3-S)
- A missing settings.json hook entry (and separately an absent AGENTS.md block / invalid cursor) → warn + fix hint
- Healthy install → all ok, exit 0 both modes

---

### vibe-cli/14 — vibe uninstall + update (surgical, cursor-safe)

**Goal:** `uninstall` removes only init's artifacts (per-file inverse via the leaves' `remove`),
preserving `.spec/`, prose, cursor (unless `--yes`); `update` re-provisions idempotently,
preserving the live cursor and prose. Exports `register(app)`.

**Requirements:** R4

**Dependencies:** vibe-cli/11

**Files:** `cli/src/vibe/commands/lifecycle_cmd.py`, `cli/tests/test_lifecycle.py`

**Test scenarios:**
- **Discriminating uninstall:** user file in each shared dir survives AND shipped files gone (fails vs a blanket `rm`); **`.spec/` untouched**; **cursor preserved without `--yes`, removed with `--yes`**; AGENTS.md prose outside markers survives
- **Cursor survives update:** target at `feature.impl foo` → `vibe update` refreshes managed files, cursor still `feature.impl foo`, prose untouched

---

### vibe-cli/15 — vibe spec wrapper + check/orders inspection

**Goal:** `vibe spec validate`/`setup` shell to the spec scripts (target `.claude/skills/spec/`
first, else bundled `_assets/skills/spec/`), needing no flow runtime; `vibe check <path>` and
`vibe orders` expose policy/orders verdicts. Exports `register(app)`.

**Requirements:** R5

**Dependencies:** vibe-cli/4, /5, bundled spec tree

**Files:** `cli/src/vibe/commands/spec_cmd.py`, `cli/tests/test_spec_cmd.py`

**Test scenarios:**
- `vibe spec validate` result equals running `validate.sh` directly, succeeds with no cursor/hooks present (R5-S)
- `vibe check .spec/lessons.md` at `idle` prints the block verdict; `vibe orders` prints current orders

---

### vibe-cli/16 — Wire app.py + full asset-sync + retire legacy

**Goal:** `app.py` imports and calls every `register(app)`; whole-tree `_assets/` drift guard;
migration note (README + MIGRATION) → `vibe init`/`vibe update`; mark bash flow scripts +
`install.sh` deprecated per D4.

**Requirements:** — (D4)

**Dependencies:** all prior units

**Files:** `cli/src/vibe/app.py` (final wiring), `cli/tests/test_assets_sync.py`, `README.md`, `cli/README.md`, migration note

**Test scenarios:**
- Full `vibe --help` lists every group; every `register` imported and invoked
- **Full-tree sync:** every file under `_assets/` byte-identical to its `flow/`/`spec/` source; the two `state-machine.json` copies (flat `_assets/state-machine.json` for machine.py and `_assets/skills/vibe/state-machine.json` for the bundled tree) both match the single `flow/state-machine.json` source
- Full suite green; parity suites (4, 5, 10) green — the retirement gate
