---
type: platform
updated: 2026-10-19
---

# Adapter Contract

| Platform | spec | flow | instruct |
|---|---|---|---|
| Claude Code | `SessionStart` router, `PreToolUse` guard (blocking tooth), `Stop` (`validate --changed`), `PostToolUse` (`validate --file`) | `PreToolUse` guard, `PostToolUse` sniffer + Task redirect, `Stop` tooth, `SubagentStop`, `SessionEnd` | `SessionStart` composition (+compact/resume), `UserPromptSubmit`, `PreCompact` reset |
| OpenCode | `tool.execute.after` shim | `tool.execute.before/after`, `event` | `chat.message`, `system.transform`, `session.compacting`, `config` |
| Codex / other AGENTS.md readers | reads committed spec tree | reads AGENTS.md flow block; no enforcement | reads committed AGENTS.md instruct block |

Each plugin ships `bin/hook.sh`, a POSIX sh shim invoked as
`"${CLAUDE_PLUGIN_ROOT}/bin/hook.sh" <event>`. Discovery order:
`$VIBE_<TOOL>_BIN` → `$CLAUDE_PLUGIN_ROOT/bin/<os>-<arch>/vibe-<tool>` →
`command -v vibe-<tool>` → (flow guard/stop only) frozen bash
`bin/fallback/<event>.sh` → exit 0 with one stderr line. Never downloads.
Full discovery, fallback set, and exit codes: [tech-rust.md](../tech/rust.md).
