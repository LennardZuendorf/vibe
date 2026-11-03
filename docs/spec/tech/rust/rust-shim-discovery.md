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

A hook uses 0 for pass/degrade and 2 for block. The shim accepts a block
only when the producer returns 0 or 2 and its complete output matches the
block grammar: one `block:<reason>` line, a guard block with its exact
optional advice line, or a gate block with indented detail lines. Reasons
must be nonempty. Banners, extra lines, reasonless blocks and leading
whitespace cannot prove a block. Diagnostic values escape backslash and
C0/DEL bytes so filenames cannot manufacture output structure.
Unexpected status retries the frozen fallback with the same payload;
its output must pass the same proof. Unknown events read no stdin and exit 0.

`flow/tests/hooks.sh` is the test surface: the five-tier matrix
(`f6/tier1`–`f6/tier5`), the crash/proved-block pair, the corrupt-policy
fail-safe with its control, and the `f6/notier` legs (no binary, no engine,
no fallback).
