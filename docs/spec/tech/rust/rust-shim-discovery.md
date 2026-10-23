---
type: tech
applies:
  - "*/bin/hook.sh"
  - "*/bin/fallback/**"
  - "spec/hooks/**"
  - "flow/hooks-fallback/**"
updated: 2026-10-19
---

# Shim and discovery

`hooks.json` invokes one shim per event:
`"${CLAUDE_PLUGIN_ROOT}/bin/hook.sh" <event>`. `bin/hook.sh` is POSIX sh,
shellcheck-clean, ≤40 lines — the only shell left in the runtime path.
Discovery order:

1. `$VIBE_<TOOL>_BIN` (explicit override)
2. `$CLAUDE_PLUGIN_ROOT/bin/<os>-<arch>/vibe-<tool>` (bundled, optional)
3. `command -v vibe-<tool>` (PATH)
4. flow guard/stop only: `bin/fallback/<event>.sh` (frozen bash, kept
   permanently)
5. `exit 0` with one stderr line — never downloads

Each tier is tested with the higher tiers' variables **deleted**, not
merely unset, from the child environment.
