---
type: tech
applies:
  - "spec/hooks/**"
  - ".claude/settings.json"
  - "plugin/hooks/**"
  - "crates/vibe-spec/**"
updated: 2026-11-12
---

# Hooks

Spec hooks need no flow. Hooks run through the shim `spec/hooks/run-hook.sh
<event>`. It resolves the binary from `$VIBE_SPEC_BIN`, then `vibe-spec` on
`PATH`, then the newer executable in `$CLAUDE_PROJECT_DIR/target/{release,debug}`. With no
binary it prints one stderr line and exits 0. Only `guard` exit 2 with
exactly one stderr line beginning `blocked: ` passes through as a block.
Other failures warn and exit 0.

Wiring: `build-plugin.sh` writes `plugin/hooks/hooks.json`, and this repo
carries `.claude/settings.json`. Both wire the same four events with
`"timeout": 10`. The plugin manifest does not register hooks. Flow's guard and
the spec tooth are independent: both must allow.

| Event | Matcher | Command |
|---|---|---|
| SessionStart | `startup\|clear\|compact` | `hook session-start` |
| PreToolUse | `Edit\|Write\|MultiEdit\|NotebookEdit\|Bash` | `hook guard` |
| PostToolUse | `Edit\|Write\|MultiEdit` | `hook validate`: `validate --file`, 500 ms work budget |
| Stop | none | `hook validate`: `validate --changed`, 2 s work budget |

`validate --changed` keeps the findings of changed spec files from `git status
--porcelain=v1 -z --untracked-files=all`, falls back to mtime against
`.vibe/spec-session`, and keeps tree-level findings. `validate --file <path>`
keeps one file's findings. `--format hook` prints hook JSON. Validate hooks
always exit 0.
These limits bound validation work, not managed cleanup.
A timeout cancels and joins an admitted managed worker.
In-flight Git commands and publication bookkeeping can exceed the budget.
Unmanaged external lint keeps its fast timeout.

Router: `hook session-start` prints the `using-spec` skill inside
`<EXTREMELY_IMPORTANT>`, shaped for Claude, Cursor or generic hosts. It
touches `.vibe/spec-session` and never writes through a symlink.

Tooth (D47): `hook guard` exits 2 with one stderr line when the target is
under the spec root, is `.vibe/spec.json`, or is under
`docs/superpowers/specs/` or `docs/superpowers/plans/` (other paths under
`docs/superpowers/` are not guarded), and the transcript holds no load of the
matching skill. Spec root and `.vibe/spec.json` need `spec`; `specs/` needs
`superpowers:brainstorming` or `spec`; `plans/` needs
`superpowers:writing-plans` or `spec`. A load is a `Skill`
tool call or a `<command-name>` record, in the main transcript or the
subagent's own. Prose and tool results never count. It fails open (exit 0, one
stderr line) on malformed input, an unreadable transcript, no spec root or a
target outside the project. `.vibe/spec.json` `"tooth": "warn"` downgrades a
block to a warning. A Bash command that writes a target gets a warning, not a
block.
