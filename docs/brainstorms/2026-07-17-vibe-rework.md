# vibe rework — direction brainstorm (2026-07-17)

Scratch artifact from `strategy.brainstorm`. Direction is **not yet approved** —
this captures the exploration findings and the proposed rework so the next
session (or `strategy.spec`) can pick it up. Sources: five parallel subagent
reports — flow rigidity/overlap map, spec-skill depth audit, transfer-surface
map, external landscape (OpenSpec v1.6, superpowers v6.1.1, Claude Code native),
and a Claude Code capabilities check (/goal, plugins, injection channels).

The owner's six pain points, verified against the tree and the ecosystem:

---

## 1. vibe re-implements the build loop instead of orchestrating it

**Confirmed.** The re-implementation inventory: own plan grammar
(`spec/reference/templates/feature-plan.md`), own design interview (spec
`feature.md` steps 1–4 injected over `superpowers:brainstorming`), own
verify/review protocol (evidence receipts + ≥80-confidence routing in
`flow/verify.md`), own compound. Native plan mode, Ultraplan, `/goal`, and Task
tools are unused.

**But the ecosystem converged toward vibe** since these were built: superpowers
v6 plans now carry `## Global Constraints` + `Interfaces` blocks natively (vibe's
hybrid template already mirrors both), v6 replaced dual reviewers with one
unified per-task reviewer + a whole-branch review, subagent-driven-development
(SDD) now keeps a progress ledger at `.superpowers/sdd/progress.md`, dispatches
via `scripts/task-brief` / `scripts/review-package`, and mandates explicit
per-task model allocation.

**Direction (recommended): thin-orchestrator rework.** vibe stops owning
*method* anywhere and keeps only what nothing else provides:

- `.spec/` artifact destinations (durable memory — superpowers' specs/plans are
  date-stamped files, not living specs),
- stable unit IDs (`<feature>/n`) + R-ID trace as a thin **overlay** on the
  superpowers v6 plan grammar (drop the parts v6 now provides),
- the two human gates, lessons/compound, and the write invariants.

Concretely delete/absorb: the bespoke unified-review protocol in `verify.md`
(defer to v6's per-task reviewer + whole-branch review; feature-dev's
`code-reviewer` already carries the 80% confidence rule), and let the evidence
receipt *accept the SDD progress ledger* as its primary input instead of a
hand-written artifact. Wire native surfaces in: offer plan mode/Ultraplan at
`feature.plan`, feature-dev agents stay cherry-picked, mirror plan units into
TaskCreate (with `addBlocks` dependencies) during impl — noting Task tools
currently bypass PreToolUse hooks (anthropics/claude-code#20243), so the guard
can't see them.

Alternative considered: full surrender (superpowers end-to-end, vibe only maps
artifacts into `.spec/` at compound). Cheaper, but loses in-flight write
invariants and per-turn orders — the things that actually enforce adherence.

## 2. /goal is absent from the build cycle

**Fact:** `/goal` (v2.1.139+) is a session-scoped wrapper around a
**prompt-based Stop hook** — a small model judges a completion condition each
turn and re-prompts until met. It cannot be set programmatically; no hook/skill
API exists.

**Direction:** two integrations.

1. **Recipes, not automation:** each arc's orders/gate text recommends the
   matching goal at entry — e.g. entering `feature.impl` (handover mode):
   `/goal all <feature>/n units DONE in .superpowers/sdd/progress.md and test
   suite exits 0; stop after 20 turns`. The plan gate prompt offers it alongside
   the impl-mode choice.
2. **Per-state goal conditions in the machine:** since `/goal` is literally a
   prompt-based Stop hook, vibe can ship the same mechanism per state — extend
   the Stop gate with an optional prompt-based hook whose condition comes from
   the machine's `exit` field. That is the "programmable /goal" and doubles as
   the auto-advance nudge (see §5).

## 3. Transferability under-delivers

**Fact that changes everything:** the 2026-06-18 lesson ("a plugin cannot bundle
skills outside ./skills/") is **obsolete**. Plugins now bundle `skills/`
(namespaced), agents, hooks, commands, MCP servers, and `bin/`, and install
**per-user** so they apply across all repos with zero team-visible writes.

**Direction: plugin-first for Claude Code.** Package vibe as a plugin (own
marketplace repo): `vibe`+`spec` skills, `/flow`, the three hooks + a new
SessionStart hook, feature-dev-style agents, scripts under `bin/`. Hooks
self-detect vibe-enabled repos (no `.spec/` and no machine → fast exit 0, which
`orders.sh` mostly already does). `install.sh` remains the platform-neutral path
(Codex/Warp: `.agents/`, AGENTS.md merge) and the per-repo materializer
(`.spec/` seed, cursor, gitignore stanzas). "Install my stack" becomes: add
marketplace, `/plugin install vibe`, run `vibe setup` in a repo.

## 4. Spec skill not competitive with OpenSpec

**Confirmed gaps** (vs OpenSpec v1.6): no delta grammar — updating a living spec
is a free-form model edit; `promote.sh` can only append `<!-- merge -->` blocks
to EOF (no section targeting, no modify/remove — an edited re-promoted block
duplicates); the documented `requirements:`/`units:` machine-readable
frontmatter is implemented by **no validator**; ID shapes disagree (`R-1` vs
`(R1)`); scenario GWT structure is counted, never validated; `updated:`
freshness and parent↔child backlinks unchecked. OpenSpec's robustness comes
from: header-keyed deltas (`## ADDED/MODIFIED/REMOVED/RENAMED Requirements`)
applied in strict order, one shared parser across validate+archive, and
re-validating the rebuilt spec before writing.

**Direction (recommended): steal the delta engine, stay bash.**

- Adopt a delta grammar for feature→root promotion: feature specs declare
  ADDED/MODIFIED/REMOVED (later RENAMED) blocks keyed by exact requirement/
  section header; `promote.sh` becomes a header-keyed merge engine with ordered
  ops and post-merge re-validation. Supersedes EOF-append.
- Implement the already-documented frontmatter checks; unify ID shape; validate
  GWT structure inside scenarios; check `updated:` freshness and backlinks
  (warn-first per the existing lesson).
- Add an `update` route to the spec skill mirroring `/opsx:update` — revise
  existing artifacts *coherently* (propagate a requirement edit into trace,
  plan units, and scenarios) instead of ad-hoc edits.

Alternative to decide consciously: adopt OpenSpec itself as the spec engine and
keep vibe as the flow/lessons layer on top. Pros: mature merge machinery, 25+
tools, cross-repo stores. Cons: Node dependency, two toolchains, loses the
bash/KISS self-hosting property and tight lessons/compound integration.

## 5. Flow too rigid; no automatic movement; brainstorm loops uncovered

**Confirmed mechanics:** nothing auto-advances — every transition is the agent
remembering `set-state.sh`; the stuck-phase nudge is warn-only; gates exist only
in `/flow` prose (the writer doesn't gate); cursor-drift (feature work while
cursor says idle) produces at most a warn on file-tool writes and nothing for
Bash writes. The machine has **no iteration edges**: `strategy.brainstorm` and
`feature.design` are forward-only; no research loop; no
`strategy.spec→brainstorm` back-edge; long brainstorms collapse into one state.

**Direction: fewer states, more edges, mechanical nudges.**

1. **Add the missing loops as legal edges:** `strategy.spec→strategy.brainstorm`,
   `feature.plan→feature.design`, and a design↔research self-loop (research.md
   already exists as an artifact; it needs an edge, not a new phase). Iterating
   becomes a sanctioned move instead of abort-and-reenter.
2. **Collapse ceremony:** merge `quick.triage`+`quick.fix`; non-gated advances
   happen silently (no announce line); evaluate collapsing the machine to three
   arcs (shape / build / close) with the current phases demoted to checklist
   items inside orders — the "trust model intelligence" experiment.
3. **Cursor-drift detection with a one-command fix:** the guard/Stop gate infers
   likely state from activity (src edits at idle → "run /flow feature.impl or
   quick.fix") and the inject hook surfaces it as the *first* line, not a
   trailing warn. Optionally: Stop gate auto-runs `set-state.sh` on
   non-gated edges whose exit predicate is mechanically checkable.
4. **Per-state prompt-based Stop conditions** (§2) give the flow teeth that
   scale with the model instead of more hard rails.

## 6. Team repos: the missing main instruction

**Root cause found:** the session-start doctrine — "read `.spec/lessons.md` +
`.spec/plan.md`; `.spec/` is durable memory, sessions are ephemeral; write
invariants; two human gates" — lives **only** in the AGENTS.md managed block.
vibe wires no SessionStart hook; the idle inject never says "read `.spec/`
first". No AGENTS.md control → the discipline silently vanishes. `doctor.sh`
doesn't even check the block.

**Direction: a SessionStart hook carrying the doctrine, delivered personally.**

- New `session-start.sh` hook: when the repo is vibe-enabled, emit the
  session-start doctrine (+ current cursor summary) as context; matcher
  `compact` re-injects after compaction. This makes AGENTS.md *optional* — a
  nice-to-have for other tools, no longer the single carrier.
- Delivery in team repos, two compatible channels:
  `install.sh --local` mode wiring everything through gitignored
  `.claude/settings.local.json` + `CLAUDE.local.md` (repo-scoped, invisible to
  teammates), and/or the per-user plugin (§3) whose hooks fire everywhere and
  self-detect. Recommend both: plugin for the standing stack, `--local` for
  repo-specific state.
- `doctor.sh` gains an instruction-coverage check (block present OR SessionStart
  wired).

## Cross-cutting: subagent model policy

Owner directive (2026-07-17): subagents always run on Opus or Sonnet, never
inherited defaults. superpowers v6 SDD already mandates explicit model choice
per dispatch. Fold into vibe's delegation contracts: every contract block and
orders line that dispatches subagents names a tier (mechanical/explore →
sonnet; review/architecture/synthesis → opus).

## Open questions (human gate for strategy.spec)

1. **Build-loop ownership:** thin-orchestrator rework (recommended) or full
   surrender to superpowers with compound-only mapping?
2. **Spec engine:** bash delta engine in the spec skill (recommended) or adopt
   OpenSpec underneath?
3. **Machine shape:** keep ~13 states + new loop edges, or collapse to three
   arcs with checklist-orders?
4. **Distribution:** commit to plugin-first for Claude (separate marketplace
   repo?) with install.sh for neutral core?
5. **Rewrite scope:** one rework feature (`vibe-rework`) or three sequenced
   features (flow-slim, spec-delta, plugin-transfer)?
