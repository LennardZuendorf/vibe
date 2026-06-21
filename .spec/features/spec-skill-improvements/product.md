---
type: feature-product
feature: spec-skill-improvements
sibling: tech.md
parent: ../../product.md
updated: 2026-06-21
---

# spec-skill-improvements — Product Spec

Elevate the spec skill from a complete v1.8 framework to an excellent v2.0
platform: specialized roles, mechanical automation for the most error-prone
phase (compound), machine-readable output for tooling, and self-describing
metadata that makes the skill composable with the superpowers ecosystem.

The core two-layer model and the 6-step authoring flow are NOT changed — they
are proven and correct. This feature enriches the skill around those foundations.

**Research:** [research.md](research.md)

---

## Scope

| Owns | Does not own |
|---|---|
| `.agents/skills/spec/SKILL.md` metadata and role sections | Flow state machine transitions |
| `.agents/skills/spec/feature.md` output profiles | `vibe-compound` procedural logic |
| `.agents/skills/spec/strategy.md` additions | `vibe-feature` delegation ordering |
| `scripts/promote.sh` — compound promotion automation | Platform adapter hooks |
| `scripts/lessons-for.sh` — D8 tag-based extraction | `lessons.md` content (written by vibe-flow/D8) |
| `scripts/score.sh` — spec quality scorecard | CI pipeline wiring (deferred) |
| `validate.sh` extensions SF13–SF15 | vibe-flow D8 read-on-entry mechanism |
| New `argument-hint` routes and `## Routing` entries | Skill discovery registry |
| OpenSpec-compatible requirement markers (opt-in convention) | External tool integrations |
| `reference/product.md` additions for markers | `vibe-compound` script body |
| SKILL.md `delegates:`, `outputs:`, `reads:`, `phases:`, `caveman:` frontmatter | superpowers:* namespace ownership |

---

## Requirements

### Requirement: Specialized subagent role profiles

The spec skill SHALL define four named authoring roles — spec-interviewer,
spec-architect, spec-auditor, spec-compactor — each with explicit cognitive
mode, delegation chain, input/output contracts, and validation criteria.

Each role MUST be invocable as a discrete `/spec <role>` argument without
requiring the user to run the full 6-step flow.

#### Scenario: Role invocation without full flow

**Given** a feature with an existing `product.md`
**When** the user runs `/spec audit`
**Then** only the spec-auditor role activates, running `validate.sh` and
presenting findings without re-running the WHAT interview

#### Scenario: Role sections are machine-readable

**Given** SKILL.md with `## Role: spec-interviewer` section
**When** a skill-discovery tool scans SKILL.md
**Then** it can identify the role, its inputs, outputs, and superpower delegates
by parsing the structured section header and frontmatter fields

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
`interview [<name>]`, `promote <name>`, `audit`, `score`, `lessons-for <tag>`.

Each new route MUST have a one-line description in the Routing table and invoke
the corresponding script.

#### Scenario: New routes are discoverable

**Given** a user running `/spec` with no argument in an environment that shows
argument hints
**Then** the hint includes `interview`, `promote`, `audit`, `score`,
and `lessons-for` alongside the existing options

---

## Non-Goals

- Changing the two-layer model (root + feature layers)
- Changing the 6-step feature authoring interview flow
- Rewriting existing SF0–SF12 validators
- Adding mandatory structure to requirements (OpenSpec markers are opt-in)
- Shipping a CI pipeline (wiring is deferred; scripts work standalone)
- Creating a separate SKILL.md file per subagent role (roles are sections, not files)
- Modifying `vibe-compound` body (spec skill provides `promote.sh`; vibe-compound calls it)
- Owning the `superpowers:*` namespace
