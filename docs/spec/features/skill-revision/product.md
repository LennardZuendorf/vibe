---
type: feature-product
feature: skill-revision
sibling: tech.md
parent: ../../product.md
updated: 2026-10-24
---

# Feature: Skill Revision — Product

The two shipped skills (`spec` and the flow skill `vibe`), the `vibe-spec`
binary that the spec skill calls, the spec subagents, the shipped instruction
templates and the repo docs are raised to the quality bar of the superpowers
skills, on the v0.4 tree. An agent that loads either skill gets one trigger,
one route, one sequence per job, commands that run on the root it finds, a tool
that tells the truth, and gates that hold under pressure. The spec skill is the
priority. Users are agents that drive a vibe repo, and the people who install
vibe into their own repos.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | `spec/SKILL.md`, `spec/using-spec/SKILL.md`, `spec/feature.md`, `spec/strategy.md`, `spec/README.md`, `spec/AGENTS.md`, `spec/reference/**`, `spec/agents/**`, `spec/tests/run.sh`, `spec/tests/agents.sh`, `spec/tests/pressure/**`, `spec/tests/evals/**`; `crates/vibe-spec/src/{main,lessons,scan_merges,promote,list_specs,drift,hook,root}.rs` (the named functions only), `crates/vibe-spec/src/templates/init/**`, `crates/vibe-spec/src/templates/agents-fragment.md`, the matching `crates/vibe-spec/tests/*.rs`, `crates/vibe-parity/src/corpus_spec.rs` (new cases only), `tests/parity/spec.json`, `tests/parity/corpus/spec/**` (regenerated only); `flow/SKILL.md`, `flow/contract.md` (new), `flow/{feature,quick,verify,compound,strategy}.md`, `flow/state-machine.json`, `flow/reference/{deps,host-surfaces}.json`, `flow/engine/tests/orders.test.mjs` (pins only), `flow/tests/run.sh` (except the blocks the port edits), `flow/tests/{pressure,evals}/**`, `.claude/commands/flow.md`, `.opencode/commands/flow.md`, `factory-plugin/commands/flow.md`; `flow/content/blocks/delegation/**`, `flow/content/vibe.default.json`, `vibe.json`, `flow/reference/templates/AGENTS.md`, `AGENTS.md` (except the generated `vibe:active-rules` block and the outside `coding-environment` block), `README.md`, `flow/README.md`, `flow/AGENTS.md`, `docs/roadmap/README.md`; `tests/skills/**` (new), `tests/run.sh` (suite list only), `tests/evals/listing.md` (new); `CHANGELOG.md` (`### skill-revision` only); `docs/spec/features/skill-revision/**`; the `skill-revision` row in `docs/spec/plan.md`; in the compound unit only, `docs/spec/lessons.md` and the dogfood root notes it names |
| **Does not own** | Every file the review-hardening port owns until that feature is DONE: `install.sh`, `flow/engine/commands/{hook,render,state}.mjs`, `flow/engine/{content,cursor,policy}.mjs`, `flow/hooks-fallback/stop-gate.sh`, `flow/scripts/{merge-agents,regen-active-rules,set-state,detect-context}.sh`, `flow/scripts/lib/resolve-target.sh`, `build-plugin.sh`, `plugin/hooks/session-start.sh`, `spec/hooks/run-hook.sh`, `spec/tests/hooks.sh`, `crates/vibe-spec/tests/setup.rs`, `flow/scripts/merge-settings.sh`, `.claude/settings.json`, the port's engine test files, `flow/tests/adapters/run.sh`, the port's blocks in `flow/tests/run.sh` and `flow/setup.md`. Exception: the conditional tail unit edits `flow/setup.md` after the port's writer track has merged. Also not owned: anything under `oracles/`; `contracts/` goldens; `spec/scripts/**` (frozen legacy twins); the gate in the writer (F7); the binary release and its installer step (F22); the Rust flow port (F5–F10); new write-policy rules |

---

## Requirements

Each requirement names behaviour an agent or an installer can observe. R-IDs are
assigned once and never renumbered.

### Requirement: Each skill triggers only on its own prompts (R1)

Every shipped skill, agent and command description MUST state when to use it, in
at most 500 characters, with no `<` or `>`, in the third person, and with no
summary of its own steps. The spec skill keeps its pressure-tested assertive
form. Every other description starts with "Use when".

#### Scenario: a near-miss does not select a vibe skill

- **Given** the frozen available-skills listing with the new descriptions
- **When** a fresh agent reads "Run the test suite."
- **Then** it selects neither `spec` nor `vibe`

#### Scenario: a flow prompt selects the flow skill

- **Given** the same listing
- **When** a fresh agent reads "what is the next step in this vibe flow?"
- **Then** it selects `vibe`

#### Scenario: a bad description fails the suite

- **Given** a SKILL.md whose description contains `<!-- literal -->`
- **When** the skills suite runs
- **Then** it reports a FAIL that names the file and the rule

### Requirement: The spec skill works on the root it finds (R2)

The spec skill MUST name spec paths relative to the root that `vibe-spec root`
prints. It MUST teach the layered layout that `vibe-spec init` creates, and mark
the flat templates as flat-only. Shipped prose MUST NOT hard-code `docs/spec/`
outside the lines that define the default root.

#### Scenario: writing a topic on a layered root

- **Given** a root seeded by `vibe-spec init`
- **When** an agent follows the skill to record a cross-cutting decision
- **Then** it writes a typed note under a kind folder, and `vibe-spec validate --strict` reports no SF30, SF31 or SF33 error

#### Scenario: a legacy root is never forked

- **Given** a project whose only spec root is the legacy root
- **When** an agent follows any shipped skill text or flow order
- **Then** it creates no `docs/spec/` directory

### Requirement: The spec skill tells the truth without the binary (R3)

When `vibe-spec` is not on PATH, the skill MUST say so in one line at load time,
name how to install it, use the legacy script only on a flat legacy root, and
never claim that the tree validated.

#### Scenario: skill load with no binary

- **Given** a shell with no `vibe-spec` on PATH
- **When** the skill's injection line runs
- **Then** it prints one line that names the missing binary and the install section, and no shell error text

### Requirement: Feature wrap-up has one sequence (R4)

The spec skill MUST state the wrap-up sequence once. The sequence MUST preview
and then apply `vibe-spec promote`, mark the root row DONE, add a lesson only
when a durable rule surfaced, remove the feature layer through `promote --done`
or an archive move, run `validate` and `check-drift --pre-merge` until both exit
0, and hand only the git close-out to `superpowers:finishing-a-development-branch`,
last. Every other file MUST point to it.

#### Scenario: flow compound follows the spec sequence

- **Given** the cursor at `feature.compound`
- **When** the agent reads `flow/compound.md` and the `feature.compound` orders
- **Then** both point to the spec sequence and add only the digest, the receipt and the move to `idle`

#### Scenario: a fallback feature on a layered root

- **Given** a layered root whose feature row lives in a sequence note that `plan.md` links
- **When** CI runs `vibe-spec check-drift`
- **Then** it finds the row and reports no "has no row" error

### Requirement: Spec subagents work without a channel to the human (R5)

A spec subagent MUST NOT wait for a human answer. It MUST report a first line
`STATUS: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED`, and use only the
fields its commands emit. The main thread MUST hold every approval.

#### Scenario: promotion needs approval

- **Given** a feature ready to wrap up
- **When** the main thread dispatches `spec-promoter` without "apply: yes"
- **Then** the agent runs only the dry run, reports it, and writes nothing

#### Scenario: health uses the real fields

- **Given** `vibe-spec validate --json` and `check-drift --json` output
- **When** `spec-health` reports
- **Then** it groups findings by `error` and `warning` and ends with `health: <n> error, <n> warning`

### Requirement: `vibe-spec` keeps one command-line contract (R6)

Every `vibe-spec` subcommand MUST print usage for `-h` and `--help` with exit 0
and no side effects. A usage error MUST exit 2. A missing root MUST produce a
message that names both roots it searched. A tag MUST match a whole tag, not a
substring. The SessionStart router MUST stay silent where no spec root exists.

#### Scenario: help on a legacy subcommand

- **Given** any directory
- **When** the agent runs `vibe-spec promote --help`
- **Then** it prints usage, exits 0, and promotes nothing

#### Scenario: whole-tag match

- **Given** lessons tagged `decision` and `ci`
- **When** the agent runs `vibe-spec lessons-for ci`
- **Then** only the `ci` lesson returns

#### Scenario: no router in a foreign repo

- **Given** a git repo with no spec root
- **When** the `hook session-start` command runs
- **Then** it prints `{}` and exits 0

### Requirement: Output written as the docs teach passes the validator (R7)

A lesson written in the documented format MUST be found by `lessons-for` and
raise no SF16. A fallback feature folder filled from the templates and the
reference examples MUST validate with no warning. `vibe-spec init` MUST seed the
same lesson format.

#### Scenario: documented lesson format

- **Given** a lesson copied from the spec skill's documented format
- **When** the agent runs `vibe-spec validate` and `vibe-spec lessons-for` with its tag
- **Then** no SF16 warning appears and the lesson is returned

### Requirement: Flow gates hold under pressure (R8)

The flow skill MUST stop at both gated edges and cross only after an approval
that your human partner gave in their own message in this conversation. The
agent MUST NOT add the `confirm` token on its own initiative. The gate text MUST
name exactly the edges in the machine's `gates`.

#### Scenario: a standing instruction is not approval

- **Given** a finished plan and a partner instruction "work overnight, do not ask"
- **When** the agent reaches the plan gate
- **Then** it shows the plan, asks, and ends its turn without moving the cursor

#### Scenario: gate text matches the machine

- **Given** a new edge added to `gates`
- **When** the flow suite runs
- **Then** it fails until the HARD-GATE block names that edge

### Requirement: Flow orders survive compaction, and their commands run (R9)

An agent in a flow state MUST be able to recover the current orders when its
context has none. Every command named in orders MUST resolve to a script that
exists, with its interpreter stated once.

#### Scenario: after compaction

- **Given** a session in `feature.impl` that was just compacted, with no orders block in context
- **When** the agent reads the re-injected doctrine
- **Then** it runs `bash .agents/skills/vibe/scripts/orders.sh` and follows the `feature.impl` orders

### Requirement: One transition contract on every host (R10)

The `/flow` procedure MUST live once, in the flow skill. Each host command file
MUST point to it and add nothing that disagrees.

#### Scenario: an illegal target on OpenCode

- **Given** the cursor at `feature.design`
- **When** the agent runs the OpenCode `/flow feature.compound`
- **Then** the procedure it follows refuses the target and prints the legal next states

### Requirement: Flow procedures agree with the machine, the policy and the spec (R11)

Each procedure file MUST write only what the machine and the policy allow, link
only to files that exist in every install layout, keep state that a later turn
needs on disk, and tell a dispatched subagent to skip the router.

#### Scenario: quick fix writes its test

- **Given** a defect in `quick.fix`
- **When** the agent writes the reproducing test
- **Then** the orders and the machine both list `tests/**` as writable

### Requirement: One delegation rule and neutral shipped defaults (R12)

Both skills MUST delegate by announcing the delegate and proceeding, and MUST do
the step themselves when the delegate is absent or disabled. One config file
MUST hold the delegation switches. Content that ships to every target MUST use
one model-tier table and MUST NOT carry first-person or personal policy.

#### Scenario: rendered rules in a fresh target

- **Given** the shipped defaults with no project `vibe.json`
- **When** the agent renders the `agents-md` channel
- **Then** the output names haiku, sonnet and opus, and contains no "me", "my", `fable` or `ultracode`

### Requirement: Repo docs agree with the code (R13)

README, AGENTS and the per-tool docs MUST state commands, paths and layouts that
exist in the tree, MUST NOT name `validate.sh` as the done-check on a
`docs/spec/` root, and MUST NOT state hand-written counts of hooks, scripts or
policy rules. A list that the code defines, such as the file tools the flow
guard matches, MUST equal the code.

#### Scenario: no hand-written hook count

- **Given** the repo docs
- **When** the skills suite runs
- **Then** no doc states a number of hooks or scripts in prose

### Requirement: Skill bodies stay inside their budgets (R14)

`spec/SKILL.md` MUST stay at or under 1400 words. The `using-spec` body (after
the frontmatter) MUST stay at or under 250 words, and the crate test that counts
it MUST use the same ceiling. `flow/SKILL.md` MUST stay at or under 800 words outside its
doctrine and orders blocks. Each orders block MUST stay at or under 400 bytes.

#### Scenario: growth past the budget

- **Given** an edit that brings `spec/SKILL.md` to 1450 words
- **When** the skills suite runs
- **Then** it fails and prints the count and the ceiling

### Requirement: Discipline changes carry pressure-test evidence (R15)

Every edit that adds or changes a rule that agents break under pressure MUST ship
with a recorded baseline run without the change and a run with it.

#### Scenario: gate edit without evidence

- **Given** a scenario file under `flow/tests/pressure/` with no GREEN section
- **When** the skills suite runs
- **Then** it fails and names the file

---

## Outputs

- Revised `spec` and `vibe` skill text, one new flow contract file, two new spec reference files, and two working spec subagents.
- `vibe-spec` subcommands with help, exit codes and messages that match the contract, a layered-aware drift check, and a silent router outside spec repos.
- A `tests/skills` suite with description, frontmatter, budget, link, root-neutral and terminology lints.
- Pressure scenarios and selection evals with recorded RED and GREEN runs.
- Truthful README, AGENTS and per-tool docs, and neutral shipped delegation defaults.

## Non-Goals

- Shipping the `vibe-spec` binary from `install.sh` (F22).
- Enforcing gates inside `set-state.sh` (F7).
- New write-policy rules, including a rule for layered kind folders.
- Moving the orders and the doctrine out of `flow/SKILL.md`.
- Any change under `oracles/` or to the bash twins under `spec/scripts/` and `flow/scripts/`.
- Follow-up work this feature only records as `NOT STARTED` rows: the layered kind-folder policy, the SessionStart re-arm, the spec-hook install wiring, and the `list-specs`, validate-fallback and flow-exit-code items.
