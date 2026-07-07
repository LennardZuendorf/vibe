---
type: feature-tech
feature: flow-mvp
sibling: product.md
parent: ../../tech.md
updated: 2026-07-07
---

# Feature: flow-mvp — technical design

How the operating-layer MVP lands in the existing flow half. All changes are
data + prose + one hook + tests; no new runtimes, no machine-shape rewrite.
Contract evidence for every seam: [../vibe-flow/research.md](../vibe-flow/research.md) part 2.

## D1 — Precedence section (R1)

`flow/SKILL.md` gains a short `## Precedence` section (above Orders):

> The cursor owns sequencing and artifact destinations; delegates own method.
> When a delegated skill's text names its own artifact path, commits its own
> output, or hands off to another skill, the current state's orders win: write
> to the state's surface, leave commits to the flow, transition only via
> `set-state.sh`.

The same 4 lines go into `flow/reference/templates/AGENTS.md` inside the managed
instructions block (merge via existing `merge-agents.sh`; no new machinery).
Orders blocks stay unchanged in size; precedence is ambient, not per-turn.

## D2 — Machine data changes (R4, R5, R7, R8, R9)

`flow/state-machine.json`:

- New top-level `"gates"`: `{"feature.plan": "human approves plan units",
  "feature.verify": "human approves ship"}` — a stop-for-approval marker on the
  *source* state's exit. Everything else auto-advances.
- New state `quick.compound`: skill `vibe`, caveman lite (receipts ultra),
  reads lessons, writes `[".spec/lessons.md", "CLAUDE.md#active-rules",
  "AGENTS.md#active-rules"]`, next `["idle"]`. `quick.verify.next` becomes
  `["quick.compound", "quick.fix", "idle"]` (adds the missing fix back-edge and
  the optional compound).
- `idle.delegates`: drop `superpowers:using-superpowers` (R9).
- `setup.apply.delegates`: drop `superpowers:writing-skills` (unexplained; see
  research part 2 per-state table).
- `feature.impl.delegates`: add `superpowers:subagent-driven-development`.
- `detect-context.sh`: add `quick.compound` to the lessons-write allow list
  (the only policy change).

`validate-state.sh` / machine self-consistency tests extend to the new state and
`gates` keys (must name known states).

## D3 — Contract-block format for phase files (R2)

One fixed shape per delegation site, replacing bare "Delegate to X":

```markdown
> **Delegate — superpowers:writing-plans**
> - offer: "I can hand decomposition to `superpowers:writing-plans` — ok?"
>   (self-execute from this file if declined/absent)
> - inject: feature product.md requirements; hybrid plan template
>   (`spec/reference/templates/feature-plan.md`); stable-ID rules
> - redirect: plan → `.spec/features/<feature>/plan.md` (its documented
>   storage-location seam)
> - skip: its own `docs/superpowers/plans/` path; its exec handoff — the flow
>   advances via the gate instead
```

Sites and their `skip` payloads (from research part 2): `strategy.brainstorm`
(skip design-doc write + self-commit + writing-plans handoff — dialogue only),
`feature.design` (redirect design doc into feature spec docs; skip self-commit;
optionally dispatch 2–3 explorers/architects in parallel per feature-dev's own
pattern), `feature.plan` (above), `feature.impl` (skip
`finishing-a-development-branch` handoff → `feature.verify`), `*.verify` (one
review protocol: requesting-code-review's dispatch with feature-dev
`code-reviewer` as the reviewer, confidence ≥ 80; findings route to
`feature.impl`/`quick.fix`, never fixed in verify), `feature.compound`
(finishing-a-development-branch = narrow git-cleanup only, sequenced last,
mirroring `spec/feature.md`'s compound note).

## D4 — Hybrid plan template (R3)

`spec/reference/templates/feature-plan.md` additions (structure kept, nothing
removed): the `> For agentic workers: …` header line; a `## Global Constraints`
section after Key Technical Decisions; per-unit `**Steps:**` checkbox list
(failing test → verify fail → implement → verify pass → commit citing
`{name}/n`). Units stay canonical; Steps are the executor grammar. Template
guardrail comment notes: Steps are consumed by `superpowers:executing-plans` /
SDD — keep them real commands/code, no placeholders.

## D5 — Auto-advance prose (R4)

Phase-file rule text changes from "transitions are agent-suggested … confirm"
to: *"At a non-gated exit, advance immediately: `set-state.sh <next>`, announce
in one line, continue. Stop only at a `gates` state exit and ask."* SKILL.md
orders append `gate: plan-approval` / `gate: ship-approval` to the two gated
states' blocks (stays within byte budget).

## D6 — Evidence receipt + verify tooth (R6)

Convention: `.agents/skills/vibe/evidence/<flow>-<feature-or-slug>.md`, written
during `*.verify` step 2 (commands run + observed output + per-unit verdicts).
Directory is runtime-not-memory: gitignored next to the cursor (installer's
existing gitignore stanza gains one line).

`stop-gate.sh` change (the only teeth promotion): in a `*.verify` state —
- receipt missing → **block** (exit 2) with the write-the-receipt instruction;
- receipt present but older than the newest file under `src/ tests/` → block
  (staleness via `find src tests -type f -newer <receipt> | head -1`; portable,
  no `stat -f/-c` divergence);
- else pass. Outside `*.verify`, behavior unchanged (warn-only). jq-free and
  missing-machine degrade paths keep the existing always-exit-0 fallbacks —
  the tooth only bites when state is readable and clearly `*.verify`.

## D7 — Caveman demotion (R8)

Remove the `caveman` object from `flow/reference/deps.json`; `doctor.sh` loses
the `dep.caveman` row automatically (it iterates the manifest). `check-skills.sh
caveman <level>` stays (it prints machine-frozen definitions — vibe vocabulary).
`state-machine.json.$comment` + README dependency table gain an attribution
line; the "If absent" caveman row is deleted.

## D8 — Tests (R10)

- **Hermeticity:** `flow/tests/run.sh` copies `flow/` (deref) into its sandbox
  and points every script invocation there (same pattern the adapters suite
  already uses); the live cursor is never read or written by tests.
- **New assertions:** every `state-machine.json` delegate appears in the linked
  phase file (machine ⊆ prose); `gates` keys are known states and match the two
  gated orders blocks; stop-gate blocks in `feature.verify` without a receipt,
  passes with a fresh one, blocks on stale (touch a src file after receipt);
  `quick.compound` reachable and lessons-writable per `detect-context.sh`;
  every orders block ≤ 400 bytes; orders byte-parity via symlink path
  (existing) still green.

## File inventory

```
flow/SKILL.md                          # Precedence section, orders tweaks (gate:, impl modes)
flow/state-machine.json                # gates, quick.compound, delegate list edits
flow/{strategy,feature,quick,verify,compound}.md   # contract blocks, auto-advance rule
flow/scripts/detect-context.sh         # + quick.compound lessons allow
flow/reference/deps.json               # - caveman
flow/reference/templates/AGENTS.md     # + precedence lines
.claude/hooks/stop-gate.sh             # verify tooth (D6)
spec/reference/templates/feature-plan.md  # hybrid grammar (D4)
install.sh                             # + evidence/ gitignore line
flow/tests/run.sh                      # hermetic sandbox + new assertions
```

## Risks

| Risk | Mitigation |
|---|---|
| Verify tooth misfires (evidence exists but path/staleness heuristic wrong) | Block message names the exact expected path + `find` rule; escape hatch documented (write receipt manually); predicate covered by 3 tests before promotion |
| Contract blocks bloat phase files / orders | Blocks live in phase files only; orders keep ≤ 400 B budget under test |
| Hybrid template confuses the spec-only (no-flow) audience | Steps section marked optional-when-no-executor in the template guardrail |
| SDD ignores precedence and merges the branch | Handover-mode contract block injects "stop before finishing-a-development-branch"; residual risk accepted (personal tool, verify gate still audits after) |
