---
type: brief
scope: review
audience: external reviewing agent
updated: 2026-07-06
---

# vibe — Review Brief

Brief for another agent to **review + judge** vibe: what it claims, how it works,
whether it delivers, where it drifts. Caveman-lite prose (compressed, no filler);
paths / state names / commands stay byte-exact. Read this whole file first, then
run the verify block, then judge against the dimensions at the end.

---

## Review target (read this or you judge the wrong thing)

- **Branch:** `feat-cli-port` @ commit `197a24e` (worktree `~/Development/vibe`).
- **Dual state — the single most important fact:** this branch carries TWO
  implementations of the same harness side by side.
  1. **bash flow** — LIVE + wired + dogfooded. Hooks fire every turn. This is what
     actually runs. Judge this as "the product."
  2. **python CLI** (`cli/`) — BUILT but NOT wired. `vibe`/`vibe-hook` not on PATH;
     nothing calls it at runtime. It is an in-progress port, not the live path.
- Do NOT judge the two as one coherent shipping product. They are a live system +
  a parallel port mid-migration. Coherence between them is itself a review question.
- **Sibling branch:** `chore-release-polish` (PR #13, worktree
  `~/Development/vibe-shell-core`) = the pure-bash release candidate. Diverged, not
  a superset of this branch (different test layout, no `cli/`). Out of scope here
  unless you compare.

---

## What vibe is

Self-hosting spec + workflow harness for coding with agents. Bash + Markdown +
JSON, no runtime, no build step. Repo builds itself with its own harness. Two
halves that ship together but stand alone:

- **spec framework** (`spec/`) — durable `.spec/` planning layer: `product` /
  `tech` / `design` / `plan` / `lessons` + `features/<name>/`. Templates +
  warn-first validator. Works with any agent or none.
- **vibe flow** (`flow/`) — state-machine workflow for Claude Code. Routes each
  phase to the right skills/subagents, injects per-turn "orders", guards its own
  write invariants with hooks.

Canonical dirs = `spec/` + `flow/`. `.agents/skills/{spec,vibe}` are compat
symlinks → those dirs (portable runtime interface). `install.sh` dereferences
symlinks (`cp -RL`) into real dirs in a target repo.

**Design intent (product.md):** separate durable memory (`.spec/`) from runtime
state (`.agents/skills/vibe/`). Specs = memory; flow = runtime. Adapters
(`AGENTS.md`, `.claude/**`) are thin, never canonical.

---

## How the flow works

Cursor `.agents/skills/vibe/state.json` = `{flow, phase, feature}` points at one
state in `state-machine.json` (source of truth: each state's linked skill, caveman
level, write surface, legal `next`). 15 states:

```
idle
setup.detect  setup.apply
strategy.brainstorm  strategy.spec  strategy.compound
feature.design  feature.plan  feature.impl  feature.verify  feature.compound
quick.triage  quick.fix  quick.verify
amend            (modifier — edits scope from any state, returns there)
```

Everything starts `idle`. Agent self-locates, drives one flow. Human gates before
`feature.impl` and before ship (`feature.verify` → `feature.compound`).

**Transitions:** only via `bash flow/scripts/set-state.sh <flow.phase> [feature]`.
IMPORTANT nuance to judge: `set-state.sh` is the **writer, not the gate** (its own
header). It does NOT enforce legal `next` — a raw call can jump to any state.
Sequence legality is enforced by the `/flow` command + the agent honoring the
advertised `next`, not by the writer. So "flow direction" = soft (convention +
guard rails), not a hard rail on transitions.

**Three Claude Code hooks** (wired via `.claude/settings.json`, each a thin bash
shell over `flow/scripts/`):

| Hook | Event | Role | Enforcement |
|---|---|---|---|
| inject | `UserPromptSubmit` | emit current state's orders every turn (skill, write surface, caveman, next) | advisory — directs the agent |
| guard | `PreToolUse` (`Edit\|Write\|NotebookEdit`) | allow/warn/block via `detect-context.sh decide` | HARD — 3 invariants block (exit 2) |
| gate | `Stop` | warn-first exit-predicate checks | warn-only |

**Guard's 3 hard invariants** (`detect-context.sh decide`, exit 2): (1) `state.json`
only via `set-state.sh`; (2) `.spec/lessons.md` only in a `*.compound` state or
`setup.apply`; (3) root `.spec/{product,tech,design,plan}.md` only in
`strategy.spec` or `feature.compound`. Everything else allow/warn (the AGENTS.md
active-rules marker block is warn, not block).

**Verified live (this session):** inject fires every turn (the `state=idle · no
active flow…` line atop each turn IS the inject hook). Guard blocked a `state.json`
hand-edit (exit 2), allowed `README.md` (exit 0). `idle → feature.design`
transition changed inject orders to that state's write surface + `next`. Doctor
all-green. So: hooks installed + wired + firing, and flow is directed.

---

## How the spec framework works

Two layers. Root `.spec/{product,tech,design,plan,lessons}.md` = durable, current
only, no backlog/archaeology. Feature folders `.spec/features/<name>/` =
branch-scoped: written at design, consumed at impl, merged (cross-cutting) at
compound, then DELETED before branch merges. **Code is truth.**

- `product.md` + `tech.md` required per feature; `design`/`plan`/`research`
  optional. Requirement+Scenario format (RFC-2119 + Given/When/Then). Plan units
  = stable IDs (`U1`, `U2`…).
- Validate: `bash spec/scripts/validate.sh` — warn-first structural checks
  (Scope table, frontmatter, units, ID trace, link integrity SF13, child refs).

---

## Spec ↔ reality drift (judge from code, not specs — specs lag)

The `.spec/` docs describe an older design. Known lags as of `197a24e`:

1. **Plugin vs settings.json.** Specs describe a Claude Code plugin
   (`.claude-plugin/plugin.json`) bundling command+hooks. Reality: hooks wire via
   `.claude/settings.json` as plain bash scripts in `.claude/hooks/`. No plugin
   dependency at runtime.
2. **Test paths.** Specs cite `tests/spec/run.sh` etc. This branch: dispatcher
   `tests/run.sh` → `tests/{spec,flow,adapters,cli}/run.sh`. (Sibling branch
   `chore-release-polish` uses a different split: `spec/tests/`, `flow/tests/`.)
3. **python CLI in plan.md** listed as feature 9 "BUILT" + feature 10 PLANNED.
   Live path is still bash; CLI not wired.
4. Feature-spec folders for done features (vibe-flow, platform-adapters, etc.)
   were pruned 2026-07-06; root refs healed. Their durable record lives in
   `plan.md` "Delivered (history)". Truth for those = code, not spec.

Treat specs as **claims to verify**, not ground truth. Verified session facts (15
states, 3 wired hooks, suite counts below) are truth.

---

## Verify block (correct for THIS branch)

```bash
cd ~/Development/vibe

# spec structural validation — expect 0 errors
bash spec/scripts/validate.sh

# core bash suites (via dispatcher)
bash tests/run.sh

# install health (warn-only, exits 0)
bash flow/scripts/doctor.sh

# python CLI tests (the unwired port)
cd cli && uv run pytest        # or: pytest
```

**Known results @ `197a24e` (baseline for your judging):**

| Suite | Result |
|---|---|
| spec (bash) | 123 passed, 0 failed |
| flow (bash) | 68 passed, 0 failed |
| adapters (bash) | 71 passed, 0 failed |
| validate | 0 errors, 2 warnings (doc-length only, benign) |
| cli (pytest) | **464 passed, 2 FAILED** |

The 2 CLI failures are real drift signal, unrelated to today's spec edits (they
test asset-sync + bash↔python policy parity — untouched by the `.spec`/`AGENTS.md`
changes):

- `tests/test_assets_sync.py::test_every_bundled_asset_matches_its_source` — the
  CLI's vendored `cli/src/vibe/_assets/skills/{spec,vibe}` no longer byte-matches
  source `spec/` + `flow/`. The port's bundled copy drifted from the live harness.
- `tests/test_parity_policy.py::test_decide_matches_bash_origin` — python
  `policy.decide()` diverges from bash `detect-context.sh decide`. The two guard
  policies disagree.

Both confirm: **the python CLI mirrors an older harness state and has not been
reconciled with the current bash source.** Weigh accordingly.

---

## Honest tensions to weigh

1. **bash ↔ python duality.** Two harnesses, one repo. Live = bash. Port = python,
   drifted (2 parity/sync failures). Is the port on track, or an abandoned fork
   accumulating drift? What is the migration/cutover plan (see cli-restructure
   feature 10, PLANNED)?
2. **set-state.sh permissive-by-design.** Flow legality is soft. Acceptable
   (writer/gate split) or a hole (any agent can jump states)?
3. **spec ↔ reality drift.** Specs describe plugin + old test paths. How much does
   stale planning memory cost a "self-hosting" project whose pitch is durable specs?
4. **branch divergence.** feat-cli-port vs chore-release-polish diverged (test
   layout, cli/). Which is the release line? Merge story?
5. **guard strength.** Only 3 hard blocks; all Stop predicates warn-only
   ("earn-the-teeth" — teeth added after dogfooding proves them). Enough
   protection, or too permissive?

---

## Judging dimensions (score + evidence each)

1. **Delivers its claims?** Does the live bash flow do what product.md promises
   (separate memory/runtime, per-turn direction, guarded invariants, graceful
   degrade)? Cite verify evidence.
2. **Internally coherent?** Do specs, code, tests, and docs agree? Where they
   disagree, is it honest lag or real breakage?
3. **The port.** Is `cli/` a credible replacement or a liability? Judge the 2
   failures + migration plan.
4. **Design quality.** Memory/runtime split, one-skill-many-phases, thin adapters,
   scripts-as-machinery — sound? Over/under-engineered for a one-developer tool?
5. **Gaps / risks.** What's missing, unverified, or fragile? What would you fix
   first before calling this releasable?

Output: verdict per dimension + a short "fix-first" list. Judge the live bash flow
as the product; judge the python CLI as an in-flight port; keep them separate.
