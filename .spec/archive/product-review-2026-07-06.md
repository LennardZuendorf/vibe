---
type: brief
scope: review
audience: external reviewing agent
updated: 2026-07-06
---

# vibe — Review Brief (release line)

Brief for another agent to **review + judge** vibe: what it claims, how it works,
whether it delivers, where it drifts. Caveman-lite prose (compressed, no filler);
paths / state names / commands stay byte-exact. Read this whole file, then run the
verify block, then judge against the dimensions at the end.

---

## Review target

- **Branch:** `chore-release-polish` @ commit `2f55a37` (worktree
  `~/Development/vibe-shell-core`). Open as **PR #13**.
- **This is the pure-bash release candidate.** ONE implementation — bash +
  Markdown + JSON. No python CLI, no `cli/` dir, no dual state. Judge this as the
  shipping product, end to end.
- **Sibling branch** `feat-cli-port` carries an in-progress python CLI port
  (built, unwired) on top of the same bash core. Out of scope here. It has its
  own review brief. If you compare: the branches diverged (this one uses the split
  test layout `spec/tests/` + `flow/tests/`; the other keeps `tests/{spec,flow,…}/`
  and adds `cli/`). Neither is a superset of the other.

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
symlinks → those dirs. `install.sh` dereferences symlinks (`cp -RL`) into real
dirs in a target repo. Co-located `tests/` + contributor `AGENTS.md` are pruned on
install (source-only, never ship).

**Design intent:** separate durable memory (`.spec/`) from runtime state
(`.agents/skills/vibe/`). Specs = memory; flow = runtime. Adapters (`AGENTS.md`,
`.claude/**`) are thin, never canonical.

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
shell over `flow/scripts/`). NOTE: no plugin — the specs still describe a
`.claude-plugin/plugin.json`, but the live wiring is `settings.json` (see drift):

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

**Verified live:** inject fires every turn (the `state=…` line atop each turn IS
the inject hook). Guard blocks a `state.json` hand-edit (exit 2), allows a normal
file (exit 0). A transition changes inject orders to the new state's write surface
+ `next`. `flow/scripts/doctor.sh` exits 0 (all green). So: hooks installed +
wired + firing, and flow is directed.

---

## How the spec framework works

Two layers. Root `.spec/{product,tech,design,plan,lessons}.md` = durable, current
only, no backlog/archaeology. Feature folders `.spec/features/<name>/` =
branch-scoped: written at design, consumed at impl, merged (cross-cutting) at
compound, then DELETED before branch merges. **Code is truth.**

This branch still carries its done-feature folders (vibe-flow, platform-adapters,
agent-instructions, install-tooling, release-docs) as living architecture docs —
a deliberate keep, referenced from root `product/tech/plan` frontmatter. (The
sibling branch pruned them; that is a branch choice, not a defect either way.)

- `product.md` + `tech.md` required per feature; `design`/`plan`/`research`
  optional. Requirement+Scenario format (RFC-2119 + Given/When/Then). Plan units =
  stable IDs (`U1`, `U2`…).
- Validate: `bash spec/scripts/validate.sh` — warn-first structural checks.

---

## Spec ↔ reality drift (judge from code; specs lag)

1. **Plugin vs settings.json.** Specs (`product.md`, `tech.md`, `plan.md`) describe
   a Claude Code plugin (`.claude-plugin/plugin.json`) bundling command+hooks.
   Reality: no `.claude-plugin/` dir; hooks wire via `.claude/settings.json` as
   plain bash scripts in `.claude/hooks/`. Live path has no plugin dependency.
2. **Test paths in older spec prose** may cite `tests/spec/run.sh`; this branch
   uses the split layout `spec/tests/run.sh`, `flow/tests/run.sh`,
   `flow/tests/adapters/run.sh`, dispatched by `tests/run.sh`.

Treat specs as **claims to verify**, not ground truth.

---

## Verify block (correct for THIS branch)

```bash
cd ~/Development/vibe-shell-core

# spec structural validation — expect 0 errors
bash spec/scripts/validate.sh

# all core suites via dispatcher
bash tests/run.sh
# or individually:
bash spec/tests/run.sh
bash flow/tests/run.sh
bash flow/tests/adapters/run.sh

# install health (warn-only, exits 0)
bash flow/scripts/doctor.sh
```

**Known results @ `2f55a37` (baseline for your judging):**

| Suite | Result |
|---|---|
| spec | 123 passed, 0 failed |
| flow | 68 passed, 0 failed |
| adapters | 74 passed, 0 failed |
| **total** | **265 passed, 0 failed** |
| validate | 0 errors, 7 warnings (release-docs scenario hints + doc-length; benign) |
| doctor | exit 0, all green |

Clean baseline: no failures. Judge coherence and design, not breakage.

---

## Honest tensions to weigh

1. **set-state.sh permissive-by-design.** Flow legality is soft (writer/gate split).
   Acceptable, or a hole (any agent can jump states)?
2. **guard strength.** Only 3 hard blocks; all `Stop` predicates warn-only
   ("earn-the-teeth" — teeth added after dogfooding proves them). Enough
   protection, or too permissive for a "guarded" harness?
3. **spec ↔ reality drift.** Specs describe a plugin that no longer exists. How
   much does stale planning memory cost a project whose pitch is *durable specs*?
4. **release readiness.** This is the public/release line (READMEs, LICENSE,
   CHANGELOG, CI, logo, examples, stranger-eval-gated). Is it actually shippable?
   What blocks a 0.1.0 tag?
5. **self-hosting claim.** Repo builds itself with its own harness. Does the
   evidence (dogfood tests, this very `.spec/` tree) support "self-hosting", or is
   it aspirational?

---

## Judging dimensions (score + evidence each)

1. **Delivers its claims?** Does the live bash flow do what `product.md` promises
   (separate memory/runtime, per-turn direction, guarded invariants, graceful
   degrade)? Cite verify evidence.
2. **Internally coherent?** Do specs, code, tests, docs agree? Where they disagree
   (plugin drift), is it honest lag or real breakage?
3. **Release quality.** READMEs, install/uninstall lifecycle, doctor, CI, trust
   rails — is this a credible public 0.1.0?
4. **Design quality.** Memory/runtime split, one-skill-many-phases, thin adapters,
   scripts-as-machinery — sound? Over/under-engineered for a one-developer tool?
5. **Gaps / risks.** What's missing, unverified, or fragile? What would you fix
   first before tagging a release?

Output: verdict per dimension + a short "fix-first" list.
