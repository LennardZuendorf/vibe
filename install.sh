#!/usr/bin/env bash
# install.sh — install the vibe workflow harness into a target repo.
#
#   ./install.sh <target-repo-root> [--only spec|flow] [--dry-run] [--adapters claude,warp]
#
# What it does (OPEN-3 default = COPY the platform-neutral core; MERGE the
# instruction file with markers; never blind-overwrite user content):
#   1. Copy the core   .agents/skills/{spec,vibe} (skills + vibe flow engine).
#   2. Copy the Claude adapter   .claude/commands, .claude/hooks, .claude-plugin/.
#   3. Seed   .agents/skills/vibe/state.json from state.example.json if absent.
#   4. Ignore the mutable cursor in the target .gitignore.
#   5. Merge   AGENTS.md via the target's merge-agents.sh (agent-instructions).
#   6. Symlink requested adapters (CLAUDE.md, WARP.md) — opt-in, never clobber.
#   7. Print how to register the Claude Code plugin so the hooks go live.
#
# Flags:
#   --only spec|flow   install a single half (default: both).
#   --dry-run          print the action plan and write nothing.
#
# Re-runnable: refreshes the managed core (.agents/**, .claude/** adapter) from
# source each run, preserves the target's live flow cursor (state.json) and any
# content outside the managed AGENTS.md markers. Managed core files are owned by
# vibe — local edits to them are replaced on re-run.

set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

err() { echo "install: $1" >&2; }
note() { echo "install: $1"; }
# say — announce a mutating action. In dry-run every action is described but
# never performed, so the plan reads the same whether or not it is applied.
say() { if [[ "$DRY_RUN" -eq 1 ]]; then echo "install: [dry-run] would $1"; else echo "install: $1"; fi; }

DRY_RUN=0
WANT_SPEC=1
WANT_FLOW=1
TARGET=""
ADAPTERS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --adapters) ADAPTERS="${2:-}"; shift 2 ;;
    --adapters=*) ADAPTERS="${1#*=}"; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --only)
      case "${2:-}" in
        spec) WANT_FLOW=0 ;;
        flow) WANT_SPEC=0 ;;
        *) err "ERROR: --only takes 'spec' or 'flow' (got '${2:-}')"; exit 1 ;;
      esac
      shift 2 ;;
    --only=*)
      case "${1#*=}" in
        spec) WANT_FLOW=0 ;;
        flow) WANT_SPEC=0 ;;
        *) err "ERROR: --only takes 'spec' or 'flow' (got '${1#*=}')"; exit 1 ;;
      esac
      shift ;;
    -h|--help) sed -n '2,23p' "$0"; exit 0 ;;
    -*) err "ERROR: unknown option '$1'"; exit 1 ;;
    *) TARGET="$1"; shift ;;
  esac
done

if [[ -z "$TARGET" ]]; then
  err "ERROR: target repo root required. Usage: ./install.sh <target-repo-root> [--only spec|flow] [--dry-run] [--adapters claude,warp]"
  exit 1
fi

# Resolve TARGET to an absolute path. Create it only for a real install; a
# dry-run must not touch the filesystem, so normalize without mkdir.
if [[ "$DRY_RUN" -eq 0 ]]; then
  mkdir -p "$TARGET"
fi
if [[ -d "$TARGET" ]]; then
  TARGET="$(cd "$TARGET" && pwd)"
else
  case "$TARGET" in
    /*) ;;
    *)  TARGET="$PWD/$TARGET" ;;
  esac
fi

if [[ "$TARGET" == "$SRC" ]]; then
  err "ERROR: target is the vibe source repo itself; nothing to install."
  exit 1
fi

# 1. Core (platform-neutral). Copy the spec + vibe skill trees. The cursor is
# per-project runtime state: never inherit the source's, never clobber the
# target's — preserve any live target cursor across the copy. Halves are copied
# independently so --only can install one without the other.
if [[ "$WANT_SPEC" -eq 1 || "$WANT_FLOW" -eq 1 ]]; then
  say "copy core skills into $TARGET/.agents/skills/"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    mkdir -p "$TARGET/.agents/skills"
  fi
fi
# -L dereferences: source stores canonical dirs at spec/ + flow/ behind
# .agents/skills/* symlinks; targets always get real directories.
if [[ "$WANT_SPEC" -eq 1 ]]; then
  say "copy spec skill -> $TARGET/.agents/skills/spec"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    cp -RL "$SRC/.agents/skills/spec" "$TARGET/.agents/skills/"
  fi
fi
if [[ "$WANT_FLOW" -eq 1 ]]; then
  say "copy vibe flow skill -> $TARGET/.agents/skills/vibe"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    SAVED_CURSOR=""
    if [[ -f "$TARGET/.agents/skills/vibe/state.json" ]]; then
      SAVED_CURSOR="$(mktemp)"
      cp "$TARGET/.agents/skills/vibe/state.json" "$SAVED_CURSOR"
    fi
    cp -RL "$SRC/.agents/skills/vibe" "$TARGET/.agents/skills/"
    if [[ -n "$SAVED_CURSOR" ]]; then
      mv -f "$SAVED_CURSOR" "$TARGET/.agents/skills/vibe/state.json"
      note "preserved existing flow cursor across re-install"
    else
      rm -f "$TARGET/.agents/skills/vibe/state.json"
    fi
  fi
fi

# 2. Claude Code adapter (command + hooks + plugin manifest). Adapter belongs to
# the flow half — it wires the hooks that drive the flow.
if [[ "$WANT_FLOW" -eq 1 ]]; then
  say "copy Claude adapter (.claude/commands, .claude/hooks, .claude-plugin/)"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    mkdir -p "$TARGET/.claude"
    cp -R "$SRC/.claude/commands" "$TARGET/.claude/"
    cp -R "$SRC/.claude/hooks" "$TARGET/.claude/"
    mkdir -p "$TARGET/.claude-plugin"
    cp "$SRC/.claude-plugin/plugin.json" "$TARGET/.claude-plugin/"
    chmod +x "$TARGET"/.claude/hooks/*.sh \
             "$TARGET"/.agents/skills/spec/scripts/*.sh \
             "$TARGET"/.agents/skills/vibe/scripts/*.sh 2>/dev/null || true
  fi
fi

# 3. Seed the cursor if absent (flow half only).
if [[ "$WANT_FLOW" -eq 1 && ! -f "$TARGET/.agents/skills/vibe/state.json" ]]; then
  say "seed .agents/skills/vibe/state.json from template"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    cp "$TARGET/.agents/skills/vibe/state.example.json" "$TARGET/.agents/skills/vibe/state.json"
  fi
fi

# 4. Ignore the mutable cursor (flow half only).
if [[ "$WANT_FLOW" -eq 1 ]]; then
  GI="$TARGET/.gitignore"
  if ! { [[ -f "$GI" ]] && grep -qF ".agents/skills/vibe/state.json" "$GI"; }; then
    say "add .agents/skills/vibe/state.json to .gitignore"
    if [[ "$DRY_RUN" -eq 0 ]]; then
      printf '\n# vibe mutable flow cursor (runtime; version state-machine.json, not this)\n.agents/skills/vibe/state.json\n' >> "$GI"
    fi
  fi
fi

# 5. Merge AGENTS.md via the copied merge script (agent-instructions). The merge
# script and its instruction template live in the flow half, so the merge runs
# only when the flow half is installed (--only spec stays a pure spec install).
MERGE="$TARGET/.agents/skills/vibe/scripts/merge-agents.sh"
if [[ "$WANT_FLOW" -eq 1 ]]; then
  if [[ "$DRY_RUN" -eq 1 ]]; then
    say "merge AGENTS.md via merge-agents.sh (managed markers only)"
  elif [[ -f "$MERGE" ]]; then
    bash "$MERGE" "$TARGET"
  else
    err "WARN: merge-agents.sh not found; skipping AGENTS.md merge."
  fi
fi

# 6. Opt-in adapter symlinks.
if [[ -n "$ADAPTERS" ]]; then
  IFS=',' read -r -a chosen <<< "$ADAPTERS"
  for a in "${chosen[@]}"; do
    case "$a" in
      claude) say "symlink CLAUDE.md -> AGENTS.md"
              [[ "$DRY_RUN" -eq 1 ]] || bash "$MERGE" link "CLAUDE.md" "$TARGET" || err "WARN: CLAUDE.md not linked (real file?)." ;;
      warp)   say "symlink WARP.md -> AGENTS.md"
              [[ "$DRY_RUN" -eq 1 ]] || bash "$MERGE" link "WARP.md" "$TARGET" || err "WARN: WARP.md not linked (real file?)." ;;
      *)      err "WARN: unknown adapter '$a' (known: claude, warp)." ;;
    esac
  done
fi

# 7. Plugin registration guidance (the hooks go live once the plugin is loaded).
if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "install: [dry-run] plan complete — nothing was written."
  exit 0
fi
cat <<EOF
install: done.
install: to activate the Claude Code hooks + /flow command, register the plugin:
install:   - local dev:  add "$TARGET" as a plugin (it has .claude-plugin/plugin.json)
install:   - the inject/guard/gate hooks read .agents/skills/vibe via \${CLAUDE_PROJECT_DIR}.
install: the spec + vibe skills are installed as project files under .agents/skills/.
EOF
