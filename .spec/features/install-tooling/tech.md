---
type: feature-tech
feature: install-tooling
sibling: product.md
parent: ../../tech.md
updated: 2026-07-03
---

# Feature: Install Tooling — Architecture

Extend `install.sh` with an arg parser (flags compose: `--only spec --dry-run`
valid), add `flow/scripts/doctor.sh` following the `check-skills.sh` warn-only
pattern, and introduce `flow/reference/deps.json` as the single dependency
manifest. Everything bash, shellcheck-clean, graceful-degrade.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## Files

```
install.sh                      # arg parser, action plan model, DRY_RUN guard on every write
flow/scripts/doctor.sh          # new — health checks, warn-only, exit 0
flow/reference/deps.json        # new — external dep manifest
flow/tests/adapters/run.sh      # flag + uninstall + dry-run assertions
flow/tests/run.sh               # doctor assertions
```

---

## Contract / API

```bash
install.sh <target> [--only spec|flow] [--dry-run] [--uninstall] [--adapters claude,warp]
# --uninstall composes with --only (remove one half) and --dry-run (preview removal)

doctor.sh [<repo-root>]         # default: resolved repo root
# output: "ok|warn <check-id> <message>" per line; exit 0 always
```

```json
// flow/reference/deps.json
{ "deps": [ { "name": "superpowers", "kind": "skill-collection",
  "source": "<url>", "required_by": ["feature.*", "strategy.*", "quick.*"],
  "degrade": "phases self-execute from constraint docs" } ] }
```

---

## Implementation Detail

**Install actions as data.** Refactor install.sh's copy/seed/merge steps into
an ordered list of (kind, src, dst) actions; executor honors `DRY_RUN=1` by
printing instead of doing. Uninstall = inverse manifest of managed paths —
derived from the same action list so the two can't drift.

**Uninstall safety.** Managed AGENTS.md block removed via existing
marker-pairing validation in `merge-agents.sh` (reuse, don't reimplement —
lessons.md marker rule applies). Cursor: prompt unless `--yes`; never silently
delete runtime state (lessons.md installer rule applies).

**Doctor checks (initial set):** core dirs present; symlinks resolve (source
repo) / real dirs (target); `state.json` parses + state legal
(delegates `validate-state.sh`); `.claude/settings.json` hook wiring present when
flow installed (the plugin manifest approach was retired); each `deps.json` entry
present in `~/.claude/skills` or plugin cache — warn with degrade text when
absent; `jq` availability.

## Open Questions

1. **deps.json accuracy** — required_by phase lists must be derived from grep
   of skill bodies at authoring time; keep coarse (`feature.*`) not per-phase
   to avoid drift.
