---
type: feature-product
feature: spec-skill-improvements
sibling: tech.md
parent: ../../product.md
updated: 2026-06-21
---

# spec-skill-improvements — Product Spec

Elevate the spec skill from a complete v1.8 framework to an excellent v2.0
platform. The core two-layer model and 6-step authoring flow are NOT changed.
This feature adds: a composable subagent architecture, structural SKILL.md
fixes that unblock direct invocation, mechanical automation for compound, and
machine-readable spec formats for better tooling and validation.

**Design principle — Promote first, self-suffice always:**
At every authoring step the skill SHOULD surface the optimal executor (a
superpower or subagent) as a proactive offer to the user before executing
anything itself. The fallback — the agent running the step directly with
constraint docs as its guide — MUST always work without external dependencies.
The order is: suggest the best tool, wait for the user's answer, then
self-execute if declined or unavailable. Never silently skip the offer;
never block on the answer.

**Research:** [research.md](research.md)

---

## Scope

| Owns | Does not own |
|---|---|
| `.agents/skills/spec/SKILL.md` — all frontmatter and body changes | Flow state machine transitions |
| `.agents/skills/spec/subagents/*/SKILL.md` — four subagent definitions | `vibe-compound` procedural logic |
| `.agents/skills/spec/feature.md` output profiles | `vibe-feature` delegation ordering |
| `.agents/skills/spec/strategy.md` additions | Platform adapter hooks |
| `scripts/promote.sh`, `scripts/lessons-for.sh`, `scripts/scan-merges.sh` | `lessons.md` content (written by vibe-flow/D8) |
| `validate.sh` extensions SF13–SF16 | vibe-flow D8 read-on-entry mechanism |
| `reference/templates/` — new branch doc templates + research template | Skill discovery registry |
| New `argument-hint` routes and `## Routing` entries | `vibe-compound` script body |
| OpenSpec-compatible frontmatter (opt-in, warn-only initially) | External tool integrations |
| SKILL.md `allowed-tools`, `context:`, `subagents:`, `superpowers:`, `caveman:` frontmatter | superpowers:* namespace ownership |

---

## Requirements

### Requirement: Subagent role profiles as superpowers delegation contexts

The spec skill SHALL define four named authoring roles in `SKILL.md § Roles` —
spec-interviewer, spec-architect, spec-planner, spec-auditor — where each role
is a **delegation context**, not a custom tool: it names the executor superpower
and the constraint document to inject, not a bespoke implementation.

Roles that have a superpower executor (spec-interviewer → `superpowers:brainstorming`;
spec-planner → `superpowers:writing-plans`) MUST document: the superpower name,
the constraint section from the spec skill to inject, and the output validation
criteria the resulting spec must satisfy.

Roles that are spec-unique (spec-auditor → `validate.sh`; spec-compactor →
`promote.sh`) MUST document: the script to run and any secondary superpower
that receives the script output as context.

Each role MUST be invocable as a discrete `/spec <role>` argument without
requiring the full 6-step flow to run.

Each role with a superpower or subagent executor MUST proactively offer that
executor to the user before running anything — phrased as "I can use X for
this, which gives you Y — want me to?" The role MUST then self-execute the
step directly (using its constraint document as guidance) if the user declines
or the executor is unavailable. Silent delegation and silent self-execution are
both forbidden; the offer is the handshake.

#### Scenario: spec-interviewer delegates to superpowers:brainstorming

**Given** a user invoking `/spec interview my-feature`
**When** the spec-interviewer role activates
**Then** it injects `feature.md § Interview for WHAT` as the constraint document
and delegates the dialogue to `superpowers:brainstorming` — no custom interview
script runs

#### Scenario: spec-planner delegates to superpowers:writing-plans

**Given** a feature with product.md and tech.md already written
**When** the user invokes `/spec feature my-feature` at the plan step
**Then** the spec-planner role injects `reference/plan.md` + stable-ID rules
and delegates unit decomposition to `superpowers:writing-plans`

#### Scenario: Role proactively offers executor before running

**Given** a user invokes `/spec feature my-feature` at the plan step
**When** the spec-planner role activates
**Then** it tells the user "I can use `superpowers:writing-plans` for this —
it's purpose-built for decomposing requirements into stable-ID units. Want me
to?" and waits for an answer before writing anything; if the user declines, it
runs the decomposition directly using `reference/plan.md` as its constraint

#### Scenario: Role self-executes when executor unavailable

**Given** a user invokes `/spec interview my-feature` but `superpowers:brainstorming`
is not available in the current environment
**When** the spec-interviewer role activates
**Then** it conducts the WHAT interview directly using `feature.md § Interview
for WHAT` as its constraint, producing the same output format as if brainstorming
had run — no error, no degraded output quality signal

#### Scenario: Role sections are machine-readable

**Given** SKILL.md with `## Roles` section containing role subsections
**When** a skill-discovery tool reads the frontmatter `delegates:` field
**Then** it can enumerate all roles, their executor superpowers, and I/O contracts
without parsing the prose body

---

### Requirement: Compound promotion automation

The spec skill SHALL provide `scripts/promote.sh <name>` that extracts
`<!-- merge -->` / `<!-- /merge -->` blocks from `features/<name>/tech.md`
and promotes them to root `.spec/tech.md` via atomic temp-file rename.

The script MUST validate that merge markers exist as exact lines, that
start precedes end, and MUST refuse to proceed (never mangle) on
reversed or overlapping markers, per the active rule.

The script SHOULD offer a `--dry-run` flag that prints a preview diff
without modifying any files.

#### Scenario: Normal promotion

**Given** `features/my-feature/tech.md` with valid `<!-- merge -->` blocks
**When** `promote.sh my-feature` is run
**Then** the marked blocks are appended to `.spec/tech.md` and a success
message states how many blocks were promoted

#### Scenario: Reversed markers — refuse, never mangle

**Given** `features/bad-feature/tech.md` with a `<!-- /merge -->` appearing
before the paired `<!-- merge -->`
**When** `promote.sh bad-feature` is run
**Then** the script exits non-zero with an error message and `.spec/tech.md`
is byte-unchanged

#### Scenario: Dry run preview

**Given** `features/my-feature/tech.md` with valid merge blocks
**When** `promote.sh my-feature --dry-run` is run
**Then** the diff of what would be promoted is printed to stdout and no
files are modified

---

### Requirement: Tag-based lessons extraction

The spec skill SHALL provide `scripts/lessons-for.sh <tag> [<tag>...]` that
reads `.spec/lessons.md` and returns only the lessons whose `**Tags:**` line
includes at least one of the supplied tags.

The script MUST exit 0 with empty output when no lessons match (graceful
degrade; never error on absent match).

The script SHOULD support `--format inject` to wrap output in delimiters
suitable for per-turn hook injection.

#### Scenario: Relevant lessons found

**Given** `lessons.md` with a lesson tagged `spec, validate`
**When** `lessons-for.sh validate` is run
**Then** that lesson is printed and unrelated lessons are omitted

#### Scenario: No matching lessons

**Given** `lessons.md` with no lessons tagged `auth`
**When** `lessons-for.sh auth` is run
**Then** stdout is empty and exit code is 0

#### Scenario: Inject format for hook

**Given** matching lessons exist
**When** `lessons-for.sh spec --format inject` is run
**Then** output is wrapped in `<!-- lessons: spec -->` … `<!-- /lessons -->`
delimiters suitable for injection into a per-turn hook

---

### Requirement: Cavekit-aware output profiles

The spec skill SHALL define output profiles for each caveman level (lite,
full, ultra) in `feature.md § Output profiles` so that agents know exactly
which sections to produce, compress, or omit at each level.

Each profile MUST be a named table specifying mandatory, compressed, and
omitted sections for product.md, tech.md, and plan.md.

Agents running the spec skill SHOULD match the feature's active flow phase
to the appropriate profile without requiring explicit override.

#### Scenario: Lite profile during feature.design

**Given** an agent running `feature.design` with `caveman=lite`
**When** the agent references the spec skill's lite output profile
**Then** it produces a product.md with Scope table and requirement titles
only (scenarios omitted or one-line), and a tech.md with file paths and
contract signatures but no prose rationale paragraphs

#### Scenario: Full profile as default for plan phase

**Given** an agent running `feature.plan` with no caveman override
**When** the agent references the spec skill's full profile
**Then** it produces a complete plan.md with all unit ID fields, requirement
trace section, and verification evidence rows per unit

---

### Requirement: SKILL.md metadata enrichment

The spec skill SKILL.md frontmatter SHALL be extended with structured fields:
`outputs:` (list of files the skill writes), `reads:` (files it reads on entry),
`delegates:` (superpower and subagent delegation map), `phases:` (which flow
phases invoke this skill), and `caveman:` (per-level description).

These fields MUST be valid YAML parseable without custom tooling.

`validate.sh` SHOULD warn (SF-meta) when a `delegates:` entry names a skill
path that does not exist in `.agents/skills/`.

#### Scenario: Automated skill dependency scan

**Given** SKILL.md with `delegates:` listing `code-explorer` and `spec-interviewer`
**When** a dependency-check tool reads the SKILL.md frontmatter
**Then** it can list all delegation targets without parsing the prose body

---

### Requirement: SF13 cross-reference integrity validator

`validate.sh` SHALL add SF13: scan root spec files for markdown links pointing
to `features/<name>/` or `archive/<name>/` paths that do not exist on disk.

SF13 MUST emit a WARN (never an error) for each stale link with file, line
number, and the broken target path.

SF13 MUST pass silently when all linked feature/archive paths exist or when
root specs contain no such links.

#### Scenario: Stale link to deleted feature

**Given** `.spec/product.md` containing a markdown link to `features/old-thing/product.md`
and the `old-thing` folder has been deleted
**When** `validate.sh` runs
**Then** SF13 emits a WARN naming `product.md` line N and the broken target

#### Scenario: No stale links

**Given** all markdown links in root specs point to existing files or external URLs
**When** `validate.sh` runs
**Then** SF13 passes silently (no WARN emitted)

---

### Requirement: SF14 scope conflict detection

`validate.sh` SHALL add SF14: parse all active feature `product.md` Scope
"Owns" tables and warn when two features claim the same responsibility keyword.

SF14 MUST normalize keywords to lowercase and match by substring (to catch
"flow state" matching "flow state machine").

SF14 MUST list both conflicting features and the shared keyword in the WARN
message.

#### Scenario: Two features own the same thing

**Given** `features/a/product.md` owns `flow state` and `features/b/product.md`
also owns `flow state` in its Scope table
**When** `validate.sh` runs
**Then** SF14 emits a WARN naming both feature folders and the conflicting term

---

### Requirement: `argument-hint` and routing expansion

The SKILL.md `argument-hint` and `## Routing` table SHALL be extended to cover:
`interview [<name>]`, `promote <name>`, `audit`, `diff`, `health`,
`research <name>`, `lessons <tag>`.

Each new route MUST have a one-line description in the Routing table and invoke
the corresponding subagent or script.

#### Scenario: New routes are discoverable

**Given** a user running `/spec` with no argument in an environment that shows
argument hints
**Then** the hint includes `diff`, `health`, `research`, `lessons`, `promote`,
`interview`, and `audit` alongside the existing options

---

### Requirement: SKILL.md `allowed-tools` expansion

The SKILL.md `allowed-tools` field SHALL include `Edit`, `Write`, `Glob`, and
`Grep` so that the spec skill can write spec files when invoked directly without
vibe-flow.

The current restriction to Read-only tools MUST be lifted because a user running
`/spec feature <name>` without the vibe flow active expects the skill to produce
spec files, not just read them.

#### Scenario: Direct invocation writes a spec

**Given** a user runs `/spec feature my-feature` without a vibe-flow cursor active
**When** the spec skill runs the feature authoring flow
**Then** it can write `features/my-feature/product.md` and `features/my-feature/tech.md`
directly without erroring on a missing write permission

---

### Requirement: SKILL.md structural frontmatter additions

The SKILL.md frontmatter SHALL add: `context:` (session-start auto-injection list),
`subagents:` (manifest of subagent SKILL.md paths with trigger and caveman fields),
`superpowers:` (per-phase declaration), and `caveman: lite` (top-level default).

The metadata version SHALL be bumped to `2.0` to signal the structural additions.

The `context: session-start:` list MUST include `.spec/lessons.md` and `.spec/plan.md`
so the harness can auto-inject them on every `/spec` invocation.

The `subagents:` manifest MUST list all four subagents with `name`, `path`,
`trigger`, `caveman`, and optionally `parallel-safe: true` for spec-tracer.

#### Scenario: Harness auto-injects session-start files

**Given** SKILL.md with `context: session-start: [.spec/lessons.md, .spec/plan.md]`
**When** the user invokes any `/spec` command
**Then** the harness loads both files into context before the skill body runs,
without the agent needing to remember to read them

#### Scenario: spec-tracer marked parallel-safe

**Given** the `subagents:` manifest entry for `spec-tracer` has `parallel-safe: true`
**When** `vibe-feature` reads the manifest at `feature.design`
**Then** it knows to invoke `spec-tracer` and `spec-interviewer` simultaneously

---

### Requirement: Composable subagent architecture

The spec skill SHALL organise four dedicated subagent SKILL.md files under
`subagents/<name>/SKILL.md` — replacing the current approach of role sections
embedded in the main SKILL.md body.

Each subagent MUST be a complete, standalone SKILL.md with its own
`allowed-tools`, `caveman`, input/output contracts, and validation criteria.

`spec-interviewer` MUST conduct a structured 5-question dialogue (problem →
scope → first scenario → failure modes → done signal) and output a partial
`product.md` draft for human review, not a committed file.

`spec-tracer` MUST be read-only (only `Read`, `Glob`, `Grep`, `Bash` for
read-only commands) and MUST declare `parallel-safe: true` in its manifest
entry because it does not touch any files written by `spec-interviewer`.

`spec-promoter` MUST call `scan-merges.sh` first to produce a structured diff,
present it to the user, and execute the merge to root specs only after explicit
confirmation. It MUST update the root `plan.md` feature sequence to DONE.

`spec-health` MUST check: stale `updated:` dates (>30 days with git activity on
the file), requirements with no test evidence in plan.md, orphaned `<!-- merge -->`
blocks in archived features, feature plans with empty verification columns, and
root specs exceeding 200 lines. It MUST output a per-spec health report.

#### Scenario: spec-tracer runs parallel with spec-interviewer

**Given** vibe-feature enters `feature.design` and reads the subagents manifest
**When** it invokes the design phase
**Then** spec-tracer and spec-interviewer start simultaneously; spec-tracer
reads the codebase while spec-interviewer conducts the WHAT dialogue

#### Scenario: spec-promoter shows diff before executing

**Given** `features/my-feature/tech.md` has two `<!-- merge -->` blocks
**When** the user invokes `/spec promote my-feature`
**Then** spec-promoter calls `scan-merges.sh`, displays a structured diff of
the two blocks and their destination in root `tech.md`, and waits for explicit
user confirmation before writing anything

---

### Requirement: Branch doc templates

The `reference/templates/` directory SHALL contain three new templates:
`product-topic.md`, `tech-topic.md`, and `plan-topic.md` for branch docs.

Each template MUST pre-fill `type: branch` frontmatter and include `parent:`,
`scope:`, and `covers:` fields (currently validated by validate.sh but not
templated anywhere), plus a cross-reference stub to root entrypoints.

Additionally, a `research.md` template SHALL be added covering: problem being
investigated, approaches tried, measurements/results, rejected alternatives with
rationale, and references.

#### Scenario: Branch doc template eliminates validation warnings

**Given** a user creates `tech-infra.md` by copying `product-topic.md` template
**When** `validate.sh` runs
**Then** no warnings are emitted for missing frontmatter fields (type, parent,
scope, covers), because the template pre-fills them correctly

---

### Requirement: `scripts/scan-merges.sh`

The spec skill SHALL provide `scripts/scan-merges.sh <feature-name>` that reads
`features/<name>/tech.md`, finds all `<!-- merge -->` / `<!-- /merge -->` blocks,
and outputs a structured report: block count, source file, line ranges, estimated
line count per block, and the content preview.

The script MUST exit 0 with a "no merge blocks found" message when the feature
has no markers (graceful degrade).

`spec-promoter` MUST call `scan-merges.sh` as its first step to produce the
diff preview before asking for user confirmation.

#### Scenario: Scan reports merge blocks before promotion

**Given** `features/my-feature/tech.md` with two `<!-- merge -->` blocks
**When** `scan-merges.sh my-feature` is run
**Then** it prints: block count (2), line ranges for each block, and a preview
of the content that would promote to root `tech.md`

---

### Requirement: OpenSpec-compatible optional frontmatter

Feature `product.md` SHOULD support an optional `requirements:` YAML block in
frontmatter where each entry declares `id`, `title`, `priority` (RFC-2119 strength),
`scenarios` (expected count), and `unit` (traced plan unit ID).

Feature `plan.md` SHOULD support an optional `units:` YAML block where each entry
declares `id`, `seq`, `summary`, `depends`, `verification`, and `req-ids`.

Both additions MUST be opt-in — specs without them pass validation unchanged.

`validate.sh` SHOULD add a warn-only check: when `requirements:` frontmatter is
present, the declared `scenarios` count SHOULD match the actual `#### Scenario:`
block count in the file body.

#### Scenario: Requirement frontmatter is optional and non-blocking

**Given** a feature `product.md` with no `requirements:` frontmatter
**When** `validate.sh` runs
**Then** no warning is emitted about missing structured requirements

#### Scenario: Scenario count mismatch warns

**Given** a `product.md` with `requirements: [{id: R1, scenarios: 3}]` but
only 2 `#### Scenario:` blocks under the R1 requirement
**When** `validate.sh` runs
**Then** a WARN is emitted about the count mismatch (never an error)

---

### Requirement: SF15 and SF16 validators

`validate.sh` SHALL add two new warn-only checks:

**SF15 — Root spec length:** if `product.md` or `tech.md` exceeds 200 lines,
emit a WARN that feature-level detail may have leaked into the root layer.

**SF16 — Lessons tag coverage:** if any lesson entry in `lessons.md` has no
`**Tags:**` line, emit a WARN. (All current entries have tags; this guards
future entries against the format degrading.)

Both SHALL be warn-only with no path to error promotion in this feature.

#### Scenario: Oversized root spec triggers SF15

**Given** `.spec/product.md` is 250 lines long
**When** `validate.sh` runs
**Then** SF15 emits a WARN: "product.md exceeds 200 lines — check for feature-level detail"

---

## Non-Goals

- Changing the two-layer model (root + feature layers)
- Changing the 6-step feature authoring interview flow
- Rewriting existing SF0–SF12 validators
- Making OpenSpec frontmatter mandatory (always opt-in, warn-only)
- Shipping a CI pipeline (wiring is deferred; scripts work standalone)
- Modifying `vibe-compound` body (spec skill provides tooling; vibe-compound calls it)
- Owning the `superpowers:*` namespace
- Implementing spec-interviewer or spec-health before structural items ship (see priority)
