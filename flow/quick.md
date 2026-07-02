# quick — small fixes

The low-ceremony path. Its existence keeps heavy process off vibe-sized work.
States: `quick.triage → quick.fix → quick.verify → idle`.

## Procedure

1. **Locate.** Read `.agents/skills/vibe/state.json`. Enter with
   `bash .agents/skills/vibe/scripts/set-state.sh quick.triage`.
2. **Triage** (caveman **full**, never ultra — triage is where a dropped edge
   case is expensive). Read `.spec/lessons.md` first. Delegate to
   `superpowers:systematic-debugging`. Diagnose; do **not** fix yet.
   - **Escalation check:** if the fix is no longer small and bounded, stop and
     suggest `feature.design` instead of stretching quick mode.
3. **Fix** (`quick.fix`, caveman full). Delegate to
   `superpowers:test-driven-development`. Write `src/**`; optionally jot a
   `.spec/quick/<slug>.md` note. No root spec writes.
4. **Verify** (`quick.verify`, caveman full). Hand to `verify.md`:
   `superpowers:verification-before-completion` + `code-reviewer`. Gather evidence
   the fix works and breaks nothing, then `idle`.

## Rules

- Quick mode does not write feature specs or root specs.
- One reproducing test per fix.
- Transitions agent-suggested; confirm before `set-state.sh`.
- Caveman compresses output, not reasoning. Security/irreversible actions in
  normal prose.
