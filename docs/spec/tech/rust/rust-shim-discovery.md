---
type: tech
applies:
  - "*/bin/hook.sh"
  - "*/bin/fallback/**"
  - "spec/hooks/**"
  - "flow/hooks-fallback/**"
updated: 2026-11-03
---

# Shim and discovery

`hooks.json` invokes one shim per event:
`"${CLAUDE_PLUGIN_ROOT}/bin/hook.sh" <event>`. `bin/hook.sh` is POSIX sh,
shellcheck-clean, ≤40 lines — the only shell left in the runtime path.
Discovery order:

1. `$VIBE_<TOOL>_BIN` (explicit override)
2. `$CLAUDE_PLUGIN_ROOT/bin/<os>-<arch>/vibe-<tool>` (bundled, optional)
3. `command -v vibe-<tool>` (PATH)
4. flow guard/stop only: the JS engine at
   `$CLAUDE_PROJECT_DIR/.agents/skills/vibe/engine/cli.mjs` while it still ships.
   FV-R9 keeps the engine running until F23, and it answers **before** the frozen
   bash because the fallback is the *pre-port* implementation: it still nudges on
   every Stop, and it counts vibe's own `.vibe/run/**` writes as staleness — a
   false block, i.e. a wedged session;
5. `bin/fallback/<event>.sh` (frozen bash, kept permanently; the two files are
   symlinks to `flow/hooks-fallback/`, the parity oracle, so they cannot drift)

If no tier applies: `exit 0` with one stderr line — never downloads.

Each tier is tested with the higher tiers' variables **deleted**, not
merely unset, from the child environment.

## Exit codes

The current shim recognizes a nonempty `block:` or shipped guard/gate block
line anywhere in producer output. It retries the frozen fallback after an
unexpected status. It does not yet require complete-output proof or escape
all diagnostic values; the newer F6 hardening is pending this stack refresh.
Unknown events read no stdin and exit 0.

`flow/tests/hooks.sh` is the test surface: the five-tier matrix
(`f6/tier1`–`f6/tier5`), the crash/proved-block pair, the corrupt-policy
fail-safe with its control, and the `f6/notier` legs (no binary, no engine,
no fallback).
