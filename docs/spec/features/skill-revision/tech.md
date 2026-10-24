---
type: feature-tech
feature: skill-revision
sibling: product.md
parent: ../../tech.md
updated: 2026-10-24
---

# Feature: Skill Revision — Architecture

Most of this feature is prose. Most of its weight is in the tests that pin the
prose to code: a new `tests/skills` lint suite, the existing spec and flow suite
pins, the `vibe-spec` crate tests, the parity divergence list, and recorded
pressure runs. The Rust changes are small and sit in the named functions of
`crates/vibe-spec`. No JavaScript changes. Nothing under `oracles/` changes.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## Files

```
spec/SKILL.md                         # router + gate; ≤1400 words; frontmatter allowlist
spec/using-spec/SKILL.md              # one precedence line; body ≤250 words
spec/reference/setup.md               # NEW: /spec setup, config keys, templates table (moved out of SKILL.md)
spec/reference/layered.md             # NEW: layered layout guide, one worked example, ≤100 lines
spec/agents/{spec-promoter,spec-health}.md   # non-interactive, STATUS line, real JSON fields
spec/feature.md, spec/strategy.md     # fallback and root guides; point to SKILL.md § Wrap-up
spec/reference/*.md, spec/reference/templates/*   # validator parity; "flat layout only" marks
spec/README.md, spec/AGENTS.md        # v0.4 truth
spec/tests/{run.sh,agents.sh}         # pins rewritten with the text they guard
spec/tests/pressure/{run.sh,scenarios.tsv,*.md}  # init-seeded sandbox, near-misses, model leg, evidence
spec/tests/evals/{selection.json,RESULTS.md}     # NEW
crates/vibe-spec/src/main.rs          # about texts; after_help; raw-tail --help (wants_help)
crates/vibe-spec/src/{lessons,scan_merges,promote,list_specs}.rs  # help, usage exit 2, messages, whole tags
crates/vibe-spec/src/drift.rs         # row lookup through plan.md children and links
crates/vibe-spec/src/hook.rs          # session_start: silent without a root
crates/vibe-spec/src/root.rs          # line-1 doc comment only
crates/vibe-spec/src/templates/init/{lessons,plan,README}.md  # lesson format; no backlog
crates/vibe-spec/src/templates/agents-fragment.md            # install pointer instead of "build it"
crates/vibe-spec/tests/*.rs           # one test file per changed command; hook_session.rs budget pin 150 -> 250
crates/vibe-parity/src/corpus_spec.rs # generator case lessons/whole-tag
tests/parity/spec.json                # divergences with reasons; floors.lessons 5; tests/parity/corpus/spec/** regenerated
flow/SKILL.md                         # router: SUBAGENT-STOP, HARD-GATE, Entry, Routing, Transition, Red Flags, doctrine, orders
flow/contract.md                      # NEW: delegate contract, config keys, model tiers pointer, commits, precedence, stop list, validate
flow/{feature,quick,verify,compound,strategy}.md   # aligned with machine, policy and spec
flow/state-machine.json               # <root> paths, quick.fix tests, exits, evidence writes
flow/reference/{deps,host-surfaces}.json           # required_by; $comment target
flow/engine/tests/orders.test.mjs    # quick.triage prefix pin, only if its text changes
flow/tests/run.sh                     # pins rewritten with the text they guard
flow/tests/{pressure,evals}/          # NEW evidence
.claude/commands/flow.md, .opencode/commands/flow.md, factory-plugin/commands/flow.md  # thin
flow/content/blocks/delegation/{subagents,workflows}.md, flow/content/vibe.default.json, vibe.json
flow/reference/templates/AGENTS.md    # template; AGENTS.md managed block re-merged byte-identical
AGENTS.md, README.md, flow/README.md, flow/AGENTS.md, CHANGELOG.md (### skill-revision)
tests/skills/{run.sh,lib.sh}          # NEW suite + lint helpers
tests/skills/cases/{setup,spec-skill,spec-docs,spec-tool,flow,docs,sweep}.sh   # NEW, one per track
tests/run.sh                          # registers the skills suite
tests/evals/listing.md                # NEW frozen available-skills listing
```

---

## Contract / API

### Finding IDs

Audit findings are cited for traceability only. Every unit carries its own fix
text, so no implementer needs the audit files. The audits are not copied into
this folder: the layered root admits only `product`, `tech`, `design`, `plan`
and `research` under `features/<f>/` (SF30 is `error` here).

| Prefix | Source (session scratchpad, `v04/`) | Shapes |
|---|---|---|
| `spec:` | `audit-spec-skill.md` | `N-1` … `N-23` |
| `flow:` | `audit-flow-skill.md` | `V-01` … `V-47` |
| `tool:` | `audit-toolset.md` | `T1`, `U1` … `U12`, `V1` … `V5`, `W1`, `X1`, `Y1` … `Y3` |
| `cons:` | `audit-consistency.md` | `C-01` … `C-27`, `S-01` … `S-12`, `R-01` … `R-15`, `L-1` … `L-6` |
| `mig:` | `audit-migration.md` | `M-01` … `M-16` |
| `rubric:` | `skill-revision/rubric.md` | `A1` … `M9` |

### Root-neutral wording

- In skill prose, "spec-root X" means X inside the root that `vibe-spec root`
  prints. Without the binary: `docs/spec/` when it exists, else the legacy root.
  One sentence in the spec skill Overview and one in the flow doctrine define it.
  No other shipped line spells `docs/spec/` as a write or read target.
- `docs/superpowers/` stays literal. D33 fixes it as the artifact root.
- Machine `reads` and `writes` use `<root>/<path>`. The machine `$comment` defines
  `<root>`. Both consumers (`detect-context.sh context` and the engine
  `{{writes}}` variable) only display the field. No enforcer reads it.
- New prose never spells the legacy root, so it needs no `<!-- literal -->` tag.
  `tests/literals/run.sh` (`migrate --to docs/spec --check`) stays green.

### Spec skill shape (`spec/SKILL.md`)

Frontmatter keys, exactly: `name`, `description`, `user-invocable`,
`argument-hint`, `allowed-tools`, `compatibility`. The `metadata`, `context`,
`agents`, `superpowers`, `outputs`, `reads`, `delegates` and `phases` keys go,
because no harness reads them.

Description (498 characters, no `<` or `>`, pressure-tested form kept):

```yaml
description: |
  You MUST use this before changing what the product does, how it is built,
  or any file under the spec root (docs/spec) or docs/superpowers: features,
  refactors, architecture, decisions, PRDs, plans, roadmaps, and also
  one-line fixes, typos and new flags. "Quick" and "just fix it" still
  route here first. Invoke it before reading files, before writing code,
  before brainstorming alone, before answering "how should we build X",
  and whenever the user mentions spec, PRD, design doc, plan, or roadmap.
compatibility: Requires the vibe-spec binary on PATH (see the README Install section). Without it, the legacy scripts check a flat legacy root only. macOS and Linux.
```

Body order: H1 → `<SUBAGENT-STOP>` → Overview (core principle, spec-root
definition, "bundled commands are run, not read") → `<HARD-GATE>` (kept) →
The Rule → Routing (+ "Not for" line) → `$ARGUMENTS` → Red Flags (+ consequence
line) → Layers (layout switch, root table, feature layer) → Wrap-up → Roles and
dispatch → Commands → Current Project State. § Config, § Setup and § Templates
move to `reference/setup.md`.

Kept as is (they pass the rubric and carry pressure evidence): the HARD-GATE
text, the Red Flags rows, the closed fallback trigger list, the one-line
tie-breakers under Routing.

Overview, exact:

```
**Core principle:** the spec holds current decisions, never history or backlog;
code is truth, so when spec and code disagree, fix the spec in the same change.
`<root>` is the directory `vibe-spec root` prints. Bundled commands are run, not read.
```

The Rule, step 2, exact:

```
2. Run `vibe-spec lessons-for <area tags>` for the area you touch, and read that
   area's hub. Read the whole `<root>/lessons.md` only at strategy and wrap-up.
   No spec root (`vibe-spec root` exits 3) → run `/spec setup` first.
```

Routing rows replaced (superpowers 7.0.0 brainstorming has no bounded path):

```
| Feature, bounded (one file, no new requirement) | Invoke `superpowers:brainstorming` and ask for a short in-chat design, no design doc | code; root-doc drift goes through the Strategy row |
| Bug | Invoke `superpowers:systematic-debugging`; fix in place | code; root-doc drift goes through the Strategy row |
| Typo, rename, new flag (no behaviour question) | Edit directly; run `vibe-spec validate` if a spec changed | code |
```

Under the table: `**Not for:** explaining code with no change planned, running
tests, git housekeeping, formatting data.` These are the near-miss scenarios of
the pressure harness. After the Red Flags table: `**Any of these thoughts means:
stop, invoke this skill, route.**`

`$ARGUMENTS`: the `audit` row and `audit|` in `argument-hint` go. `health` runs
`spec-health` (validate + check-drift). `interview` reads: "Fallback only: run
`feature.md` steps 1–2 yourself (the WHAT interview)", with `feature.md` linked in the skill.

Layers, inserted before the root table:

```
`vibe-spec root --json` prints `layout`.
- **layered** (what `vibe-spec init` creates): the four hubs `product|tech|design|plan.md`
  are `type: hub`, ≤80 lines, and only link. Detail lives in typed notes (≤60 lines)
  under `areas/ flows/ concepts/ platform/ tech/ decisions/ questions/` or
  `parts/<part>/`. A topic is a folder note, never a root `*-<topic>.md`.
  Read `reference/layered.md` before you add a note.
- **flat** (legacy): the table below and the templates in `reference/setup.md` apply.
```

Commands, exact replacement of the "build it" sentence:

```
If `vibe-spec` is not on PATH, say "vibe-spec is not installed" and point your
human partner to the README Install section. On a flat legacy root you may run
`bash <skill-dir>/scripts/validate.sh` (`<skill-dir>` is the directory that holds
this file). On any other root no validator runs: say "validation unavailable"
in your finish line and never claim the tree validated.
```

Injection line: `` !`vibe-spec list-specs 2>/dev/null || echo "(vibe-spec is not installed; see § Commands)"` ``

### Wrap-up (single source: `spec/SKILL.md` § Wrap-up)

Replaces Layers Rule 6. `spec/feature.md`, `spec/strategy.md` and
`flow/compound.md` say only `Wrap-up: spec SKILL.md § Wrap-up.`

```
Create a todo for EACH step before starting; mark one done only after its check passes.
1. Preview: `vibe-spec promote <feature> --dry-run`. Show the output to your human
   partner and get their yes. Apply through spec-promoter (§ Roles).
2. Lesson: append one tagged entry to `<root>/lessons.md` only when a durable rule
   surfaced (format: strategy.md § Lessons).
3. Row: mark the feature DONE in the root Feature Sequence (the `plan.md` hub or the
   sequence note it links), one line, linking the live surface.
4. Remove the feature layer: superpowers → `vibe-spec promote <feature> --done`;
   fallback → `git mv <root>/features/<feature> <root>/archive/<feature>`.
5. Check: `vibe-spec validate` and `vibe-spec check-drift --pre-merge`. Fix and
   re-run until both exit 0.
6. Fallback only: ask your human partner to delete `<root>/archive/<feature>/`.
7. Last: invoke `superpowers:finishing-a-development-branch` for the git close-out only.
```

Row timing: a fallback folder needs its Feature Sequence row (status `IN PROGRESS`, the token `check-drift` reads)
before design, because `check-drift` errors on a live folder with no row. The
spec skill adds it in strategy. The superpowers layer needs no row until step 3.

### Roles and dispatch

| Role | Agent file | Model | When |
|---|---|---|---|
| spec-promoter | `agents/spec-promoter.md` | sonnet | Wrap-up step 1 |
| spec-health | `agents/spec-health.md` | haiku | `/spec health` |

Dispatch protocol (main thread): dispatch `spec-promoter` with the feature name
and no "apply: yes"; show its dry run; on your human partner's yes, dispatch it
again with "apply: yes". Handling per status: `DONE` → use the report;
`NEEDS_CONTEXT` → supply it and re-dispatch once; `BLOCKED` or a second
`NEEDS_CONTEXT` → do the step yourself. The main thread runs `--done` after the
DONE row (step 4).

`spec-promoter.md` orders, exact (the line `` `vibe-spec promote <feature>` ``
stays on its own line for `agents.sh`):

```
Orders:
1. Run `vibe-spec promote <feature> --dry-run`. Copy its output verbatim into your report.
2. Only when your task says "apply: yes", run:
   `vibe-spec promote <feature>`
   Then work the printed checklist into notes under the spec root (Write for new notes, Edit for existing ones).
3. Never run `--done`. The main thread runs it after the root row is DONE.
You do not dispatch subagents. You never wait for an answer: you have no channel to the human.
Report: first line `STATUS: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED`, then `## Dry run`, `## Applied`, `## Checklist left`.
```

`spec-health.md` steps 3–4, exact:

```
3. Reply with every finding grouped by `severity` (`error`, then `warning`): for validate
   findings give `check`, `file`, `message`; for check-drift findings give `file` (when
   present) and `message`.
4. First line `STATUS: DONE` (or `BLOCKED` when a command could not run); last line
   `health: <n> error, <n> warning`.
```

Descriptions: spec-health → `Use when your human partner asks for /spec health, a
spec audit, or a structural check of the spec tree deeper than one validate run.`
spec-promoter → `Use at feature wrap-up to preview or apply a feature's promotion
into the spec root; the main thread holds the approval.`

### Config (one file)

`.vibe/spec.json` holds every key. `root.rs` rejects unknown keys only inside
`migrate`, so new top-level keys parse today. No binary reads the three skill
keys. The agent reads them.

| Key | Default | Effect |
|---|---|---|
| `featureLayer` | `"superpowers"` | `"vibe"` → fallback feature folders |
| `suggest` | `true` | `false` → do not dispatch `feature-dev:code-explorer` or `feature-dev:code-architect` |
| `superpowers` | `{}` | `{"<skill>": false}` → that delegate is unavailable; brainstorming or writing-plans false → fallback |

A legacy `<root>/.config.yaml` is read only when `.vibe/spec.json` lacks the key.
`vibe-flow` is dropped. `/spec setup` Q2 becomes "Detected: <superpowers skills
in your skill list>. Use the fallback layer instead? yes | no".

### `vibe-spec` command-line changes

All four raw-tail subcommands (`list-specs`, `lessons-for`, `scan-merges`,
`promote`) intercept `-h`/`--help` anywhere in the tail, except as the value of
`--format`, `--tag` or `--target`: print a usage block, exit 0, before the root
is used. `--root` stays before the command for every subcommand (no
`global = true`: `trailing_var_arg` with `allow_hyphen_values` on these four
would swallow a later `--root`). Clap `about` texts drop "(port of …)":

```
/// List spec files with type and scope.
/// Print lessons whose tags match (`--format markdown|inject|json`; `--json` for Spec JSON v1).
/// Report pending merge blocks in feature tech docs.
/// Validate the spec tree (structure, links, frontmatter, strictness).
/// Check that merged features left a wrap-up trail.
/// Merge a feature's durable facts into the root spec.
```

Top-level `after_help`:
`Environment:\n  VIBE_SPEC_ROOT  spec root (overridden by --root)\nExit: 0 ok, 1 findings or refused, 2 usage, 3 no spec root`.

| Change | Today | After |
|---|---|---|
| Usage errors on raw-tail commands | exit 1 (`Unknown option`, `--format` without a value, `lessons-for` with no tags, `promote` with no args) | exit 2, same message |
| `list-specs`, no root | stdout `No … directory found in current project`, exit 0 | stdout empty; stderr `vibe-spec list-specs: WARN: no spec root (searched docs/spec/ and the legacy root up to .git). Run: vibe-spec init`; exit 0 |
| `lessons-for`, no root or no lessons file | silent, exit 0 | one stderr line naming the cause; exit 0 |
| Tag match | `**Tags:**` line contains the tag (`ci` ⊂ `decision`) | split on `,`, trim, compare whole tags case-insensitively |
| `check-drift` row lookup (a) and status (d) | `plan.md` text only | `plan.md` plus every `.md` it lists in `children:` or links relatively, one level, inside the root |
| `hook session-start`, no root | prints the router | prints `{}`, exit 0 |

Each change that alters a corpus case's output gets one `divergences` entry in
`tests/parity/spec.json` with `"reason"` ending in `(skill-revision/N)`. A new
corpus case is added only when no case covers the change. The corpus is
generated (`tests/parity/corpus/spec/.generated`; pinned by
`crates/vibe-parity/tests/spec_corpus.rs::committed_corpus_matches_the_generator`):
add the case in `crates/vibe-parity/src/corpus_spec.rs`, regenerate with
`cargo run -q --locked -p vibe-parity -- gen-spec-corpus oracles/bash/spec/reference/templates tests/parity/corpus/spec`,
raise the class floor in `spec.json`, then run `bash tests/parity/run.sh` and
`cargo test -q --locked -p vibe-parity`.

### Flow skill shape (`flow/SKILL.md`)

Frontmatter: description (363 characters):

```yaml
description: |
  Use when the repo runs the vibe flow and the turn concerns its state machine:
  a state= cursor line or a vibe orders block is in context, the partner types
  /vibe or /flow, asks for the next flow step, names a flow state, wants to
  start, resume, abort or switch a flow, or wants the vibe harness set up,
  repaired or audited. Not for spec edits outside a flow state.
compatibility: Requires bash. Node 18+ runs the hooks and the content engine (bash fallbacks keep the hard blocks). jq optional. vibe-spec optional (validate, promote). macOS and Linux.
```

Body order: H1 → `<SUBAGENT-STOP>` → Overview (2 lines, a link to
`contract.md` with "Read before any Delegate block") →
`<HARD-GATE>` → Entry → Routing (Argument | Procedure file | States | Read when) →
Transition (/flow) → Red Flags → Doctrine (SessionStart) block → Orders blocks.
§ Style, § Precedence and § Model tiers leave SKILL.md: precedence and tiers move
to `contract.md`; the style rule already rides every turn in `style.ste100`.
The machine `style` key stays (engine and suite pins); no prose calls it the
governor.

SUBAGENT-STOP, exact:

```
<SUBAGENT-STOP>
If you were dispatched as a subagent for a specific task, skip this skill. Your dispatch
prompt holds your orders. Never run set-state.sh or /flow.
</SUBAGENT-STOP>
```

HARD-GATE, exact (no `<flow>.<phase>` keys outside the edge names, which the
suite compares with the machine's `gates`):

```
<HARD-GATE>
Two edges need your human partner's approval, given in their own message in this
conversation: `feature.plan>feature.impl` (plan + impl mode) and
`feature.verify>feature.compound` (ship). At either edge: show the plan or the
evidence receipt, ask, and end your turn. Cross only after a reply that approves
this plan or this ship. Silence, a standing "don't stop" instruction, a delegate's
own approval question, or your confidence is not approval. Never add `confirm` to
`/flow` on your own initiative. Never cross a gated edge with `set-state.sh`: it
checks no gate.
</HARD-GATE>
```

Red Flags: a Thought | Reality table built only from verbatim RED quotes of unit
19's scenarios, ending with `**All of these mean: stop at the gate and ask.**`

Entry, exact:

```
0. No `.agents/skills/vibe/scripts/set-state.sh` → the flow is not installed here: say so and run no flow command.
1. Read `.agents/skills/vibe/state.json` (missing → idle).
2. No argument: open the procedure file for the cursor's state and follow it.
3. The argument names the active flow: open that procedure file.
4. The argument names another flow while the cursor is not idle: ask your human partner to finish or abort (`set-state.sh idle`) first.
5. From idle, `setup`, `strategy`, `feature` and `quick` enter at their first state; `verify` and `compound` are reached only from inside a feature or quick flow.
```

Transition (/flow): the seven steps of today's `.claude/commands/flow.md`, plus:
step 5 accepts `confirm` only when your human partner typed it in their own
message; step 6 runs `bash .agents/skills/vibe/scripts/set-state.sh <target>
[feature]`; more than one non-`confirm` token after the target → refuse with the
usage line; step 7 runs `bash .agents/skills/vibe/scripts/orders.sh` and follows
it now.

Command wrappers. OpenCode and Factory stay byte-identical and keep the three
tokens `flow/tests/adapters/run.sh` asserts (`.agents/skills/vibe/SKILL.md`,
`.agents/skills/vibe/state-machine.json`, `.agents/skills/vibe/scripts/set-state.sh`)
and none of `Gate teeth` or `Membership.`:

```
---
description: Use when moving the vibe flow cursor to another state, aborting a flow to idle, or crossing a human gate with your human partner's confirm token.
---

Transition the vibe flow: $ARGUMENTS

Follow `.agents/skills/vibe/SKILL.md` § Transition (/flow) exactly, against
`.agents/skills/vibe/state-machine.json`: refuse a target outside `next`, refuse a
gated edge without a `confirm` your human partner typed, then run
`bash .agents/skills/vibe/scripts/set-state.sh <target> [feature]`.
Never edit `.agents/skills/vibe/state.json` by hand.
```

The Claude file keeps `argument-hint: <flow.phase> [feature] [confirm]`, the
string `set-state.sh <target> [feature]`, the word `confirm` and the path
`state-machine.json` (adapter and flow-suite pins). Its body becomes the same
pointer plus the parse rules. The full procedure lives only in SKILL.md.

### Doctrine (exact; pins kept)

```
vibe flow — working model. The spec root (`vibe-spec root`; without the binary `docs/spec/` when it exists, else the legacy root) is durable memory; sessions are ephemeral. "spec-root X" means X inside that root; never create `docs/spec/` in a repo whose root is the legacy one. On session start, read spec-root `lessons.md` and `plan.md`, then the active feature's `docs/superpowers/{specs,plans}/*-<feature>*` (fallback: spec-root `features/<feature>/`). A missing `state.json` cursor means idle — not an error.

You drive the flow. Every turn, a `state=… · transition: …` line names the state and the command that leaves it; full orders arrive only on the turn after the cursor moves. No orders for the current state in this context (new session, compaction) → run `bash .agents/skills/vibe/scripts/orders.sh` and follow it. In orders, `X.sh` means `bash .agents/skills/vibe/scripts/X.sh`. At a non-gated edge, run the transition yourself — nothing advances on its own.

Two human gates — stop and get explicit approval before crossing: plan → impl, and verify → ship. Approval is your human partner's own message in this conversation; never add `confirm` yourself. Everything else auto-advances.

Write invariants (`detect-context.sh decide`; default idle when `state.json` is absent): `docs/spec/lessons.md` (or the legacy root's) only in feature.compound, setup.apply, strategy.spec, quick.verify; spec-root `{product,tech,design,plan}.md` only in strategy.spec, feature.compound, setup.apply; `.agents/skills/vibe/state.json` only via `set-state.sh`.

Output: brief technical English (ASD-STE100) — answer first, one idea per sentence, active voice, no filler or hedging; compress receipts and subagent summaries. Never compress security warnings, irreversible-action confirmations, or code/paths/commands — those stay full and byte-exact.
```

Pins this keeps: `plan → impl, and verify → ship`, `sessions are ephemeral`,
`docs/spec/lessons.md`, `.agents/skills/vibe/state.json`, `brief technical
English`, `answer first`, no `Cursor:` line. The doctrine↔`decide` parity test
splits on `;` and greps `lessons.md` and `product,tech`: each clause keeps one of
those tokens and only its own states.

Orders recovery is prose only. The code re-arm (clearing `.vibe/last-inject` on
SessionStart) needs the hook payload's `source`, which only `hook.mjs` sees;
`hook.mjs` is port-owned, so the re-arm is on the cut list.

### Orders (exact; ≤400 bytes each, `<feature>` literal)

| State | Bytes | Text |
|---|---:|---|
| `setup.detect` | — | unchanged |
| `setup.apply` | 299 | `skill=vibe · WRITE/MERGE bootstrap: merge-agents.sh AGENTS.md block, .agents/skills/vibe scaffold, bare spec root → vibe-spec init, optional adapter symlinks (partner-driven) · never clobber; on merge-agents WARN show both + ask · regen-active-rules.sh after merge · done → set-state.sh idle` |
| `strategy.brainstorm` | — | unchanged (already root-neutral: "the spec root's lessons.md") |
| `strategy.spec` | 304 | `skill=vibe · delegate spec · WRITE spec-root hubs + notes (spec skill § Layers) · validate (.agents/skills/vibe/contract.md § Validate) · durable lesson: append tagged spec-root lessons.md + regen-active-rules.sh before idle · no source · done → set-state.sh idle \| iterate: strategy.brainstorm` |
| `feature.design` | 262 | `skill=vibe · delegate superpowers:brainstorming (topic=<feature>) · READ spec-root lessons.md + product/tech hubs · WRITE docs/superpowers/specs/*-<feature>-design.md (fallback: spec-root features/<feature>/) · no source · done → set-state.sh feature.plan` |
| `feature.plan` | 292 | `skill=vibe · delegate superpowers:writing-plans · WRITE docs/superpowers/plans/*-<feature>.md (fallback: spec-root features/<feature>/plan.md, unit IDs <feature>/n) · no source · HUMAN GATE · gate: show plan + ask mode, end turn · partner approves in chat → /flow feature.impl confirm` |
| `feature.impl` | 341 | `skill=vibe · delegate executing-plans (interactive) \| subagent-driven-development (handover) + test-driven-development + receiving-code-review on verify re-entry (all superpowers:) · WRITE src/**, tests/** · no spec-root or docs/superpowers/specs/** edits · cite plan task numbers in tests/commits · done → set-state.sh feature.verify` |
| `feature.verify` | 349 | `skill=vibe · delegate superpowers:verification-before-completion + superpowers:requesting-code-review · EVIDENCE per plan task, receipt last · no spec writes · HUMAN GATE · gate: show receipt + ask, end turn · partner approves in chat → /flow feature.compound confirm \| fix → set-state.sh feature.impl \| drift → set-state.sh feature.plan` |
| `feature.compound` | 280 | `skill=vibe · spec skill wrap-up (§ Wrap-up): promote --dry-run, apply, row DONE, lesson if durable, promote --done or archive, validate + check-drift --pre-merge · regen-active-rules.sh · superpowers:finishing-a-development-branch LAST · receipt · done → set-state.sh idle` |
| `quick.triage` | ~228 | `skill=vibe · READ the spec root's lessons.md first · defect: delegate superpowers:systematic-debugging (diagnose only) \| non-defect: self-scope · escalate to feature.design: announce AND ask · done → set-state.sh quick.fix` |
| `quick.fix` | ~250 | `skill=vibe · delegate superpowers:test-driven-development (defect only) + superpowers:receiving-code-review on verify re-entry · WRITE src/**, tests/** (+ optional spec-root quick/<slug>.md) · no root spec writes · done → set-state.sh quick.verify` |
| `quick.verify` | 311 | `skill=vibe · delegate superpowers:verification-before-completion + superpowers:requesting-code-review · EVIDENCE the fix works, receipt last · no root spec writes · durable lesson: append tagged spec-root lessons.md + regen-active-rules.sh · findings → set-state.sh quick.fix \| else → set-state.sh idle` |

`\|` is a table escape; the block text has a bare `|`. Two prefixes are pinned outside this feature's files and stay: `skill=vibe · delegate executing-plans` (`flow/tests/adapters/run.sh`, port-shared) and `skill=vibe · READ the spec root's lessons.md first` (`orders.test.mjs`). Ten blocks change: unit 19 writes `feature.plan` and `feature.verify`, unit 22 the other eight. Measure each block as the flow suite does (`flow/tests/run.sh:414-422`: `orders.sh <state>` with no cursor, so `<feature>` stays literal). In a worktree whose cursor names a feature, `orders.sh` interpolates it and the count grows (`feature.compound` measures 419 B with `review-hardening`).

### Machine edits (`flow/state-machine.json`)

- Every spec-root path in `reads`/`writes` → `<root>/<path>`; `$comment` gains the `<root>` definition.
- `quick.fix.writes` → `["src/**", "tests/**", "<root>/quick/<slug>.md"]`.
- `quick.triage.exit` → `defect: root cause identified; non-defect: change is bounded and its verify evidence is named; else escalate to feature`.
- `quick.fix.exit` → `change made; defect: a reproducing test now passes`.
- `feature.verify.writes` → `[".agents/skills/vibe/evidence/feature-<feature>.md"]`; `quick.verify.writes` → `[".agents/skills/vibe/evidence/quick.md", "<root>/lessons.md"]`.
- `feature.compound.writes` adds `docs/superpowers/specs/*-<feature>-design.md`, `docs/superpowers/plans/*-<feature>.md`, `<root>/features/<feature>/**`, `<root>/archive/<feature>/**`; `exit` → `spec wrap-up done (promoted, row DONE, feature layer removed, validate and check-drift --pre-merge pass); digest regenerated`.
- `delegates` arrays keep their current names. Only the orders prose qualifies superpowers skills, so `check-skills.sh` (oracle-paired, not edited) needs no re-check.
- Keep the state keys at exactly 4 spaces (`jq` default 2-space indent does this). The jq-less `set-state.sh` parser matches `^    "<state>": {` (`set-state.sh:62-68`); unit 22 checks the count is 13 and runs the adapters set-state parity section, which has a jq and a no-jq leg.
- Top-level keys and the `style` key are unchanged.

### Flow contract (`flow/contract.md`, reference, ≤150 lines)

Sections, in order. `SKILL.md` and every procedure file link it.

1. **Delegate blocks.** `> **Delegate — X**` means **REQUIRED SUB-SKILL:** invoke `X`
   with the Skill tool (an agent name: dispatch it with the Agent tool). Say one line
   "delegating to `X` — say *self* to keep it inline" and proceed without waiting. Do
   the step yourself from the procedure file when your human partner says *self*,
   when `X` is not installed, or when `.vibe/spec.json` `superpowers.X` is `false`.
   `inject`, `output`, `redirect`, `skip`, `model` keep their meanings. When `X`
   dispatches subagents, copy `model`, `redirect` and `skip` into each prompt:
   subagents get no per-turn orders.
2. **Config.** The keys of spec § Config (`.vibe/spec.json`); `suggest: false` stops
   only `feature-dev:code-explorer` and `feature-dev:code-architect`.
3. **Model tiers.** Set `model` on every dispatch per the `delegation.subagents` block
   rendered into AGENTS.md; never inherit. Pins: `feature-dev:code-explorer` → sonnet;
   `feature-dev:code-architect` → opus; `feature-dev:code-reviewer` → opus in
   `feature.verify`, sonnet in `quick.verify`.
4. **Precedence.** The cursor owns sequencing, artifact destinations, approvals and
   commits. A delegate owns its method, except where its Delegate block overrides it.
   `set-state.sh idle` is always legal.
5. **Commits.** Follow the repo's commit policy. Where it allows commits: a delegate
   may commit its own artifact; `feature.impl` commits per plan task citing its
   number; `feature.compound` commits the spec and `docs/superpowers/` changes once,
   before `finishing-a-development-branch`; `quick.fix` commits once after
   `quick.verify` passes. Where the policy says "only on request", ask at those points.
6. **When to stop and ask (closed list).** A gated edge. Quick → feature escalation.
   Replacing a real file or user content outside managed markers. Deleting
   `archive/<feature>/`. Choosing adapter symlinks. Offering a worktree (handover mode).
   A commit where the repo policy says "only on request". Everything else: decide,
   write `Ruling: <what> — <why>`, continue.
7. **Validate.** Run `vibe-spec validate` (exit 0 = pass). Not on PATH: on a flat
   legacy root run `bash .agents/skills/spec/scripts/validate.sh`; on any other root
   write "validation unavailable (no vibe-spec)" in the receipt and never claim the
   tree validated.

### Shipped delegation defaults

- `flow/content/blocks/delegation/subagents.md` summary:
  `delegation: set model on EVERY sub-agent call — haiku (mechanical bulk) · sonnet (default implementation, exploration, per-task review) · opus (architecture, adversarial or final review) · unsure → the cheaper tier, escalate on failure`.
  Body in the second person; no `fable` bullet, no "me"/"my".
- `delegation.workflows` stays shipped and stays in the shipped `agents-md` channel
  (`content.test.mjs`, port-owned, asserts it). Its text becomes neutral: second
  person, no `fable`, no `ultracode`, no "me"/"my". The maintainer's rules (the
  ultracode opt-in and the Fable rules) move to a project block
  `local.delegation-policy` defined in this repo's root `vibe.json` and added to its
  `agents-md` channel, so this repo's rendered AGENTS.md keeps them.
- `vibe.default.json` strings name `node .agents/skills/vibe/engine/cli.mjs render agents-md --write`.

### Glossary

| Use | Not | Meaning |
|---|---|---|
| your human partner | the user, the human (agent-facing prose) | the person in the conversation; descriptions may keep "the user" |
| `<feature>` | `<name>` | the feature placeholder |
| state | phase (for `flow.phase`) | a machine key such as `feature.impl` |
| procedure file | phase file | `flow/{feature,quick,…}.md` |
| delegate | executor, superpower (noun) | the skill or agent a step goes to |
| spec root, `<root>` | spec tree, dot-spec root | the directory `vibe-spec root` prints |
| hub, note | entrypoint, branch doc | layered root files |
| topic doc | branch doc | a flat-layout cross-cutting root file |
| wrap-up | wrapup, COMPOUND | the spec sequence; `feature.compound` is the state |

### Canonical texts (single source each)

| Text | Home | Readers point to it |
|---|---|---|
| Wrap-up sequence | `spec/SKILL.md` § Wrap-up | `spec/feature.md`, `spec/strategy.md`, `flow/compound.md`, `feature.compound` orders |
| Lesson format (`**Tags:**`, optional `**Pinned-by:**`) | `spec/strategy.md` § Lessons | init `lessons.md`, flow procedure files |
| Config keys | `spec/SKILL.md` § Config pointer → `spec/reference/setup.md` | `flow/contract.md` § Config |
| Delegate contract | `flow/contract.md` § Delegate blocks | every `> **Delegate —**` block |
| Gate rule | `flow/SKILL.md` `<HARD-GATE>` | orders, command files, template |
| Transition procedure | `flow/SKILL.md` § Transition (/flow) | three command files |
| Model tiers | `delegation.subagents` block | `flow/contract.md` § Model tiers |
| Validate fallback | `flow/contract.md` § Validate (flow), `spec/SKILL.md` § Commands (spec) | orders, procedure files |
| Write rules | `flow/content/policy.json` (rendered) | doctrine (parity-tested), `spec/strategy.md` |
| Status enum `NOT STARTED \| IN PROGRESS \| BLOCKED \| DONE` | `spec/reference/plan.md` | templates |

### Test organisation

- New suite `tests/skills/run.sh`, registered in `tests/run.sh` as `skills`
  (bash). It sources `tests/skills/lib.sh`, then every `tests/skills/cases/*.sh`
  in sorted order, then prints `=== results`. Each track owns one case file, so
  parallel tracks never edit the same hunk.
- `tests/skills/lib.sh` (bash 3.2, no jq needed): `pass`/`fail`, and
  - `desc_of FILE` — the folded description;
  - `lint_description FILE [assertive]` — one line per violation: over 500 chars, `<` or `>`, first person, missing "Use when" (skipped with `assertive`);
  - `lint_frontmatter FILE ALLOWED_KEYS EXPECTED_NAME` — delimiters, key allowlist, `name`;
  - `words_outside FILE` — words outside frontmatter and outside `vibe:doctrine` / `vibe:orders` blocks;
  - `md_links FILE` — relative link targets outside code fences;
  - `resolve_links FILE LAYOUT_ROOT` — every link resolves in source and in the `.agents/skills/{vibe,spec}` layout;
  - `fixed_root_hits FILE…` — lines spelling `docs/spec/` as a target, minus the defining lines (marked by the phrase "without the binary" or "spec root (docs/spec)");
  - `pressure_ok FILE` — a file whose first line starts `# skill-revision/` has `## RED` with ≥5 rep lines and either `## GREEN` with ≥5 rep lines or a `NO-RED` verdict.
- Every guard asserts its population (files examined ≥ a floor).
- Existing pins: a wording pin is replaced by a behaviour or parity assertion, or
  deleted with the text it guarded, in the same unit. Known pins:
  - `spec/tests/run.sh`: `test_skill_v2_frontmatter_fields`, `test_skill_roles_section`, `test_skill_roles_have_agent_dirs`, `test_skill_routing_new_routes`, `test_spec_health_output_levels` (pins CRITICAL/WARN/INFO, which `--json` never emits), `test_setup_section_has_interview_flow`, `test_config_section_present`, `test_config_defaults_documented`, `test_setup_writes_config`, `test_setup_config_preserved`, `test_sf17_skill_size`, `test_sf17_description_budget`, `test_f25_voice`, `test_subagents_folder_wiring`.
  - `spec/tests/agents.sh`: ≤40 lines, exact rule lines, `model: haiku`/`sonnet`, the bare apply line.
  - `flow/tests/run.sh` (port-shared: port units 4, 7 and 19 edit their own blocks first): orders ≤400 B and `gate:` placement (kept); tier strings at the `flow-legibility/3` block; `spec-v04/F25` pins (`fallback → \`docs/spec/features/<name>/\` only when`, `root specs, git rm them`); doctrine strings; `SKILL.md precedence documents the /flow confirm token`; template hand-authored half may name only `setup.apply` as a state.
  - `crates/vibe-spec/tests/hook_session.rs` `no_root_still_routes` (unit 17) and `skill_is_within_budget` (body ≤150 words today; unit 7 raises it to 250).
  - `flow/tests/run.sh:969`: `SKILL.md` contains `confirm` (passes on any text); unit 19 replaces it with the HARD-GATE edge and provenance assertions, unit 20 adds the Transition assertions.
  - Port-owned files that pin strings this feature touches (keep the string, never edit the file): `flow/tests/adapters/run.sh` (`skill=vibe · delegate executing-plans`; the three wrapper tokens), `flow/engine/tests/content.test.mjs` (`delegation.workflows` in `agents-md`).
  - `spec/tests/run.sh` is shared by function: Track A owns `test_skill_*`, `test_spec_*`, `test_subagents_folder_wiring`, `test_config_*`, `test_setup_section_has_interview_flow`, `test_sf17_skill_*`, `test_sf17_argument_hint`, `test_sf17_description_budget`, `test_f25_voice`; Track B owns `test_feature_output_profiles`, `test_sf17_templates`, `test_sf17_companion_docs`, `test_reference_template_links_resolve`, `test_branch_doc_templates_exist`, `test_topic_doc_*`, `test_flow_mvp_4_template_hybrid_grammar`. `test_setup_writes_config` and `test_setup_config_preserved` test the frozen `setup.sh` and stay as they are.

### Pressure tests (discipline edits)

Scenario file `<skill>/tests/pressure/<slug>.md`:

```
# skill-revision/<n> — <rule under test>
Pressures: <three or more of time, sunk cost, authority, economic, exhaustion, social, pragmatic>
Compliant option: <A|B|C>
## Prompt
IMPORTANT: This is a real scenario. You must choose and act. … A) … B) … C) … What do you do?
## RED (without the change)
- rep 1: <choice> — "<verbatim rationalization>"
…
## GREEN (with the change)
- rep 1: <choice> — "<quote>"
… (one rep marked haiku)
## Verdict
```

- The orchestrator dispatches each rep as a fresh subagent: `model: sonnet` (one
  GREEN rep `model: haiku`), context = the skill text (`git show <unit-base>:<file>`
  for RED, the working copy for GREEN) plus the scenario, no session history.
  Implementers write the scenario and the change; they do not dispatch reps.
- 4 or more of 5 RED reps compliant → `NO-RED`; drop the guidance (rubric M4).
- GREEN passes at 5 of 5 sonnet reps. A new rationalization in GREEN gets a
  counter and a re-run.
- Rationalization rows and red flags quote RED reps verbatim.
- Trigger edits to the spec skill use the headless harness instead:
  `bash spec/tests/pressure/run.sh --sp both` before and after, results appended to
  `spec/tests/pressure/baseline-red.md` under a `## skill-revision/<n>` heading.
  It needs the `claude` CLI and a token; without them it exits 2 ("not run") and
  the orchestrator runs it. Scoring: the accept column is a `|` list of skills, or
  `none` for a near-miss; a near-miss passes only when the first skill is neither
  `spec` nor `vibe`. The `sp=absent` leg forces `accept=spec` only on should-trigger
  rows. 12 rows × 2 `sp` legs = 24 runs.

### Selection evals (descriptions)

`<skill>/tests/evals/selection.json`: 10 should-select and 10 near-miss queries,
each with the expected skill or `none`. The listing is `tests/evals/listing.md`:
name and description of `spec`, `vibe`, `using-spec` and the `superpowers:*`,
`feature-dev:*` and `code-review:*` skills, copied once and dated, with the
candidate description substituted. One fresh subagent per query answers with one
skill name. Old and new descriptions run on sonnet; the new one once on haiku.
Pass (sonnet): ≥9/10 positives and ≤1/10 near-misses select the skill. The listing is a dated eval fixture: re-copy it whenever the evals re-run, so old and new descriptions always meet the same competitors. Haiku is
recorded with a floor of 8/10 and 2/10. Results go to `<skill>/tests/evals/RESULTS.md`.

### Budgets

| File | Ceiling | Today |
|---|---|---|
| `spec/SKILL.md` | 1400 words (whole file) | 1718 |
| `spec/using-spec/SKILL.md` body (after frontmatter) | 250 words | 150 (175 whole file); the crate pin caps it at 150 until unit 7 |
| `flow/SKILL.md` outside doctrine/orders blocks | 800 words | 450 |
| `flow/contract.md` | 150 lines | — |
| `spec/agents/*.md` | 40 lines (existing pin) | 19 / 23 |
| each orders block | 400 bytes (existing pin) | ≤397 |
| descriptions | 500 chars | spec 507 |

Known deviation (rubric D2): the rubric caps a router at 500 words for the
whole file; `flow/SKILL.md` is 1197 words today, because the orders and the
doctrine stay in place (D12; moving them touches `content.mjs`, which the port
owns). This feature's ceiling is the 800 words outside those blocks in the table
above. The compound unit records the deviation in Spec vs Implementation.

<!-- merge -->
## Skill authoring standard

A skill description says when to use the skill, never its steps, in at most 500
characters with no angle brackets. A rule lives in one file and other files point
to it. Shipped prose names the spec root, not a fixed path. A doc that restates a
code-enforced rule is parity-tested against the code. A discipline edit ships with
a recorded pressure run without the change and with it. The `tests/skills` suite
enforces the mechanical parts.
<!-- /merge -->

<!-- merge -->
## vibe-spec command-line contract

Every `vibe-spec` subcommand prints usage for `-h`/`--help` with exit 0 and no side
effects, exits 2 on a usage error, and names both searched roots when none is
found. Lesson tags match whole tags. The SessionStart router is silent where no
spec root exists. Each divergence from the frozen bash oracles is listed with its
reason in `tests/parity/spec.json`.
<!-- /merge -->

---

## Cut list (follow-ups, not in this feature)

| Item | Why cut | Where it goes |
|---|---|---|
| flow:V-01 / mig:M-05 policy rule for layered kind folders | needs `detect-context.sh`, `policy.mjs`, `policy.test.mjs` (port-owned) and is a new policy rule | `NOT STARTED` row `layered-kind-guard` (unit 31), started after the port is DONE |
| flow:V-02 level line text | rendered by `content.mjs` (port-owned) | covered by HARD-GATE and doctrine; revisit with F7 |
| flow:V-09 plugin doctrine in spec-only repos | `build-plugin.sh`, `session-start.sh` (port-owned) | port unit 7 (its Open Question 4: silent without `.agents/skills/vibe/state-machine.json`) |
| tool:T1, mig:M-01, mig:M-03 binary install and seeding | `install.sh` (port-owned) and F22 | F22; this feature documents the install in README |
| tool:T2/T3 spec hooks wiring, `doctor` rows, shim message | the fix lives in `merge-settings.sh` (port unit 19 edits the same matcher lines), in the oracle-paired `doctor.sh`/`doctor.mjs` (a JS change; this feature makes none) and in `run-hook.sh` (port-owned) | `NOT STARTED` row `spec-hooks-install` (unit 31) |
| tool:U6 validate colour | changes every parity case | after F23 retires the oracles |
| tool:U9/U10 engine and flow-script `--help` | oracle-paired scripts; F23 retires the engine | F23 |
| tool:X1 `_engine-hook.sh` block-by-output | hook shim behaviour | follow-up |
| flow:V-45 rendered `(current: the current state)` | `policy.mjs` `renderInvariants` (port-owned) | follow-up |
| flow:V-06 code half: SessionStart re-arm of `.vibe/last-inject` | needs the payload `source`, seen only in `hook.mjs` (port-owned) | `NOT STARTED` row `session-start-rearm` (unit 31) |
| cons:C-05 flow guard matcher misses `MultiEdit` | code in `merge-settings.sh` and pins in port-owned `flow/tests/adapters/run.sh` | port unit 19; unit 27 states the matcher from the script, unit 29 re-checks it |
| tool:W2 cursor resolves into the plugin cache | `set-state.sh` (port-owned); new cursor-location behaviour | F7 |
| spec:N-16(c) `list-specs` lists top-level files only; tool:U7 validate text-mode fallback to `cwd/.spec`; tool:U4 flow-script exit codes missing from the contract | new tool behaviour or a per-script audit, outside the help/exit/no-root/tag changes | `NOT STARTED` row `spec-tool-followups` (unit 31) | <!-- literal -->
| flow:V-47 drop the doctrine's invariants clause | pinned by the parity test; owner ruling | Open Question |
| mig:M-04, M-06 … M-10 custom roots, anchoring, migrate scan set | enforcement and migrate behaviour | follow-up |
| cons:C-01 / tool:T4 version strings | release decision | release step |

---

## Review log

Two critiques of 2026-10-24: the builder review (B1-B33) and the completeness review (A1-A12 uncovered items, C1-C14 churn). Findings that touch only review-hardening are logged in its tech.md. O1 is our own finding from the checks.

| Finding | Verdict | Reason |
|---|---|---|
| B17 `using-spec` budget conflict | accept, corrected | The crate counts the body (150 words, capped at 150); 175 is the whole file. Unit 7 raises the pin to the R14 ceiling of 250; `hook_session.rs` is in Track A's Files and the overlap table. |
| B18 parity corpus is generated | accept | Unit 15 edits `corpus_spec.rs`, regenerates with `gen-spec-corpus`, raises `floors.lessons` to 5, runs `tests/parity/run.sh` and `cargo test -p vibe-parity`. |
| B19 open-ended divergence list | accept, corrected | Derived from the corpus Tags lines: every existing run returns today's lessons under whole-tag matching; only `lessons/whole-tag ["ci"]` diverges. Any other failing row stops the unit. |
| B20 pressure near-miss logic | accept | `accept=none` defines a near-miss (first skill not `spec` or `vibe`), exempt from the :59 override; 24 runs; the :49 `.spec/` check becomes `vibe-spec root`. | <!-- literal -->
| B21 bare `finishing-a-development-branch` | accept | `feature.compound` orders name `superpowers:finishing-a-development-branch`; 280 bytes. |
| B22 order count, contract path, baseline method, state count | accept | 8 blocks in unit 22; orders cite `.agents/skills/vibe/contract.md`; bytes measured as `flow/tests/run.sh:414-422` does; 13 states. |
| B23 plugin validator never run | accept | `spec/tests/plugin-validate.sh` runs in units 1 (baseline), 3, 5, 6, 7, 18, 20, 29, 30. |
| B24 weak `confirm` pin | accept, moved | HARD-GATE first appears in unit 19, so unit 19 replaces the :969 pin; unit 20 adds the Transition assertions. |
| B25 cross-track pin handling | accept | The unit that changes guarded text edits the other track's assertion line; `test_f25_voice` named. |
| B26 dangling config anchors | accept | Units 8 and 9 replace the Config notes and ban `.config.yaml`, `suggest-superpowers` and `SKILL.md#config`; links and anchors must resolve. |
| B27 budget contradiction | accept, reworded | The 500-word figure is the rubric's; it stays as the recorded D2 deviation, not as this feature's ceiling. |
| B28 `--root` and `--help` | accept | No `global = true` (it cannot reach the raw tails); `-h` is not help when it is the value of `--format`, `--tag` or `--target`; tests added. |
| B29 commit bodies | accept | Records go to `flow/evidence/skill-revision-records.md` and the PR; subject `<type>(sr/N): <subject>`, no body. |
| B30 missing shellcheck | accept | Every unit that adds or edits `*.sh` runs `tests/run.sh` and the CI shellcheck command before commit. |
| B31 undeclared overlaps | accept | `flow/tests/run.sh`, `flow/README.md`, the `AGENTS.md` active-rules block and `hook_session.rs` are in the overlap table; Track D waits for port units 4, 7, 19. |
| B32 `state-machine.json` indent | accept | Unit 22 checks 13 state keys at 4 spaces and runs the adapters set-state parity section (jq and no-jq legs). |
| B33 two writers of `## Branch work` | accept | First writer creates the section; the other appends; keep both rows. |
| A1 resolve-target sandboxes | n/a here | Port unit 4 owns all three sandboxes; Track D merges after it. |
| A2 plugin doctrine (V-09) | route | Port unit 7, its Open Question 4. |
| A3 MultiEdit (C-05) | route | Port unit 19 owns code and pins; units 27 and 29 state and check the matcher from the script. |
| A4 stale `AGENTS.md` sections (S-01) | accept, partly | Unit 27 checks Task routing, Repo layout, Spec layout and the Dogfood row. The `coding-environment` block is rejected for now: no in-repo writer (Open Question 13). |
| A5 router fragment "build it" | accept | Unit 15 fixes `agents-fragment.md`; unit 29 re-syncs the `AGENTS.md` block; unit 30's grep covers it. |
| A6 exit-code contract overclaims (U3) | accept | Unit 31 qualifies the `state set` row as a target. |
| A7 README and flow README claims (S-02, S-04) | accept | Unit 27 checks README :150, the badge, and counts of write invariants and blocking rules. |
| A8 medium findings | partly | C-08 and C-20 → unit 27 checks; N-16(c), U7, U4 → row `spec-tool-followups`; W2 → F7; C-14 rejected: naming the front door belongs to the v0.5 CLI arc (`docs/spec/tech/cli.md`). |
| A9 partial dogfood fixes | accept, partly | `spec-subagents.md:20`, `delivered-features.md`, `issue-catalog.md`, F19/F20 status, `style-note.md` → unit 31; the roadmap README → unit 29. N-23 is rejected: `lessons.md:42` already carries a `**Superseded:**` line. |
| A10 cut items with no owner | accept | Unit 31 adds `NOT STARTED` rows `layered-kind-guard`, `session-start-rearm`, `spec-hooks-install`, `spec-tool-followups`; `check-drift` reads rows only for live folders, so they raise no error. |
| A11 tool:T2 cut reason | accept the correction | Reason rewritten; still cut, because unit 19 of the port edits the same matcher lines and `doctor.mjs` is a JS change outside this feature. |
| C1 unit 7 pressure cycle | reject | R15 requires RED and GREEN for a discipline edit; the NO-RED exit caps the cost at 5 reps. |
| C2 flat zero-warning fixture | accept, modified | The fixture is a `vibe-spec init` layered root plus a fallback feature folder (the live path); root templates are only marked flat-only. |
| C3 frozen listing rots | reject | A fixed listing keeps old-versus-new comparisons fair; it is re-copied and re-dated on every eval run. |
| C4 unit 30 repeats evals | accept | The three live queries go; the pressure re-run happens only if trigger text changed after unit 4. |
| C5 rubric-A3 grep | accept | Dropped from unit 29. |
| C6 CHANGELOG backfill | accept | Dropped from unit 27. |
| C7 cosmetic sweeps | accept | The `plan.md` link-label sweep and `--root global = true` are dropped; `<feature>` and `procedure file` apply only to rewritten lines. |
| C8 qualifying delegates | accept, partly | The machine `delegates` arrays and the `check-skills.sh` fallback are dropped; the orders prose still qualifies names, which B21 needs. |
| C9 exit 2 versus one contract line | reject | Already Open Question 3; the default meets R6 at the cost of seven call sites and three divergences. |
| C14 two CI-equivalent runs | reject | Each feature's ship gate needs a receipt on its own HEAD; the legs are automated. |
| O1 status token | accept | `drift.rs` `row_status` reads only `DONE` and `IN PROGRESS`; rows and the reference enum use `IN PROGRESS` (Open Question 14). |
