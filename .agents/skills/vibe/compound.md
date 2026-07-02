# compound — consolidate & learn

Turns finished work into durable memory. Caveman **lite** for the body, **ultra**
for receipts. Serves `feature.compound` and `strategy.compound`; both end at
`idle`.

## Procedure

1. **Locate.** Read `.agents/flow/state.json`. Confirm a `*.compound` state — this
   is the only state where `lessons.md` and root specs are writable.
2. **Lessons.** Append a tagged entry to `.spec/lessons.md` using the canonical
   format (`### title`, `**Pattern:**`, `**Rule:**`, `**Tags:**`, `**Date:**`,
   optional `**Pinned-by:**`). Only record durable lessons — pinning is
   deliberately expensive.
3. **Promote (feature.compound only).** Merge cross-cutting decisions into root
   `.spec/{product,tech,design,plan}.md` via the `spec` skill. Feature-specific
   detail stays in the archive, not in root specs.
4. **Archive (feature.compound only).** Delegate to
   `superpowers:finishing-a-development-branch`. Move
   `.spec/features/<feature>/` → `.spec/archive/<feature>/`. Archive is a
   **transient safety net** (e.g. CI fails right after wrapup) — never a store for
   active work. CODE IS TRUTH.
5. **Regenerate digest.** Run `bash .agents/flow/scripts/regen-active-rules.sh` to
   refresh the managed active-rules block in `CLAUDE.md`/`AGENTS.md` from the
   updated lessons (capped top-5, pinned first). Graceful degrade: if the script
   errors, warn — `lessons.md` is still written.
6. **Prompt to delete archive (feature.compound only).** After validation passes,
   prompt the user to delete `.spec/archive/<feature>/`. The folder should be gone
   **before the branch merges**; keeping it is the justified exception (standalone
   decision archaeology), not the default.
7. **Receipt (ultra).** Emit a compact receipt: `lesson +1 → <tag>`, promoted
   files, archived path, `delete prompted`, `digest refreshed`. Then
   `set-state.sh idle`.

## Rules

- `regen-active-rules.sh` is the only writer of the active-rules block; never
  hand-edit inside its markers.
- Archive is cold and transient; never read it for active work. CODE IS TRUTH.
- Caveman lite body, ultra receipts. Security/irreversible actions normal prose.
