#!/usr/bin/env bash
# install.sh — install the vibe workflow harness into a target repo.
#
#   ./install.sh <target-repo-root> [--adapters claude,warp]
#
# What it does (OPEN-3 default = COPY the platform-neutral core; MERGE the
# instruction file with markers; never blind-overwrite user content):
#   1. Copy the core   .agents/skills/** (skills + vibe flow engine) into the target.
#   2. Copy the Claude adapter   .claude/commands, .claude/hooks, .claude-plugin/.
#   3. Seed   .agents/skills/vibe/state.json from state.example.json if absent.
#   4. Ignore the mutable cursor in the target .gitignore.
#   5. Merge   AGENTS.md via the target's merge-agents.sh (agent-instructions).
#   6. Symlink requested adapters (CLAUDE.md, WARP.md) — opt-in, never clobber.
#   7. Print how to register the Claude Code plugin so the hooks go live.
#
# Re-runnable: refreshes the managed core (.agents/**, .claude/** adapter) from
# source each run, preserves the target's live flow cursor (state.json) and any
# content outside the managed AGENTS.md markers. Managed core files are owned by
# vibe — local edits to them are replaced on re-run.

set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

err() { echo "install: $1" >&2; }
note() { echo "install: $1"; }

TARGET=""
ADAPTERS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --adapters) ADAPTERS="${2:-}"; shift 2 ;;
    --adapters=*) ADAPTERS="${1#*=}"; shift ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) TARGET="$1"; shift ;;
  esac
done

if [[ -z "$TARGET" ]]; then
  err "ERROR: target repo root required. Usage: ./install.sh <target-repo-root> [--adapters claude,warp]"
  exit 1
fi
mkdir -p "$TARGET"
TARGET="$(cd "$TARGET" && pwd)"

if [[ "$TARGET" == "$SRC" ]]; then
  err "ERROR: target is the vibe source repo itself; nothing to install."
  exit 1
fi

# 1. Core (platform-neutral). Copy the flow machine + scripts and all skills.
# The cursor is per-project runtime state: never inherit the source's, never
# clobber the target's. Preserve any live target cursor across the copy.
note "copying core .agents/ into $TARGET"
mkdir -p "$TARGET/.agents"
SAVED_CURSOR=""
if [[ -f "$TARGET/.agents/skills/vibe/state.json" ]]; then
  SAVED_CURSOR="$(mktemp)"
  cp "$TARGET/.agents/skills/vibe/state.json" "$SAVED_CURSOR"
fi
cp -R "$SRC/.agents/skills" "$TARGET/.agents/"
if [[ -n "$SAVED_CURSOR" ]]; then
  mv -f "$SAVED_CURSOR" "$TARGET/.agents/skills/vibe/state.json"
  note "preserved existing flow cursor across re-install"
else
  rm -f "$TARGET/.agents/skills/vibe/state.json"
fi

# 2. Claude Code adapter (command + hooks + plugin manifest).
note "copying Claude adapter (.claude/, .claude-plugin/)"
mkdir -p "$TARGET/.claude"
cp -R "$SRC/.claude/commands" "$TARGET/.claude/"
cp -R "$SRC/.claude/hooks" "$TARGET/.claude/"
mkdir -p "$TARGET/.claude-plugin"
cp "$SRC/.claude-plugin/plugin.json" "$TARGET/.claude-plugin/"
chmod +x "$TARGET"/.claude/hooks/*.sh \
         "$TARGET"/.agents/skills/spec/scripts/*.sh \
         "$TARGET"/.agents/skills/vibe/scripts/*.sh 2>/dev/null || true

# 3. Seed the cursor if absent.
if [[ ! -f "$TARGET/.agents/skills/vibe/state.json" ]]; then
  cp "$TARGET/.agents/skills/vibe/state.example.json" "$TARGET/.agents/skills/vibe/state.json"
  note "seeded .agents/skills/vibe/state.json from template"
fi

# 4. Ignore the mutable cursor.
GI="$TARGET/.gitignore"
if ! { [[ -f "$GI" ]] && grep -qF ".agents/skills/vibe/state.json" "$GI"; }; then
  printf '\n# vibe mutable flow cursor (runtime; version state-machine.json, not this)\n.agents/skills/vibe/state.json\n' >> "$GI"
  note "added .agents/skills/vibe/state.json to .gitignore"
fi

# 5. Merge AGENTS.md via the copied merge script (agent-instructions).
MERGE="$TARGET/.agents/skills/vibe/scripts/merge-agents.sh"
if [[ -f "$MERGE" ]]; then
  bash "$MERGE" "$TARGET"
else
  err "WARN: merge-agents.sh not found; skipping AGENTS.md merge."
fi

# 6. Opt-in adapter symlinks.
if [[ -n "$ADAPTERS" ]]; then
  IFS=',' read -r -a chosen <<< "$ADAPTERS"
  for a in "${chosen[@]}"; do
    case "$a" in
      claude) bash "$MERGE" link "CLAUDE.md" "$TARGET" || err "WARN: CLAUDE.md not linked (real file?)." ;;
      warp)   bash "$MERGE" link "WARP.md" "$TARGET"   || err "WARN: WARP.md not linked (real file?)." ;;
      *)      err "WARN: unknown adapter '$a' (known: claude, warp)." ;;
    esac
  done
fi

# 7. Plugin registration guidance (the hooks go live once the plugin is loaded).
cat <<EOF
install: done.
install: to activate the Claude Code hooks + /flow command, register the plugin:
install:   - local dev:  add "$TARGET" as a plugin (it has .claude-plugin/plugin.json)
install:   - the inject/guard/gate hooks read .agents/skills/vibe via \${CLAUDE_PROJECT_DIR}.
install: the spec + vibe skills are installed as project files under .agents/skills/.
EOF
