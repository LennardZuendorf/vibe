# strategy — direction loop

The strategy flow shapes project direction. It reads lessons, brainstorms, and
writes the **root** `.spec/` docs. It never writes source code.

States: `strategy.brainstorm → strategy.spec → (strategy.compound | idle)`.

## Procedure

1. **Locate.** Read `.agents/skills/vibe/state.json`. If not already in a `strategy.*`
   state, transition in: `bash .agents/skills/vibe/scripts/set-state.sh strategy.brainstorm`.
2. **Read lessons first.** On entering `strategy.brainstorm`, read
   `.spec/lessons.md` so past mistakes shape direction (retrieval, not just
   recording).
3. **Brainstorm.**

   > **Delegate — superpowers:brainstorming**
   > - announce: "delegating to `superpowers:brainstorming` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: this state's scope — its DIALOGUE phases only (direction-shaping questions); caveman lite
   > - redirect: nothing written here; the direction artifact lands in the NEXT state (`strategy.spec` root docs)
   > - skip: its terminal design-doc write (`docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`), its self-commit, its writing-plans handoff

   Scratch only — no spec writes yet. When direction is clear, suggest:
   "direction looks set — move to strategy.spec?" and transition on confirmation.
4. **Spec.** In `strategy.spec`, delegate to the `spec` skill. Write **only** the
   root docs: `.spec/product.md`, `.spec/tech.md`, `.spec/design.md`,
   `.spec/plan.md`. Inject those exact paths. Do not touch `lessons.md` or source.
   Validate with `bash .agents/skills/spec/scripts/validate.sh`.
5. **Compound (conditional).** Most strategy runs surface no durable lesson. Only
   if one did, suggest `strategy.compound` (delegates to `compound.md`); else go
   to `idle`. Trust the agent to pick — this is "trust the agent" applied to
   control flow.

## Rules

- Caveman **lite** throughout (full sentences, no filler).
- Transitions are agent-*suggested*, not automatic: name the next state and
  confirm before calling `set-state.sh`.
- Keep receipts short: state, files written, validation result, next transition.
- Security warnings and irreversible actions stay in normal prose.
