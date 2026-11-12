# AGENTS.md — vibe kit Engineering Guide
<!-- vibe-spec: literal-file -->

> This repository builds vibe kit itself. The sections below are this repo's own
> dogfood guide (user-owned prose). The generic vibe-flow contract that ships to
> every install target lives in the managed `vibe:instructions` block further
> down — it is byte-identical to `flow/reference/templates/AGENTS.md` and is
> replaced on every `setup.apply` / install, so edit the template, not the block.

**Repository:** vibe — self-hosting bash/Markdown/JSON workflow harness (in active build).
**Canonical:** this file. `CLAUDE.md` symlinks here.

## Prime Directive

**Spec first. Always.**

1. **ASK** — clarify requirements; no assumptions.
2. **PLAN** — break down, read `docs/spec/`, present approach.
3. **CONFIRM** — approval happens at the two human gates (plan→impl, verify→ship), not before every action.
4. **EXECUTE** — implement step-by-step; verify before claiming done.

- MUST read `docs/spec/` before writing code.
- MUST NOT write code without an approved plan or spec.
- MUST NOT invent filenames or state files the repo does not have (see **Dogfood status**).

The **vibe flow** (state machine, hooks, per-turn routing) is what this repo is building.
It is **not** a prerequisite for doing work here. Follow the **Working model** in the managed
block below unless your task is explicitly to implement flow machinery.

## Dogfood status

Know what exists before you read or create files.

| Path | Status |
|---|---|
| `docs/spec/` | **Live** — primary source of truth; read every session |
| `.agents/skills/spec/` | **Live** — spec skill; validation runs `vibe-spec` (`crates/vibe-spec/`) |
| `AGENTS.md` / `CLAUDE.md` | **Live** — this guide; `CLAUDE.md` → `AGENTS.md` |
| `.agents/skills/flow/state-machine.json` | **Present** — static flow definition (data to edit when building flow) |
| `.agents/skills/flow/scripts/` | **Present** — `set-state.sh`, `detect-context.sh`, etc. |
| `.agents/skills/flow/state.example.json` | **Present** — template for the cursor file |
| `.vibe/run/flow/state.json` | **Primary** — gitignored runtime cursor; absence means idle before explicit v2 adoption |
| `.agents/skills/flow/evidence/` (`flow/evidence/`) | **Often absent** — gitignored `*.verify` receipts (`feature-<feature>.md`, `quick.md`); written during verify, missing is normal |
| `.agents/skills/flow/` | **Present** — workflow router, progressive procedures and agent briefs |
| `.agents/skills/vibe/` | **Present** — toolkit orientation guide |
| `.claude/commands/`, `.claude/hooks/`, `.claude/settings.json` | **Present** — Claude Code adapter (`/flow` command + hook scripts wired via `settings.json`) |
| **`flow.json`** | **Does not exist** — never expect, read, or create this file |
| Per-turn inject / D12 orders-in-skills | **Live** — orders sourced from linked `flow` skill via `orders.sh` |

When in doubt, read [docs/spec/plan.md](docs/spec/plan.md) for spec-vs-repo gaps.

## Task routing

Route by **intent**, not by flow cursor.

| Intent | Load first | Write surface |
|---|---|---|
| Understand the project | `docs/spec/product.md`, `docs/spec/tech.md` | — |
| Build a named feature | `docs/superpowers/specs/*-<feature>-design.md` + `docs/superpowers/plans/*-<feature>.md` (default); `docs/spec/features/<feature>/` only as the fallback | per feature Scope |
| Small bounded fix | the relevant feature spec; a quick fix needs no new doc | minimal |
| Spec / plan work | `.agents/skills/spec/SKILL.md` | `docs/spec/**` per write rules |
| Build flow machinery | `docs/spec/tech.md` + `.agents/skills/flow/` | `.agents/skills/flow/`, `flow` skill |
| Build adapters / hooks | `docs/spec/tech.md` + `.claude/` | `.claude/`, `install.sh` |
| Build AGENTS.md provisioning | `flow/reference/templates/AGENTS.md` | templates, merge scripts |
| Set up or repair harness | `.agents/skills/flow/SKILL.md` | `.agents/**`, managed blocks |

`flow` skill phases are **helpers** for their domains. Read the matching feature spec first;
the skill does not override `docs/spec/`.

## Target harness

This is the **end-state** the repo is building. Reference the root
[product.md](docs/spec/product.md) and [tech.md](docs/spec/tech.md) — not for ordinary feature work.

- **Cursor:** `.vibe/run/flow/state.json`; legacy v1 is `{flow, phase, feature, updated}`.
  Explicit v2 adoption is never automatic. Keep runtime tests in disposable fixtures.
- **Machine:** `.agents/skills/flow/state-machine.json` — static states, skills, legal `next`.
- **Transitions:** legacy v1 uses `bash .agents/skills/flow/scripts/set-state.sh <flow.phase> [feature]`;
  adopted v2 uses `vibe-flow state transition`. Never edit `state.json` by hand.
  Production trusted approval/evidence checks are unavailable/deny.
- **Routing:** per-turn orders live in the `flow` skill (D12, `## Orders` in `SKILL.md`); the machine
  holds the `skill` link, not prose. `orders.sh` resolves the current state's orders.
- **Adapters:** hooks read `.agents/skills/flow`, not `.claude/state.json`.

`.claude/` is a runtime adapter — not canonical.

## Repo layout

```text
skills/vibe/           # toolkit orientation guide
flow/                  # flow skill source: router, procedure files, machine, scripts, engine, content
spec/                  # spec skill source: guides, reference, agents, tests
crates/                # Rust workspace: vibe-core, vibe-spec, vibe-parity
contracts/             # schemas and golden fixtures
oracles/               # frozen JS and bash oracles — never edited
tests/                 # umbrella suite, parity corpus, skills lint
docs/spec/             # durable memory (hubs, lessons.md, kind folders, features/)
docs/superpowers/      # superpowers-layer specs and plans (the default feature layer)
.agents/skills/        # compatibility symlinks to spec/ and flow/
AGENTS.md              # this file (canonical)
CLAUDE.md              # symlink → AGENTS.md
```

## Conventions

- Bash MUST use `set -euo pipefail`; MUST be shellcheck-clean.
- Scripts MUST be deterministic and idempotent. Optional diagnostics degrade with warnings;
  required runtime authority and integrity checks refuse when unavailable.
- State machine is **data** — edit `state-machine.json`, not prose duplicates.
- Paths and commands stay byte-exact.
- Prefer editing existing files. Do not create files without necessity.
- Do not add comments that narrate the obvious.

## Boundaries

**Always**
- Read `docs/spec/` before code.
- Run `vibe-spec validate --strict` before claiming work is done.
- Cite plan unit IDs in tests and commits during impl.
- Respect feature **Scope** tables — do not implement another feature's units.

**Ask first**
- Root spec edits outside strategy / compound / setup.
- Scope escalation (small fix → named feature).
- Gated edges (plan→impl, verify→ship) and quick→feature escalation; other transitions auto-advance.
- Adding dependencies or deleting files.

**Never**
- Expect or create `flow.json`.
- Treat missing `state.json` as an error for normal work.
- Edit `state.json` by hand.
- Edit inside `<!-- vibe:active-rules:* -->` markers.
- Clobber user-owned content outside managed blocks.
- Treat `.claude/` as canonical.

## Commits

Conventional Commits. Imperative, lowercase, ≤50-char subject, no trailing period.

```text
feat(flow): add inject hook wiring
fix(spec): correct feature frontmatter check
docs(agents): rewrite instruction set
```

## Spec layout

Hubs: `docs/spec/{product,tech,design,plan}.md`; each lists its notes in `children:`.
`docs/spec/lessons.md` is the append-only lessons file.
Kind folders hold the notes: `areas/`, `concepts/`, `decisions/`, `parts/`,
`platform/`, `questions/`, `tech/`.
Fallback features: `docs/spec/features/<feature>/{product,tech}.md` required;
`design.md`, `plan.md`, `research.md` optional.

<!-- vibe:instructions:start -->
<!-- Managed by vibe (merge-agents.sh); replaced on setup.apply / install.
     Content outside these markers, including vibe:active-rules, is user-owned. -->
This project uses **Vibe Flow**. `vibe` explains the toolkit, `flow` owns the lifecycle,
and `spec` handles project knowledge. Direct Flow and Spec entry need no bootstrap.
## Session start
1. Read the spec root's `lessons.md` and `plan.md` when present. A flow-only install has no spec tree.
2. Read the active design and plan in `docs/superpowers/`; fallback: `docs/spec/features/<name>/`.
3. Read `.vibe/run/flow/state.json` first; otherwise use the supported legacy cursor. Only absence means idle. Adopted v2 state never falls back to legacy after corruption or removal. Load `flow` and its current orders; report corrupt state for explicit recovery.
## Working model
`design → plan →[gate] impl → compound → verify →[gate] ship`
Get explicit approval before crossing: plan → impl, and verify → ship.
Freeze approved design/plan; keep progress in runtime records. Workers receive
bounded scopes/checks and return results; the controller owns state and independent review.
Compound prepares additive docs and lessons before final checks. Preserve feature artifacts.
Never infer permission to commit, push, merge, release, delete artifacts or clean up.
## Runtime and policy
Gates are a convention today for legacy v1: `set-state.sh` writes the cursor; it does not gate.
Explicit v2 adoption enables legal-edge/revision checks and immutable records.
Use `vibe-flow state transition` for adopted runs; legacy writers refuse them.
Production trusted approval/evidence checks remain unavailable/deny. A `confirm`
token, hook registration or model-written receipt proves neither human origin nor readiness.
Follow current-state orders from `flow`; delegates supply bounded methods.
Never hand-edit state. Explicit abort is not completion or ship authorization.
Restricted paths are data in `content/policy.json`; inspect `detect-context.sh decide` before writes.
Edit blocks/config, not generated regions. Standing prompt/rules channels use root `vibe.json`.
## Commands
```bash
vibe-spec validate # before claiming done; absent on --only flow
bash .agents/skills/flow/scripts/detect-context.sh decide <path>
bash .agents/skills/flow/scripts/doctor.sh
node .agents/skills/flow/engine/cli.mjs render --check
node .agents/skills/flow/engine/cli.mjs render agents-md --write
```
## Enforcement limits
Legacy edit guards and receipt freshness do not cover every shell, interactive
input or subagent action. Native complete blocking is unverified for Claude Code,
OpenCode and Codex. Strong checks stay unavailable/deny; configured hooks are not proof.
Task results and command-output metadata remain claims until independently checked.
Candidate changes require fresh final verification/review and approval of the named action.
**Output: brief technical English** — answer first, one idea per sentence. Keep security warnings,
irreversible-action confirmations, code, paths and commands byte-exact.
<!-- vibe:instructions:end -->

<!-- vibe:active-rules:start -->
<!-- Generated from the spec lessons.md by regen-active-rules.sh. Do not edit by hand;
     edit lessons.md and re-run during compound. Top 5, pinned first. -->

### Active Rules

- 📌 **A guard is only as strong as the capability it bans, not the spelling it matches** — When a guard is evaded, do not add a pattern for the evasion — that buys exactly one round. Close the *capability*: state the invariant as "no module may obtain X by any means unless allowlisted", then find the mechanical property that makes it true regardless of syntax. Match against the whole comment-stripped file, never per line. Close the scanned set under whatever relation the attacker can traverse (here, module resolution — which covers static `import`, `import()`, `require`, and `createRequire` without naming any of them). Pin exemptions by exact line *and occurrence count*, never by file. And require the implementer to produce the list of evasions it tried against its **own** fix, including the ones that failed — the round that finally held was the first to produce that artifact.
- 📌 **Porting a script destroys the oracle that proves the port — freeze it first** — When replacing an implementation, copy its predecessor into the test tree as a frozen oracle **before** the first line of the replacement is written, and make differential comparison part of the suite. Prioritise by blast radius: anything that can block, delete, or halt gets its oracle frozen first. Where an oracle disagrees with *itself* across its own code paths, follow the fail-safe branch and pin the divergence with a control — losing one turn of enforcement is recoverable, wedging a session is not.
- **Document capability separately from installed plugins** — Discover the actual method mapping and installer catalog before documenting a workflow. Separate method skills, optional native agents, generic role briefs, installed hooks and verified capabilities. Label hypothetical paths and unavailable gates in diagrams, verify generated documentation copies, and refresh remote state before publishing.
- **Keep test inputs proportional to the change** — Keep focused executable regressions and remove added snapshots with no consumers. Generate repetitive compatibility projects in owned temporary storage; compare the full generated matrix before changing storage. Preserve existing historical fixtures and original oracles, use supported commands with safe quoting, and bind review to the reduced diff.
- **Preserve compact formatting before fixture generation** — Preserve the canonical compact JSON style before regenerating copies. Compare ordered parsed values for valid JSON, exact bytes for intentionally invalid JSON and other files, and the complete case inventory. Run generator consistency and behavioral parity separately; use task-specific shell variables and report baseline failures without changing assertions to hide them.
<!-- vibe:active-rules:end -->

<!-- vibe:rules -->
_Managed by vibe — rendered from the `agents-md` channel by `node .agents/skills/flow/engine/cli.mjs render agents-md --write`. Edit the blocks or `vibe.json`, not this region._

## Write invariants

Every write this harness restricts is one rule in `content/policy.json`. The
list below is RENDERED from that file — the same data
`detect-context.sh decide` enforces — so the prose and the enforcer cannot
disagree. Edit the rules, never this text.

- `.vibe/run/flow/state.json`, `.agents/skills/flow/state.json`, `.agents/skills/vibe/state.json`, `.agents/flow/state.json` — otherwise: block — state.json is written only via set-state.sh, never by direct edit
- `.spec/lessons.md` — `feature.compound`, `setup.apply`, `strategy.spec`, `quick.verify`: allow; otherwise: block — .spec/lessons.md is writable only during feature.compound, setup.apply, strategy.spec, or quick.verify (current: the current state)
- `docs/spec/lessons.md` — `feature.compound`, `setup.apply`, `strategy.spec`, `quick.verify`: allow; otherwise: block — docs/spec/lessons.md is writable only during feature.compound, setup.apply, strategy.spec, or quick.verify (current: the current state)
- `.spec/product.md`, `.spec/tech.md`, `.spec/design.md`, `.spec/plan.md` — `strategy.spec`, `feature.compound`, `setup.apply`: allow; otherwise: block — root .spec specs are writable only during strategy.spec, feature.compound, or setup.apply (current: the current state)
- `docs/spec/product.md`, `docs/spec/tech.md`, `docs/spec/design.md`, `docs/spec/plan.md` — `strategy.spec`, `feature.compound`, `setup.apply`: allow; otherwise: block — root docs/spec specs are writable only during strategy.spec, feature.compound, or setup.apply (current: the current state)
- `.planning/*` — `idle`: allow; otherwise: block — .planning/ is GSD's state store — vibe reads it as context and never writes it; route edits through GSD's own commands
- `.spec/features/*` — `feature.impl`, `quick.fix`: warn — .spec/features edits are frozen during impl/fix — route back to feature.design/plan to change scope (current: the current state); otherwise: allow
- `docs/spec/features/*` — `feature.impl`, `quick.fix`: warn — docs/spec/features edits are frozen during impl/fix — route back to feature.design/plan to change scope (current: the current state); otherwise: allow
- `docs/superpowers/specs/*` — `feature.impl`, `quick.fix`: warn — superpowers design docs are frozen during impl/fix — route back to feature.design to change scope (current: the current state); otherwise: allow
- `CLAUDE.md`, `AGENTS.md` — otherwise: warn — CLAUDE.md/AGENTS.md active-rules block is generated by regen-active-rules.sh; edits inside the markers are overwritten next compound
- `src/*`, `tests/*` — `feature.verify`: warn — verify writes no src — route findings back to impl (set-state.sh feature.impl); `quick.verify`: warn — verify writes no src — route findings back to fix (set-state.sh quick.fix); `feature.impl`, `quick.fix`, `setup.apply`: allow; otherwise: warn — source/test edits outside an impl/fix state (current: the current state)

Check a path before writing it:

```bash
bash .agents/skills/flow/scripts/detect-context.sh decide <path>
```

## Delegating to sub-agents

Choose the needed capability before its provider or model. Apply controlling user
and repository model preferences. Set an allowed available model explicitly for
all delegated work; do not infer one from the role name. Report unavailable
required capabilities instead of silently substituting a forbidden model.

Workers receive exact scope, dependencies and checks. The controller owns state,
progress, gate decisions and independent review. A worker returns checked-result
claims; it cannot widen scope, approve itself, commit, clean or finish a branch.

## Coordinating independent work

Use the host's available orchestration only when authorized by the controlling
instructions. If orchestration requires opt-in, propose its scope and cost and
wait for the user's reply. A workflow tool is optional; it does not replace the
controller, authorization gates or independent review.

Parallelize independent work on disjoint files when permitted. Serialize shared
files and dependent tasks. Supply the same bounded TaskBrief and TaskResult
contract to native named agents or generic agents. Preserve controlling model
preferences on every dispatch. Missing required independent review blocks readiness.

## Delegation policy (maintainer)

Maintainer rules on top of the shipped tiers.

- `fable` — rare; only when independence from your context is the point (e.g. adversarial review of your own plan or a large diff). ALWAYS check with the maintainer first — never spawn one unprompted.
- Never use `fable` agents in a dynamic workflow, not even with approval. A Fable review happens AFTER the workflow completes, as a standalone Agent-tool call (ask first).
- If ultracode is NOT on for the session (no "ultracode" keyword, no toggle, no orchestration request in the maintainer's own words), plan first — propose the workflow with rough shape and cost, and wait for the maintainer's reply; their "yes" is the opt-in. If ultracode IS on, invoke directly.
<!-- /vibe:rules -->

<!-- vibe-spec:router:start -->
Spec owns where work is recorded; superpowers process skills own how it is done.
Before changing behaviour, structure or docs, invoke the `spec` skill.
It routes the work to the spec root (`vibe-spec root` prints it).
Check the tree with `vibe-spec validate`.
If `vibe-spec` is not on PATH, build it (`cargo build -p vibe-spec`) or see the spec skill.
<!-- vibe-spec:router:end -->

<!-- coding-environment:start -->
## Coding environment (applies across /home/agent/projects)

This repo is one of the repos in the shared coding environment at `/home/agent/projects`.
**Read `/home/agent/projects/AGENTS.md` before you touch code — it is not optional.** It carries
the cross-repo rules: work inside a repo (never in a Paseo worktree), spec first with its two
human gates, never push to a default branch or open a PR unless explicitly asked, verify before
claiming, and never commit secrets. Where that file and this one disagree, this repo's rules win.
<!-- coding-environment:end -->
