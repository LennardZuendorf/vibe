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
| `.agents/skills/vibe/state-machine.json` | **Present** — static flow definition (data to edit when building flow) |
| `.agents/skills/vibe/scripts/` | **Present** — `set-state.sh`, `detect-context.sh`, etc. |
| `.agents/skills/vibe/state.example.json` | **Present** — template for the cursor file |
| `.agents/skills/vibe/state.json` | **Often absent** — gitignored runtime cursor; missing is normal |
| `.agents/skills/vibe/evidence/` (`flow/evidence/`) | **Often absent** — gitignored `*.verify` receipts (`feature-<feature>.md`, `quick.md`); written during verify, missing is normal |
| `.agents/skills/vibe/` | **Present** — workflow skill (SKILL.md router + phase files) |
| `.claude/commands/`, `.claude/hooks/`, `.claude/settings.json` | **Present** — Claude Code adapter (`/flow` command + hook scripts wired via `settings.json`) |
| **`flow.json`** | **Does not exist** — never expect, read, or create this file |
| Per-turn inject / D12 orders-in-skills | **Live** — orders sourced from linked `vibe` skill via `orders.sh` |

When in doubt, read [docs/spec/plan.md](docs/spec/plan.md) for spec-vs-repo gaps.

## Task routing

Route by **intent**, not by flow cursor.

| Intent | Load first | Write surface |
|---|---|---|
| Understand the project | `docs/spec/product.md`, `docs/spec/tech.md` | — |
| Build a named feature | `docs/superpowers/specs/*-<feature>-design.md` + `docs/superpowers/plans/*-<feature>.md` (default); `docs/spec/features/<feature>/` only as the fallback | per feature Scope |
| Small bounded fix | the relevant feature spec; a quick fix needs no new doc | minimal |
| Spec / plan work | `.agents/skills/spec/SKILL.md` | `docs/spec/**` per write rules |
| Build flow machinery | `docs/spec/tech.md` + `.agents/skills/vibe/` | `.agents/skills/vibe/`, `vibe` skill |
| Build adapters / hooks | `docs/spec/tech.md` + `.claude/` | `.claude/`, `install.sh` |
| Build AGENTS.md provisioning | `flow/reference/templates/AGENTS.md` | templates, merge scripts |
| Set up or repair harness | `.agents/skills/vibe/SKILL.md` | `.agents/**`, managed blocks |

`vibe` skill phases are **helpers** for their domains. Read the matching feature spec first;
the skill does not override `docs/spec/`.

## Target harness

This is the **end-state** the repo is building. Reference the root
[product.md](docs/spec/product.md) and [tech.md](docs/spec/tech.md) — not for ordinary feature work.

- **Cursor:** `.agents/skills/vibe/state.json` — `{flow, phase, feature, updated}`; create from
  `state.example.json` only when testing transitions.
- **Machine:** `.agents/skills/vibe/state-machine.json` — static states, skills, legal `next`.
- **Transitions:** only via `bash .agents/skills/vibe/scripts/set-state.sh <flow.phase> [feature]`;
  never edit `state.json` by hand.
- **Routing:** per-turn orders live in the `vibe` skill (D12, `## Orders` in `SKILL.md`); the machine
  holds the `skill` link, not prose. `orders.sh` resolves the current state's orders.
- **Adapters:** hooks read `.agents/skills/vibe`, not `.claude/state.json`.

`.claude/` is a runtime adapter — not canonical.

## Repo layout

```text
flow/                  # vibe skill source: router, procedure files, machine, scripts, engine, content
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
- Scripts MUST be deterministic, idempotent, graceful-degrade (warn, never hard-fail).
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
This project runs the **vibe flow**, a spec-first workflow harness. The spec root (`docs/spec/`, or legacy `.spec/`) is durable memory; sessions are ephemeral.
Read the specs before writing code. A flow-only install has no spec tree.
## Session start
1. Read the spec root's `lessons.md` and `plan.md` when present.
2. Load `docs/superpowers/specs/*-<name>-design.md` and `docs/superpowers/plans/*-<name>.md`; fallback: `docs/spec/features/<name>/{product,tech,plan}.md`.
3. Read `{flow, phase, feature}` from `.vibe/run/flow/state.json` when present, otherwise `.agents/skills/vibe/state.json`. Only absence means `idle`; report a present invalid cursor. Resume the linked `vibe` skill.
## Working model
`design → plan →[gate] impl → verify →[gate] compound`
Most transitions auto-advance. Two edges are **human gates**: get explicit
approval before crossing: plan → impl, and verify → ship. Other edges need no pause.
`/flow <flow.phase> [feature] [confirm]` carries the feature; literal `confirm`
crosses a gated edge. Never hand-edit `state.json`. The underlying command is
`bash .agents/skills/vibe/scripts/set-state.sh <flow.phase> [feature]`.
Gates are a convention today: `/flow` asks for a `confirm` token, and no hook or script checks it.
`set-state.sh` writes the cursor; it does not gate. No hook enforces the two approvals; honor them yourself.
## Precedence and policy
The cursor owns sequencing and artifact destinations; delegated skills own method. Follow current-state orders from the linked `vibe` skill, resolved by `orders.sh`,
over delegated prose or artifact paths. Scope edits stay in the current state's
write surface. Transition only via `set-state.sh`; `set-state.sh idle` always aborts.
Restricted paths and states are data in `content/policy.json`, enforced by
`detect-context.sh decide` and rendered rule-by-rule below. Absent cursor means idle.
Standing blocks compose the per-turn prompt, session start and `vibe:rules` channels. Configure them in root `vibe.json`; edit blocks/config, never generated regions.
## Commands
```bash
vibe-spec validate # before claiming done; absent on --only flow
bash .agents/skills/vibe/scripts/detect-context.sh decide <path> # before writes
bash .agents/skills/vibe/scripts/doctor.sh
node .agents/skills/vibe/engine/cli.mjs render --list
node .agents/skills/vibe/engine/cli.mjs render --check
node .agents/skills/vibe/engine/cli.mjs render agents-md --write
```
## Enforcement is partial — do not trust it blindly
The guard intercepts only `Edit`, `Write`, `MultiEdit`, and `NotebookEdit`.
Shell redirection (`>`, `tee`, `sed -i`) bypasses it; a warn-only Bash sniffer backs the convention. No hook enforces either human approval. The Stop receipt
tooth requires fresh evidence in verify only when platform hooks are wired
(e.g. `.claude/settings.json`). No wiring, no tooth.
Scripts graceful-degrade; `jq` is optional (pure bash keeps cursor bytes identical).
Other agents may edit concurrently: touch only your own files; prefer warn over hard-fail.
**Output: brief technical English** — answer first, one idea per sentence, active voice, no filler. Compress receipts/summaries; keep security warnings, irreversible-action
confirmations, code, paths and commands full and byte-exact.
<!-- vibe:instructions:end -->

<!-- vibe:active-rules:start -->
<!-- Generated from the spec lessons.md by regen-active-rules.sh. Do not edit by hand;
     edit lessons.md and re-run during compound. Top 5, pinned first. -->

### Active Rules

- 📌 **A guard is only as strong as the capability it bans, not the spelling it matches** — When a guard is evaded, do not add a pattern for the evasion — that buys exactly one round. Close the *capability*: state the invariant as "no module may obtain X by any means unless allowlisted", then find the mechanical property that makes it true regardless of syntax. Match against the whole comment-stripped file, never per line. Close the scanned set under whatever relation the attacker can traverse (here, module resolution — which covers static `import`, `import()`, `require`, and `createRequire` without naming any of them). Pin exemptions by exact line *and occurrence count*, never by file. And require the implementer to produce the list of evasions it tried against its **own** fix, including the ones that failed — the round that finally held was the first to produce that artifact.
- 📌 **Porting a script destroys the oracle that proves the port — freeze it first** — When replacing an implementation, copy its predecessor into the test tree as a frozen oracle **before** the first line of the replacement is written, and make differential comparison part of the suite. Prioritise by blast radius: anything that can block, delete, or halt gets its oracle frozen first. Where an oracle disagrees with *itself* across its own code paths, follow the fail-safe branch and pin the divergence with a control — losing one turn of enforcement is recoverable, wedging a session is not.
- **A block proof covers the complete output and the restricted runtime** — Validate the complete block grammar and allowed status, keep the shim within its declared runtime, and escape backslash and C0/DEL bytes in every diagnostic value, and pair real multiline producers with usage/crash and restricted-PATH controls. Assert binding content budgets in executable tests. Refuse a directory leaf before atomic cursor replacement. Update integration fixtures from canonical content rather than pinning an obsolete heading or repo-only configuration.
- **Capture CI diagnostics before interpreting them** — Select the failing job explicitly. Capture its log through `gh api --allow-escape-sequences repos/<owner>/<repo>/actions/jobs/<job-id>/logs`, retain the raw output, and strip ANSI escapes only in a separate reading copy. Check transport status and response format before parsing JSON; do not treat unavailable in-progress `gh run view` logs as passing evidence.
- **Keep frozen integrity separate from reviewed source divergence** — Skip freshness only for a bare oracle copy. Keep frozen bytes immutable and pin a reviewed divergence with both exact hashes, a known manifest path, and a reason. Reject malformed, duplicate, unknown, missing-source, and stale rows. Report byte-identical sources separately from accepted differences. A source-availability preflight does not make copying transactional.
<!-- vibe:active-rules:end -->

<!-- vibe:rules -->
_Managed by vibe — rendered from the `agents-md` channel by `node .agents/skills/vibe/engine/cli.mjs render agents-md --write`. Edit the blocks or `vibe.json`, not this region._

## Write invariants

Every write this harness restricts is one rule in `content/policy.json`. The
list below is RENDERED from that file — the same data
`detect-context.sh decide` enforces — so the prose and the enforcer cannot
disagree. Edit the rules, never this text.

- `.vibe/run/flow/state.json`, `.agents/skills/vibe/state.json`, `.agents/flow/state.json` — otherwise: block — state.json is written only via set-state.sh, never by direct edit
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
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>
```

## Delegating to sub-agents

Model tiers for ANY delegated work — Agent-tool calls and Workflow-script `agent()`
calls alike. Set the `model` parameter explicitly on every call; never omit it
(omission silently inherits the session model):

- `haiku` — mechanical bulk work: renames, boilerplate, format conversion, log triage.
- `sonnet` — default for well-specified implementation, exploration and per-task review.
- `opus` — architecture, adversarial or final review, genuinely tricky work:
  concurrency, subtle algorithms, gnarly debugging.

When unsure between tiers, pick the cheaper and escalate on failure.

## Dynamic workflows (Workflow tool)

Applies to ALL sessions, any model. Dynamic workflows do not need to be avoided —
reach for the Workflow tool when a task has 3+ independent parallelizable subtasks or
would benefit from a pipeline/judge panel.

Opt-in: unless your human partner already asked for orchestration in their own words,
plan first — propose the workflow in one or two sentences with the rough shape and
cost, and wait for their reply; their "yes" is the opt-in.

**Agent models inside workflow scripts:** every `agent()` call MUST set the `model`
parameter explicitly, chosen per "Delegating to sub-agents" above. Only `haiku`,
`sonnet`, or `opus`.

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
