# quick — small fixes

The low-ceremony path. Its existence keeps heavy process off vibe-sized work.
States: `quick.triage → quick.fix → quick.verify → (quick.compound | quick.fix | idle)`.

## Procedure

1. **Locate.** Read `.agents/skills/vibe/state.json`. Enter with
   `bash .agents/skills/vibe/scripts/set-state.sh quick.triage`.
2. **Triage** (caveman **full**, never ultra — triage is where a dropped edge
   case is expensive). Read `.spec/lessons.md` first. **Branch by work type:**

   **(a) Defect** (a bug with a root cause to find) — run the systematic-debugging
   contract, diagnose only:

   > **Delegate — superpowers:systematic-debugging**
   > - announce: "delegating to `superpowers:systematic-debugging` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: diagnose only — run its Phase 1–3, STOP before Phase 4 (the fix); caveman full
   > - redirect: findings stay in-turn (nothing written); the fix lands in the NEXT state (`quick.fix`)
   > - skip: its Phase 4 fix and any commit — `quick.fix` owns the change

   **(b) Non-defect** (a flag, rename, config, or docs change — no root cause to
   find) — self-execute scoping: confirm the change is bounded, name the exact
   verify evidence you will gather in `quick.verify`, and do NOT delegate.

   - **Escalation check:** if the fix is no longer small and bounded it becomes a
     named feature — announce AND confirm before switching (it renames the work):
     "this is bigger than a quick fix — move to `feature.design` as a named
     feature?" Transition only on confirmation.
3. **Fix** (`quick.fix`, caveman full). For a **bug-shaped** change TDD applies —
   one reproducing test first, then the minimal fix:

   > **Delegate — superpowers:test-driven-development**
   > - announce: "delegating to `superpowers:test-driven-development` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: one reproducing test first (RED), then the minimal fix; caveman full
   > - redirect: write `src/**` (+ optional `.spec/quick/<slug>.md` note); no root spec writes
   > - skip: any branch-finishing or chain handoff — `quick.verify` is next

   The one-reproducing-test rule is scoped to bug-shaped changes (per TDD's own
   exceptions list — config files, pure renames, docs); a **non-defect** change
   carries no reproducing test and relies on `quick.verify` evidence instead. When
   re-entering fix from verify-routed findings, apply
   `superpowers:receiving-code-review` — verify each finding against the codebase
   before implementing; push back with technical reasoning if a finding is wrong.

4. **Verify** (`quick.verify`, caveman full). Hand to `verify.md`:
   `superpowers:verification-before-completion` + `code-reviewer`. Gather evidence
   the fix works and breaks nothing.
5. **Compound (optional)** (`quick.compound`, caveman lite; receipts ultra). Most
   quick fixes surface no durable lesson — default straight to `idle`. Only when a
   keep-worthy lesson surfaced, transition in
   (`bash .agents/skills/vibe/scripts/set-state.sh quick.compound`): append one
   tagged entry to `.spec/lessons.md` (canonical format), then
   `bash .agents/skills/vibe/scripts/regen-active-rules.sh` to refresh the digest.
   Receipt (ultra): `lesson +1 → <tag>`, `digest refreshed`. Then `idle`.

## Rules

- Quick mode does not write feature specs or root specs.
- One reproducing test per bug-shaped fix; non-defect changes (config, rename,
  docs) rely on `quick.verify` evidence instead.
- At a non-gated edge, advance immediately: `set-state.sh <next>`, announce in
  one line, continue. The only exception is the `feature.design` escalation
  (step 2) — it announces AND confirms, because it renames the work.
- Caveman compresses output, not reasoning. Security/irreversible actions in
  normal prose.
