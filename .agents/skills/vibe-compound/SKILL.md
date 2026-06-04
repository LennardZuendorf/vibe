---
name: vibe-compound
description: |
  Consolidate at end of work: record tagged lessons, promote cross-cutting
  decisions into root specs, archive the feature, regenerate the active-rules
  digest. Serves feature.compound and strategy.compound. Trigger on: compound,
  wrap up, record lessons, finish the branch, promote to specs, archive feature.
user-invocable: true
argument-hint: ""
allowed-tools: Read, Edit, Write, Bash
compatibility: Requires bash + jq. macOS and Linux.
metadata:
  author: lennarddib
  version: "1.0"
---

# vibe-compound — consolidate & learn

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
   `.spec/features/<feature>/` → `.spec/archive/<feature>/`.
5. **Regenerate digest.** Run `bash .agents/flow/scripts/regen-active-rules.sh` to
   refresh the managed active-rules block in `CLAUDE.md`/`AGENTS.md` from the
   updated lessons (capped top-5, pinned first). Graceful degrade: if the script
   errors, warn — `lessons.md` is still written.
6. **Receipt (ultra).** Emit a compact receipt: `lesson +1 → <tag>`, promoted
   files, archived path, `digest refreshed`. Then `set-state.sh idle`.

## Rules

- `regen-active-rules.sh` is the only writer of the active-rules block; never
  hand-edit inside its markers.
- Caveman lite body, ultra receipts. Security/irreversible actions normal prose.
