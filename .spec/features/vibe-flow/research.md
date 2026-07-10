---
type: feature-research
feature: vibe-flow
parent: product.md
updated: 2026-07-07
---

# vibe-flow — Research: full product & flow review

External review used as discovery input for flow improvements. If a finding
should persist, extract it to a lesson (lessons.md) or a root/feature spec —
this file is deleted with the folder at wrapup.

## Question

Does the vibe flow deliver its value proposition — and how well does it
orchestrate superpowers, feature-dev, and caveman versus alternatives
(compound-engineering, spec-kit, or superpowers alone)?

## Findings

- **Date:** 2026-07-07
- **Reviewer:** external reviewing agent (Claude, multi-agent review: 2 deep-dive reviewers, 2 integration/adapter reviewers, 1 competitive-landscape researcher, adversarial synthesis by the orchestrator)
- **Target:** `main` @ `d56db8b` (post-PR #13 release line), reviewed against the brief in `.spec/archive/product-review-2026-07-06.md`
- **Baseline reproduced:** all three suites clean via `bash tests/run.sh` (serial run); `validate.sh` no errors (warnings only); `doctor.sh` reports healthy with all three external deps absent (graceful-degrade path exercised live)

---

## Verdict

**The architecture is right and genuinely differentiated; the differentiation is roughly half-built; and the half that's missing is exactly the half the product's identity claims.**

vibe wants to be the tool that *forces* structured usage of superpowers, caveman, and feature-dev through hooks. What it actually is today is the best **guidance-and-memory harness** in its category — with a real but narrow enforcement edge (3 hard-blocked write invariants) and an entirely advisory everything-else. That is still a defensible product, because *no competitor has any technical enforcement at all* — but the gap between the pitch verbs ("strict", "guarded", "force") and the shipped teeth is the single most important thing to resolve, either by earning the teeth or softening the verbs.

Scores against the brief's five dimensions:

| Dimension | Verdict |
|---|---|
| 1. Delivers its claims? | **Partially.** Memory/runtime split, per-turn direction, graceful degrade: delivered and verified. "Guarded invariants": overstated — 3 hard blocks, Bash-shaped bypass, warn-only Stop gate, transitions ungated. |
| 2. Internally coherent? | **Code + shipping docs: yes. `.spec/` memory: no.** The dogfood specs still describe a plugin the code deleted; the README contradicts itself on test count (badge 248 vs body 265, actual 265). For a spec-discipline tool, stale specs are counter-evidence for the thesis. |
| 3. Release quality | **Near-credible 0.1.0.** Install/uninstall lifecycle is excellent and verified. Blockers: badge drift, non-hermetic flow test suite (flaky under concurrency — reproduced live in this review), spec re-sweep. |
| 4. Design quality | **Sound layering, above-average rigor, over-documented relative to what it enforces.** ~3,300 LOC of flow machinery to inject one ~300-byte line per turn and deny three paths. The four-copies-of-every-fact pattern (machine JSON, script comments, phase files, three READMEs) is the recurring drift disease. |
| 5. Gaps / risks | Stop-gate has no teeth; guard blind to Bash; `quick` flow structurally cannot compound; flow tests mutate the live cursor; "offer-first" lesson never applied to `flow/`. |

---

## 1. Does the concept make sense? (product position)

Yes — and more than the repo's own docs argue. The landscape (July 2026) sorts into layers:

| Layer | Occupied by | Enforcement model |
|---|---|---|
| Methodology prose (how to brainstorm/plan/TDD/debug) | superpowers (~248k stars), compound-engineering (~23k) | Persuasion. superpowers' `<HARD-GATE>` is markdown + Cialdini techniques; CE ships **no hooks at all** |
| Spec scaffolding | spec-kit (~119k stars) | None — templates + soft phase ordering |
| Guided feature arc | Anthropic feature-dev plugin | None — slash-command chain, no memory |
| Durable memory | CE `docs/solutions/` (schema-validated), spec-kit `.specify/`, beads (git-backed DB) | Files the agent must remember to read |
| **Per-turn state + technical write denial** | **vibe, alone** | PreToolUse exit-2 deny + per-`UserPromptSubmit` injection |

Two mechanisms in vibe exist nowhere else surveyed:

1. **Per-turn cursor injection.** superpowers injects at `SessionStart` only — its discipline dies at the first context compaction. vibe re-asserts `{state, delegates, write surface, caveman level, next}` every single turn, byte-stable and prompt-cache-safe (143–332 bytes measured). This was demonstrated *during this review*: when the cursor changed, the injected orders changed on the very next turn.
2. **A real deny path.** `pre-tool-use-guard.sh` → `detect-context.sh decide` → exit 2 is qualitatively different from every competitor's "strongly worded markdown." It fails closed even when the model rationalizes past instructions — which is exactly why superpowers resorts to persuasion psychology: prompt-only compliance degrades under pressure.

So the concept — a thin state/enforcement layer *above* other people's methodology skills — is coherent, occupies empty ground, and composes rather than competes. The strategic risk is not the idea; it is that the enforcement layer is ~20% built while the *documentation about* the enforcement layer is ~100% built.

**Framing correction to the elevator pitch.** The stated goal ("force structured proper usage of superpowers/caveman/feature-dev") is not what the specs actually promise — `.spec/product.md` promises *taking planning load off the human* with graceful degradation ("warns, never hard-fails"). Those are different products. "Force" requires teeth the hooks don't have; "carry the planning load" is delivered today. Pick one and make the copy match.

---

## 2. The flow, judged

### What's excellent

- **The orders mechanism (D12) is the best-engineered part of the system.** Single-sourced from `flow/SKILL.md` marker blocks, three-tier fallback (skill block → machine inline → generic), always exits 0, jq-absent degrade tested, corrupt-cursor degrade tested, self-locates from its own script path so a fresh non-git install works (the stranger-eval bug, fixed and regression-tested — independently re-verified on a bare temp dir during this review).
- **The state graph is machine-validated and clean.** 15 states, all reachable from `idle`, no dead ends, every `next` target asserted to exist by the test suite. `amend`-as-modifier (refused as a cursor value by both `set-state.sh` and `validate-state.sh`) is good defense-in-depth.
- **Install/uninstall is the most trustworthy part of the repo.** Verified live on a fresh target: byte-identical re-install (full-tree checksum), surgical uninstall (user files in shared dirs survive; managed block stripped from `AGENTS.md` leaving user prose), live cursor preserved across re-install, reversed-marker files refused rather than mangled. The discriminating-test practice (tests that fail if safety code is swapped for the naïve destructive version) is above-average rigor for any repo, let alone a personal one.
- **Graceful degradation is real.** This review ran in a container with none of superpowers/feature-dev/caveman installed: doctor warned correctly, orders resolved, nothing hard-failed.

### Where enforcement actually stands

| Mechanism | Claimed role | Reality |
|---|---|---|
| `UserPromptSubmit` inject | per-turn direction | Works as designed. Advisory by nature. |
| `PreToolUse` guard | "guards write invariants" | 3 hard blocks only (`state.json`, `lessons.md`, root specs). Matcher is `Edit\|Write\|NotebookEdit` — **any Bash write (`tee`, `sed -i`, redirect) bypasses all three**. `.spec/features/**` and `.spec/quick/**` are completely unguarded in every state. `src/**` writes are *allowed* during verify states, contradicting `verify.md` ("no writes from verify"). |
| `Stop` gate | verify-before-done | **Always exits 0.** The verify predicate cannot check whether verification happened; it prints the same warning unconditionally. Every predicate is a TODO marked "promotion-eligible"; none promoted. |
| `set-state.sh` | "only sanctioned writer" | Writer, **not** gate — by design it does not check `next`. Any agent can `set-state.sh feature.compound` and skip design→plan→impl→verify and both human gates. Graph legality lives in the `/flow` prose command, i.e., in the model's cooperation. |

Bottom line: **the flow steers an obedient agent very well and constrains a disobedient one almost not at all.** The code is honest about this ("earn the teeth", "warn-first" comments); the READMEs and pitch language are less honest.

### Structural gaps in the graph

- **`quick` is a second-class flow.** No dedicated quick-flow compound state, and `lessons.md` is only writable in `*.compound`/`setup.apply` — so a lesson learned during a quick fix (the most common kind of work, and the most common source of lessons) **has no legal place to land**. The compounding loop silently doesn't run for the majority case.
- **No `quick.verify → quick.fix` back-edge.** `feature.verify` loops back on failure; a failed quick verify can only exit to `idle` and re-enter.
- **No mid-arc bail-out.** Once in `strategy.brainstorm` or `feature.design`, there is no legal edge back to `idle` — only forward.

### Live findings from this review session (unplanned, instructive)

1. A concurrently-running test suite leaked a live cursor (`feature.impl` / feature `widget` — a literal test fixture value) into the repo. **Root cause: `flow/tests/run.sh` is non-hermetic — it reads/writes the real repo cursor** with backup/restore via EXIT trap, which breaks under concurrency or a hard kill. This produced both a flaky `FAIL flow` on a combined run and the leaked cursor. Serial runs are deterministically green (re-verified ×3).
2. The leaked cursor **hijacked the per-turn orders** — the reviewer session was confidently instructed to "WRITE src/**, do NOT edit .spec/**" for a nonexistent feature. This demonstrates simultaneously (a) the injection works, (b) there is no staleness/liveness detection on the cursor, and (c) `git status` cannot surface the problem because the cursor is gitignored.
3. The guard hook had no opinion about any of this: the cursor was written via Bash (test suite), and would have been removable via `rm` — neither passes through `Edit|Write|NotebookEdit`.

---

## 3. Integration with superpowers, caveman, feature-dev

**The composition story is genuinely good — vibe delegates, it does not duplicate.** Phase files never re-explain how to brainstorm/TDD/debug; they name the delegate and constrain inputs/outputs/write-surface. Explicit non-goal in the specs: "Replacing Superpowers, spec, or review subagents." The feature-dev integration deliberately cherry-picks `code-explorer`/`code-architect`/`code-reviewer` into specific states instead of letting `/feature-dev` own the arc as an opaque macro — the right call.

**caveman is a vocabulary, not a dependency.** The three levels are frozen per state in the machine, and `check-skills.sh` prints the level definition inline whether or not the upstream skill exists. Honest to say so: vibe borrowed caveman's idea; it doesn't need the skill.

Gaps found:

1. **The repo's own "offer first, self-suffice second" lesson (lessons.md, 2026-06-21) was implemented in `spec/` and never in `flow/`.** Every `flow/*.md` phase file still says unconditional "Delegate to superpowers:X" — no offer, no decline branch, no `suggest-superpowers` config gate like `spec/SKILL.md` has. The flow's own delegate call sites are the exact surface the lesson describes.
2. **Skill detection is two inconsistent code paths, neither tested for success.** `check-skills.sh` never checks disk for superpowers/feature-dev (always "assumed-installed") while `doctor.sh` uses a name-based `find -maxdepth 5 -iname` heuristic that would plausibly miss marketplace-plugin layouts (namespaced dirs, per-technique folders); its "found" branch has no test — only the "absent" branch is exercised. `feature-dev` is probed as a directory name, never as its actual subagents.
3. **`deps.json` vs `check-skills.sh` duplicate the same knowledge** (the three dependency families are hard-coded in both) — the exact single-source-of-truth failure mode `lessons.md` warns about for orders.

---

## 4. Competitive position — "or would you just use superpowers?"

Direct answer: **no, but not for the reason the pitch gives.**

What "just superpowers (+ feature-dev)" gets you: excellent, battle-tested methodology prose and a `SessionStart`-injected mandate. What it doesn't survive: context compaction, long autonomous runs, and the model's tendency to rationalize past markdown gates — the authors themselves compensate with persuasion psychology, which is an admission that prose enforcement decays.

What vibe adds that is real *today*:
- Per-turn re-assertion of phase/write-surface/next (compaction-proof, drift-resistant);
- A durable, validated `.spec/` memory with scoped write surfaces;
- Three genuinely hard write blocks;
- A dependency doctor and graceful degrade so the harness works on any machine.

What vibe claims but does not add today: forced skill usage, forced phase order, forced verification. Every one of those is currently the agent's choice.

**Against compound-engineering (Every):** CE has no enforcement and no state — vibe wins the harness comparison outright. But CE's memory layer is *more mature than vibe's*: categorized, YAML-frontmatter, schema-validated `docs/solutions/` with an explicit staleness/consolidation workflow (`ce-compound-refresh`), read back by planning commands. vibe's flat `lessons.md` + regenerated Active-Rules digest is the weaker compounding story — and vibe's `quick` flow can't compound at all. The borrow-worthy idea is CE's categorized, refreshable learnings store; vibe already borrowed stable unit IDs (correctly credited in its own design docs).

**Against spec-kit:** philosophical cousin for `.spec/` (constitution/specify/plan/tasks per-feature dirs), zero runtime layer. vibe's spec half is competitive and its two-layer root-vs-feature model with merge-then-delete is arguably cleaner for a personal tool; spec-kit wins on ecosystem breadth (30+ agents).

**The moat, precisely:** it is the *mechanism* (per-turn cursor injection + a PreToolUse deny path + machine-readable state), not the current *rules* (3 blocks, warn-gate). The mechanism is cheap to extend; nobody else has it; and if the Stop gate ever earns teeth ("no done without evidence"), vibe would have the only technically-enforced verify gate in the ecosystem — the clearest possible answer to "why not just superpowers."

---

## 5. Fix-first list (ranked by leverage)

1. **Earn the Stop-gate teeth for verify states.** Even one promoted predicate — block a stop in `feature.verify` until *some* evidence artifact exists — makes "hook-enforced" true and is the single clearest differentiator vs every competitor. The TODO scaffolding is already in place.
2. **Close or acknowledge the Bash hole.** Either extend the PreToolUse matcher to inspect `Bash` writes against the same three invariants, or stop calling them "hard blocks" in the docs. (This review watched a Bash write drive the cursor straight past the guard.)
3. **Make `flow/tests/run.sh` hermetic** — point it at a temp cursor, not the live one. Proven failure mode: flaky under concurrency, leaks fixture state (`widget`) into the real repo, mutation invisible to git.
4. **Re-sweep `.spec/` to reality.** Plugin→settings.json migration (product/tech/plan, platform-adapters, CHANGELOG, lessons), the 248-vs-265 badge, vibe-flow's stale "keep all seven shims" OPEN-2 record, and `plan.md`'s now-false "no open drift" line. For this product specifically, stale specs are a credibility wound, not a nit.
5. **Give `quick` a compounding path** (a dedicated quick-flow compound state, or a legal lessons-write from `quick.verify`) and a `quick.verify → quick.fix` back-edge.
6. **Apply the offer-first lesson to `flow/` phase files** — offer the superpowers executor, self-suffice on decline, mirroring `spec/SKILL.md`'s implemented pattern.
7. **Guard the documented write surfaces you don't guard:** `.spec/features/**`, `.spec/quick/**`, and disallow `src/**` writes in verify states (the machine and `verify.md` already say so; `detect-context.sh` doesn't).
8. **Unify skill detection:** source `check-skills.sh` families from `deps.json`, reconcile with `doctor.sh`'s `dep_present`, add a success-path test (fake `~/.claude/skills/superpowers`), and probe feature-dev's actual subagents.
9. **Upgrade the compounding store** toward CE-style categorized entries (category frontmatter + staleness review) if lessons volume grows — flat `lessons.md` is the weakest layer vs competition.
10. **Small drift fixes:** `detect-context.sh` block message omits `setup.apply` (`:108`) and stale header (`:15-18`); `regen-active-rules.sh` digest sort lacks `-s` (stability); `flow/README.md` says "eight scripts" (there are nine — `merge-settings.sh` missing); `setup.md` adapter "default" wording contradicts `adapters.json` (`default:false`).

---

## Findings, part 2 — orchestration quality: how the flow combines superpowers, feature-dev, and caveman

*(Follow-up analysis, same date. Method: the actual upstream contracts were fetched
from obra/superpowers `skills/*/SKILL.md`, anthropics/claude-plugins-official
`plugins/feature-dev/` (command + three agent definitions), and
JuliusBrussee/caveman, then compared per state against vibe's phase files, orders,
and spec templates.)*

### Verdict

**The skill selection is right; the integration is nominal.** Every state names
the correct tool from the upstream libraries, and the three dependencies occupy
complementary axes — superpowers supplies *process* (how to brainstorm, plan,
TDD, debug, verify), feature-dev supplies *parallel analyst subagents* (text-only
explorers/architects/reviewers), caveman supplies *output economy*. Combining
them is coherent, not redundant. But vibe mostly integrates by *naming* the
delegate, not by *contracting* with it: it says who to call and where outputs go,
and stays silent about the parts of the upstream skill's own contract that
conflict — their hardcoded artifact paths, self-commits, chain handoffs, and
"don't pause" behaviors. The model is left to reconcile contradictions, and the
upstream text is stronger-worded (superpowers is deliberately
persuasion-hardened) than vibe's ~200-byte orders. In a conflict, the delegate's
MUST-language wins — so vibe must avoid conflicts explicitly rather than assume
routing authority it never declares.

### Per-state fit

| State | Delegates | Fit | Collision found |
|---|---|---|---|
| `idle` | using-superpowers | ⚠ risky | **Two-routers problem.** using-superpowers demands skill invocation "BEFORE any response… not negotiable" and gateways into superpowers' own chain (brainstorming → writing-plans → executing-plans → finishing). vibe's cursor claims the same first-move authority. No precedence is declared anywhere. |
| `strategy.brainstorm` | brainstorming | ⚠ contradiction | vibe: "scratch only, no spec writes yet" (`writes: []`). Upstream brainstorming **ends by writing and committing** `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`, then invokes writing-plans. The state forbids what the delegate's terminal steps require, and the handoff skips `strategy.spec` entirely. |
| `feature.design` | brainstorming, code-explorer, code-architect | ✓ good | brainstorming's design-doc write redirects legally into `.spec/features/<name>/` (path is stated upstream, not validated), but its **self-commit** and writing-plans handoff still fight the flow. feature-dev agents are fully redirect-safe (text-only, no file writes). Unused upside: `/feature-dev` dispatches **2–3 explorers/architects in parallel**, each architect committing to one approach — vibe delegates singles and loses the compare-approaches value. |
| `feature.plan` | writing-plans, code-architect | ⚠ format collision | **The biggest one.** writing-plans is the *most* redirect-friendly skill (documented optional storage location → `.spec/features/<x>/plan.md` is contract-legal). But its mandatory format (header with `> For agentic workers…`, `## Global Constraints`, ~5 checkbox steps per 2–5-minute task, real code in steps) and the spec `feature-plan` template (`{name}/n` stable units, R-ID trace, per-unit Verification) are **two different grammars — and executing-plans parses the upstream one.** vibe says nothing about the mapping, so the plan that feature.impl consumes satisfies neither parser fully. |
| `feature.impl` | executing-plans, TDD | ✓ / ⚠ | TDD: perfect, zero collision (both sides keep code/commands byte-exact). executing-plans: consumes any plan path, but (a) expects writing-plans' checkbox format, (b) assumes a **git worktree** exists (`using-git-worktrees` is a listed dependency — vibe never mentions branches/worktrees anywhere), (c) its terminal step invokes finishing-a-development-branch, **bypassing feature.verify**. |
| `feature.verify`, `quick.verify` | verification-before-completion, requesting-code-review, code-reviewer, systematic-debugging | ✓ / ⚠ | verification-before-completion: perfect philosophical fit, no artifacts. **Review double-booking:** requesting-code-review dispatches its *own* reviewer subagent from its bundled template (severity tiers); feature-dev's code-reviewer is a *different* protocol (0–100 confidence, ≥80 threshold). Both are listed side by side with no arbitration. Also upstream says "fix Critical immediately" while vibe's verify states forbid src writes — findings must route to `feature.impl`, and no one says so. |
| `feature.compound` | finishing-a-development-branch, spec | ⚠ inconsistency | `flow/compound.md` step 4 reads as if the skill performs the archive move; `spec/feature.md` gets the boundary right ("handles the narrow git-cleanup step after the spec work is done… Don't hand it the full compound procedure — it doesn't know the spec format"). The two halves disagree. Ordering is also tangled: the archive must be deleted *before merge*, but finishing (which merges) is delegated mid-procedure. |
| `quick.triage` | systematic-debugging | ✓ excellent | "Diagnose, do not fix yet" maps exactly onto the upstream Phase 1–3 / Phase 4 boundary. Could cite it ("stop at end of Phase 3") for precision. |
| `quick.fix` | TDD | ✓ excellent | One reproducing test per fix = RED discipline. |
| `setup.apply` | spec, superpowers:writing-skills | ? unclear | writing-skills is a TDD-style *skill-authoring* skill ("no skill without a failing test"). Why setup.apply delegates it is stated nowhere — the orders block never mentions it. Vestigial or unexplained. |
| `amend` | spec, receiving-code-review | ✓ fine | The no-sycophancy verify-before-acting protocol fits feedback-driven amends. |

**caveman:** orthogonal by construction — a style layer with byte-exact carveouts
for code/commands/paths (mirroring vibe's own `safety_carveouts`), so it cannot
corrupt what vibe depends on. Two unstated edges: (1) if the actual skill is
installed, its auto-activation and `/caveman [level]` selection could fight
vibe's per-state *frozen* levels — precedence undeclared; (2) `/caveman-compress
<file>` rewrites files in place — pointed at an unguarded `.spec/features/**`
doc it would mangle template structure the validator needs.

**Deliberate-looking but undocumented exclusion:** vibe delegates
`executing-plans`, not `subagent-driven-development` — upstream's *recommended*
executor. That is the right call (SDD is the one skill that actively fights
redirection: git-root-anchored `.superpowers/sdd/progress.md` ledger,
script-generated brief/report paths, per-session worktrees, and "do not pause
between tasks" — the exact opposite of cursor discipline). But the exclusion is
silent; nothing stops an agent following upstream's own recommendation into SDD.

### The systemic issue: chain ownership

superpowers is built as a **self-chaining pipeline** — each skill's terminal step
names the next skill, and using-superpowers mandates entry. vibe wants the cursor
to own sequencing. Every seam where an upstream terminal step points somewhere
else (brainstorming → writing-plans, executing-plans → finishing-a-development-branch)
is a place the agent gets pulled out of the state machine, and no text anywhere
says who wins. The spec half already discovered the right pattern (constraint
injection + offer-first + explicit boundary notes); the flow half never adopted
it — its delegate call sites are bare names.

### Improvements (ranked)

1. **Declare precedence once, visibly.** In `flow/SKILL.md` (and the AGENTS.md
   template block): *"The cursor owns sequencing and artifacts. Delegated skills
   execute within the current state's scope: ignore their artifact-path
   conventions, self-commit steps, and next-skill handoffs; all writes go to the
   state's write surface; transitions only via set-state.sh."* One paragraph
   retires the whole conflict class (two-routers, brainstorm self-commit,
   executing-plans' exit, SDD pull).
2. **Per-delegate contract blocks in the flow phase files**, adopting
   `spec/feature.md`'s Superpower-tip pattern and extending it: inputs to inject
   (template + constraints), outputs to redirect (exact `.spec` path), upstream
   steps to *skip* (brainstorming's doc-write+commit in `strategy.brainstorm`;
   executing-plans' finishing handoff), and the offer-first / self-suffice
   fallback (closing the lessons.md gap from part 1).
3. **Resolve the plan-format collision with a hybrid template**: keep the spec
   `{name}/n` unit structure as canonical, embed writing-plans-style checkbox
   steps *inside each unit* (unit = upstream task group; 2–5-minute tasks =
   checkboxes), and keep the `## Global Constraints` header seam so
   executing-plans can still parse it. This is the single fix that makes the
   design→plan→impl chain contract-clean end to end.
4. **Fix the brainstorm-state contradiction**: scope `strategy.brainstorm` to the
   dialogue phases of upstream brainstorming; the design artifact lands in the
   *next* state's write surface (`strategy.spec` root docs / `feature.design`
   feature docs). Suppress the self-commit.
5. **Unify review dispatch in `verify.md`**: one sentence — use
   requesting-code-review's dispatch protocol with feature-dev's `code-reviewer`
   as the reviewer (confidence ≥80); findings route to `feature.impl`/`quick.fix`,
   never fixed in verify.
6. **Align `flow/compound.md` with `spec/feature.md`'s boundary note** and
   sequence finishing-a-development-branch *last* (after archive + delete
   prompt), since it merges the branch.
7. **Take a worktree stance at `feature.impl`**: either adopt
   `superpowers:using-git-worktrees` on entry or state "current branch, no
   worktree — tell executing-plans so."
8. **Document the SDD exclusion** (one line in deps.json or feature.md) and
   either justify or drop `writing-skills` from `setup.apply`'s delegates.
9. **Add a delegate-consistency test**: the machine's `delegates` arrays, the
   orders blocks, and the phase-file prose are three copies of the same fact
   (e.g. setup.apply's writing-skills appears only in the machine). Assert
   machine ⊆ phase file.
10. **caveman precedence note**: vibe's frozen per-state levels are canonical
    when the skill is installed; never run `/caveman-compress` against `.spec/**`.

## Decision

The review supports keeping the three-dependency composition (the axes are
complementary and the per-state selections are correct) and **not** adopting
`/feature-dev`-as-macro or SDD. The follow-up work it points to, in order:
(1) the precedence declaration + per-delegate contract blocks (items 1–2 above —
also closes part 1's offer-first gap), (2) the hybrid plan template (item 3),
(3) the part-1 fix-first list (Stop-gate teeth, Bash-hole honesty, hermetic flow
tests, `.spec` re-sweep, quick-compound path). Findings that should outlive this
folder: the precedence rule and the "contract, don't just name, your delegates"
pattern belong in a lesson at the next compound.

## Appendix: machinery inventory

| Layer | LOC |
|---|---|
| `flow/scripts/*.sh` (9) | 1,252 |
| `.claude/hooks/*.sh` (3) | 148 |
| `install.sh` | 337 |
| JSON (machine 174 + adapters/deps/example/settings) | 264 |
| Flow markdown (SKILL + 7 phase files + READMEs) | 616 |
| Flow test suites | 666 |
| **Flow half total** | **≈ 3,280** |

Orders payload per turn: 143 B (`quick.fix`) – 332 B (`feature.compound`); byte-stable, `<feature>` is the only interpolation; cursor timestamp never injected (prompt-cache safe).
