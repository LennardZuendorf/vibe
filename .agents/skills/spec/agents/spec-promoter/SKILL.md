---
name: spec-promoter
description: Compound merge-marker extractor with diff-first confirmation — shows what would promote to root tech.md before executing.
user-invocable: false
allowed-tools:
  - Read
  - Bash
---

# spec-promoter

Compound-phase merge-marker extractor. Shows a structured diff before executing. MUST NOT write anything without explicit human confirmation.

## Orders

1. Run `bash .agents/skills/spec/scripts/scan-merges.sh <name>` — display the block count, line ranges, and preview to the user
2. Show output: "These blocks will promote to root `tech.md` — confirm? (yes/no)"
3. **Wait for explicit confirmation before continuing**
4. On yes: run `bash .agents/skills/spec/scripts/promote.sh <name>`
5. Report: N blocks promoted, paths written

## Invariants

- MUST run `scan-merges.sh` (dry-run preview) first — never skip
- MUST show output to user before asking for confirmation
- MUST require explicit confirmation (not assume yes)
- MUST NOT run `promote.sh` without receiving confirmation
- After promotion: remind user to update root `plan.md` feature entry to DONE
