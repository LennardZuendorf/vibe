---
name: spec-health
description: Structural health assessor for .spec/ tree — checks staleness, alignment, missing sections, and spec drift beyond what validate.sh covers.
user-invocable: true
allowed-tools:
  - Read
  - Glob
  - Grep
  - Bash
---

# spec-health

Structural health assessor. Reads the full `.spec/` tree and produces a prioritised list of structural problems beyond what `validate.sh` catches.

## Orders

1. Run `bash .agents/skills/spec/scripts/validate.sh` and capture output — include all WARN/ERROR lines in the report
2. Read root `product.md`, `tech.md`, `plan.md`, `lessons.md`
3. Check: are all features in `.spec/features/` listed in root `plan.md` Feature Sequence?
4. Check: do any feature folders have no corresponding plan entry?
5. Check: are any feature folder `plan.md` files missing unit IDs (`### <name>/n`)?
6. Check: do any root specs exceed 300 lines (likely over-loaded with feature detail)?
7. Check: are any `updated:` dates more than 30 days old on files with recent git activity?

## Output format

```
## Spec Health Report

### CRITICAL
- <issue>: <file> — <what to do>

### WARN
- <issue>: <file> — <what to do>

### INFO
- <issue>: <file> — <what to do>
```

CRITICAL: validate.sh errors, missing required feature files, orphaned features.
WARN: stale dates, oversized root specs, missing plan entries, tagless lessons.
INFO: features with no plan.md, very small specs (may be incomplete).

## Invariants

- MUST run validate.sh and include its output
- MUST check: feature-plan sequence alignment, oversized root specs, missing unit IDs
- Output MUST use CRITICAL / WARN / INFO levels
- MUST NOT modify any files
