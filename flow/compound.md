# compound — consolidate & learn

Turns finished work into durable memory. Serves `feature.compound` only — the
full end-of-feature consolidation (promote, archive, branch close-out); it ends
at `idle`. Strategy and quick runs record their optional lesson inline in
`strategy.spec` / `quick.verify` (see [strategy.md](strategy.md), [quick.md](quick.md)),
not here.

## Procedure

1. **Locate.** Read `.agents/skills/vibe/state.json`. Confirm `feature.compound`.
   Root `.spec/{product,tech,design,plan}.md` and `.spec/lessons.md` are writable
   here.
2. **Lessons.** Append a tagged entry to `.spec/lessons.md` using the canonical
   format (`### title`, `**Pattern:**`, `**Rule:**`, `**Tags:**`, `**Date:**`,
   optional `**Pinned-by:**`). Only record durable lessons — pinning is
   deliberately expensive.
3. **Promote (feature.compound only).** Merge cross-cutting decisions into root
   `.spec/{product,tech,design,plan}.md` via the `spec` skill. Feature-specific
   detail stays in the archive, not in root specs.
4. **Archive (feature.compound only).** The flow performs this move itself — it is
   **not** the delegate's job: relocate `.spec/features/<feature>/` →
   `.spec/archive/<feature>/`. Archive is a **transient safety net** (e.g. CI fails
   right after wrapup) — never a store for active work. CODE IS TRUTH.
5. **Regenerate digest.** Run `bash .agents/skills/vibe/scripts/regen-active-rules.sh` to
   refresh the managed active-rules block in `CLAUDE.md`/`AGENTS.md` from the
   updated lessons (capped top-5, pinned first). Graceful degrade: if the script
   errors, warn — `lessons.md` is still written.
6. **Prompt to delete archive (feature.compound only).** After validation passes,
   prompt the user to delete `.spec/archive/<feature>/`. The folder should be gone
   **before the branch merges**; keeping it is the justified exception (standalone
   decision archaeology), not the default.
7. **Finish the branch (feature.compound only — LAST).** All spec work above
   (lessons, promote, archive move, digest, delete-prompt) is the flow's own and is
   now done; only the branch close-out remains. Sequenced last because it merges.

   > **Delegate — superpowers:finishing-a-development-branch**
   > - announce: "delegating branch close-out to `superpowers:finishing-a-development-branch` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: the narrow git-cleanup only — merge/branch lifecycle; the spec work is already complete
   > - redirect: nothing to `.spec` — it touches only git; the archive move was the flow's own work above, not this skill's
   > - skip: the full compound procedure — it does not know the spec format (see [spec feature.md](.agents/skills/spec/feature.md) § Compound note)

8. **Receipt.** Emit a compact receipt: `lesson +1 → <tag>`, promoted files,
   archived path, `delete prompted`, `branch finished`, `digest refreshed`.
   Then `set-state.sh idle`.

## Rules

- `regen-active-rules.sh` is the only writer of the active-rules block; never
  hand-edit inside its markers.
- Archive is cold and transient; never read it for active work. CODE IS TRUTH.
- Output density follows the machine's `style` note (see [SKILL.md](SKILL.md)
  § Style): compress receipts and summaries. Security/irreversible actions stay
  in normal prose.
