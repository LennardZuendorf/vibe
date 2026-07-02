# amend — scope correction (modifier)

`amend` is a **modifier**, not a flow. It does not change the cursor. It makes a
targeted scope edit from whatever state you are in, then you continue in that
same state. Caveman **lite**.

## Procedure

1. **Locate.** Read `.agents/skills/vibe/state.json` to learn the current state. Do
   **not** call `set-state.sh amend` — it is not a stored state and the writer
   rejects it.
2. **Carry the current state's write rules.** Amend edits only what the current
   state may write — it does **not** widen the write surface. Read the state's
   `writes` from `.agents/skills/vibe/state-machine.json` (or
   `bash .agents/skills/vibe/scripts/detect-context.sh`) and stay inside it:
   - in `feature.design` → amend `.spec/features/<f>/{product,tech}.md`
   - in `feature.plan` → amend `plan.md` (keep stable unit IDs)
   - in `strategy.spec` → amend root specs
   - elsewhere → the corresponding allowed surface only
3. **Edit.** Delegate to `spec` (and `superpowers:receiving-code-review` when the
   amend responds to feedback). Inject the exact path.
4. **Return.** Confirm the edit and note that the cursor is unchanged — continue
   the originating state's work.

## Rules

- Never widen the write surface beyond the current state's rules.
- Preserve stable plan unit IDs when amending a plan.
- Caveman lite; security/irreversible actions normal prose.
