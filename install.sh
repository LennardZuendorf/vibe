#!/usr/bin/env bash
# install.sh — install the vibe workflow harness into a target repo.
#
#   ./install.sh <target-repo-root> [--only spec|flow] [--dry-run] [--uninstall] [--yes] [--adapters claude,warp]
#
# What it does (OPEN-3 default = COPY the platform-neutral core; MERGE the
# instruction file with markers; never blind-overwrite user content):
#   1. Copy the core   .agents/skills/{spec,vibe} (skills + vibe flow engine).
#   2. Copy the Claude adapter   .claude/commands + .claude/hooks/*.sh scripts.
#   3. Wire the flow hooks into   .claude/settings.json (merge, never clobber).
#   4. Seed   .agents/skills/vibe/state.json from state.example.json if absent.
#   5. Ignore the mutable cursor in the target .gitignore.
#   6. Merge   AGENTS.md via the target's merge-agents.sh (agent-instructions).
#   7. Symlink requested adapters (CLAUDE.md, WARP.md) — opt-in, never clobber.
#   8. Report that hooks are live and /flow is a native project command.
#
# Flags:
#   --only spec|flow   install (or uninstall) a single half (default: both).
#   --dry-run          print the action plan and write nothing.
#   --uninstall        remove managed artifacts; preserve .spec/, user AGENTS.md
#                      prose, and the flow cursor (cursor removed only with --yes).
#   --yes, -y          assume yes for the cursor-removal confirm on --uninstall.
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

# remove_shipped SRC_DIR DST_DIR — delete from DST_DIR only the files that exist
# in SRC_DIR (the precise inverse of a copy), then prune emptied subdirs. Never
# touches co-located files the user added to a shared directory.
remove_shipped() {
  local src="$1" dst="$2" f rel
  [[ -d "$src" && -d "$dst" ]] || return 0
  while IFS= read -r f; do
    rel="${f#"$src"/}"
    rm -f "$dst/$rel"
  done < <(find "$src" -type f)
  find "$dst" -type d -empty -delete 2>/dev/null || true
}

DRY_RUN=0
WANT_SPEC=1
WANT_FLOW=1
UNINSTALL=0
ASSUME_YES=0
TARGET=""
ADAPTERS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --adapters)
      [[ $# -ge 2 ]] || { err "ERROR: --adapters needs a value (e.g. --adapters claude,warp)"; exit 1; }
      ADAPTERS="$2"; shift 2 ;;
    --adapters=*) ADAPTERS="${1#*=}"; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --uninstall) UNINSTALL=1; shift ;;
    --yes|-y) ASSUME_YES=1; shift ;;
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
    -h|--help) sed -n '2,27p' "$0"; exit 0 ;;
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

# ── uninstall ────────────────────────────────────────────────────────────────
# Remove managed artifacts while preserving user content: .spec/** is never
# touched, user AGENTS.md prose outside the markers is preserved (block removed
# via merge-agents.sh's marker-pairing guard), and the flow cursor survives
# unless --yes. Composes with --only (remove one half) and --dry-run (preview).
if [[ "$UNINSTALL" -eq 1 ]]; then
  note "uninstalling vibe from $TARGET"
  SRC_MERGE="$SRC/.agents/skills/vibe/scripts/merge-agents.sh"
  SRC_MERGE_SETTINGS="$SRC/.agents/skills/vibe/scripts/merge-settings.sh"

  if [[ "$WANT_SPEC" -eq 1 && -e "$TARGET/.agents/skills/spec" ]]; then
    say "remove .agents/skills/spec"
    [[ "$DRY_RUN" -eq 1 ]] || rm -rf "$TARGET/.agents/skills/spec"
  fi

  if [[ "$WANT_FLOW" -eq 1 ]]; then
    if [[ -e "$TARGET/.agents/skills/vibe" ]]; then
      if [[ -f "$TARGET/.agents/skills/vibe/state.json" && "$ASSUME_YES" -eq 0 ]]; then
        say "remove .agents/skills/vibe (preserving the flow cursor; re-run with --yes to remove it)"
        if [[ "$DRY_RUN" -eq 0 ]]; then
          KEPT_CURSOR="$(mktemp)"; cp "$TARGET/.agents/skills/vibe/state.json" "$KEPT_CURSOR"
          rm -rf "$TARGET/.agents/skills/vibe"
          mkdir -p "$TARGET/.agents/skills/vibe"
          mv -f "$KEPT_CURSOR" "$TARGET/.agents/skills/vibe/state.json"
        fi
      else
        say "remove .agents/skills/vibe"
        [[ "$DRY_RUN" -eq 1 ]] || rm -rf "$TARGET/.agents/skills/vibe"
      fi
    fi
    # --yes removes the cursor, so fully invert install: strip the cursor stanza
    # install appended to .gitignore, leaving any other ignore rules intact.
    if [[ "$ASSUME_YES" -eq 1 && -f "$TARGET/.gitignore" ]]; then
      say "strip the vibe cursor stanza from .gitignore"
      if [[ "$DRY_RUN" -eq 0 ]]; then
        GI="$TARGET/.gitignore"; GI_TMP="$(mktemp)"
        grep -vxF \
          -e '# vibe mutable flow cursor (runtime; version state-machine.json, not this)' \
          -e '.agents/skills/vibe/state.json' "$GI" 2>/dev/null \
          | awk 'NF{last=NR} {line[NR]=$0} END{for(i=1;i<=last;i++) print line[i]}' >"$GI_TMP" || true
        if [[ -s "$GI_TMP" ]]; then mv -f "$GI_TMP" "$GI"; else rm -f "$GI_TMP" "$GI"; fi
      fi
    fi
    say "unwire the flow hooks from .claude/settings.json (user settings preserved)"
    if [[ "$DRY_RUN" -eq 0 && -f "$SRC_MERGE_SETTINGS" ]]; then
      bash "$SRC_MERGE_SETTINGS" unmerge "$TARGET" \
        || err "WARN: settings.json not unwired; left untouched."
    fi
    say "remove the Claude adapter files vibe installed"
    if [[ "$DRY_RUN" -eq 0 ]]; then
      remove_shipped "$SRC/.claude/commands" "$TARGET/.claude/commands"
      remove_shipped "$SRC/.claude/hooks" "$TARGET/.claude/hooks"
      # Retire artifacts a prior (plugin-based) install left behind.
      rm -f "$TARGET/.claude-plugin/plugin.json" "$TARGET/.claude/hooks/hooks.json"
      find "$TARGET/.claude/hooks" -type d -empty -delete 2>/dev/null || true
      rmdir "$TARGET/.claude-plugin" "$TARGET/.claude" 2>/dev/null || true
    fi
    if [[ -f "$TARGET/AGENTS.md" ]]; then
      say "remove the managed vibe:instructions block from AGENTS.md (user prose preserved)"
      [[ "$DRY_RUN" -eq 1 ]] || bash "$SRC_MERGE" unmerge "$TARGET" \
        || err "WARN: AGENTS.md block not removed (reversed markers?); left untouched."
    fi
  fi

  if [[ "$DRY_RUN" -eq 0 ]]; then
    rmdir "$TARGET/.agents/skills" "$TARGET/.agents" 2>/dev/null || true
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "install: [dry-run] uninstall plan complete — nothing was written."
  else
    note "uninstall done. Preserved: .spec/ and user AGENTS.md prose."
  fi
  exit 0
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
    # Source-only artifacts (co-located tests, contributor AGENTS.md) never ship.
    rm -rf "$TARGET/.agents/skills/spec/tests" "$TARGET/.agents/skills/spec/AGENTS.md"
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
    # Source-only artifacts (co-located tests, contributor AGENTS.md) never ship.
    rm -rf "$TARGET/.agents/skills/vibe/tests" "$TARGET/.agents/skills/vibe/AGENTS.md"
    if [[ -n "$SAVED_CURSOR" ]]; then
      mv -f "$SAVED_CURSOR" "$TARGET/.agents/skills/vibe/state.json"
      note "preserved existing flow cursor across re-install"
    else
      rm -f "$TARGET/.agents/skills/vibe/state.json"
    fi
  fi
fi

# 2. Claude Code adapter (native /flow command + hook scripts). Adapter belongs
# to the flow half — its hooks drive the flow. Hooks are wired via the target's
# .claude/settings.json (step 3), not a plugin (issue #12: one firing path).
if [[ "$WANT_FLOW" -eq 1 ]]; then
  say "copy Claude adapter (.claude/commands, .claude/hooks/*.sh)"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    mkdir -p "$TARGET/.claude"
    cp -R "$SRC/.claude/commands" "$TARGET/.claude/"
    cp -R "$SRC/.claude/hooks" "$TARGET/.claude/"
    # Retire a plugin manifest left by a prior install: settings.json now owns
    # hook wiring, and a live plugin beside it would double-fire the hooks.
    rm -f "$TARGET/.claude-plugin/plugin.json" "$TARGET/.claude/hooks/hooks.json"
    rmdir "$TARGET/.claude-plugin" 2>/dev/null || true
    chmod +x "$TARGET"/.claude/hooks/*.sh \
             "$TARGET"/.agents/skills/spec/scripts/*.sh \
             "$TARGET"/.agents/skills/vibe/scripts/*.sh 2>/dev/null || true
  fi
fi

# 3. Wire the flow hooks into .claude/settings.json — merge only vibe's three
# entries, idempotently, never clobbering user settings. Graceful-degrade if the
# helper is missing or jq is absent (it prints the snippet to paste). Flow half.
MERGE_SETTINGS="$SRC/.agents/skills/vibe/scripts/merge-settings.sh"
if [[ "$WANT_FLOW" -eq 1 ]]; then
  say "wire flow hooks into .claude/settings.json"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    if [[ -f "$MERGE_SETTINGS" ]]; then
      bash "$MERGE_SETTINGS" merge "$TARGET" || err "WARN: settings.json not wired (see message above)."
    else
      err "WARN: merge-settings.sh not found; hooks not wired into settings.json."
    fi
  fi
fi

# 4. Seed the cursor if absent (flow half only).
if [[ "$WANT_FLOW" -eq 1 && ! -f "$TARGET/.agents/skills/vibe/state.json" ]]; then
  say "seed .agents/skills/vibe/state.json from template"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    cp "$TARGET/.agents/skills/vibe/state.example.json" "$TARGET/.agents/skills/vibe/state.json"
  fi
fi

# 5. Ignore the mutable cursor (flow half only).
if [[ "$WANT_FLOW" -eq 1 ]]; then
  GI="$TARGET/.gitignore"
  if ! { [[ -f "$GI" ]] && grep -qF ".agents/skills/vibe/state.json" "$GI"; }; then
    say "add .agents/skills/vibe/state.json to .gitignore"
    if [[ "$DRY_RUN" -eq 0 ]]; then
      printf '\n# vibe mutable flow cursor (runtime; version state-machine.json, not this)\n.agents/skills/vibe/state.json\n' >> "$GI"
    fi
  fi
fi

# 6. Merge AGENTS.md via the copied merge script (agent-instructions). The merge
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

# 7. Opt-in adapter symlinks. These point at AGENTS.md, which only the flow half
# provisions — so they are meaningless (and merge-agents.sh is absent) under
# --only spec.
if [[ -n "$ADAPTERS" ]]; then
  if [[ "$WANT_FLOW" -eq 0 ]]; then
    err "WARN: --adapters is skipped under --only spec (adapter symlinks need the flow half's AGENTS.md)."
  else
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
fi

# 8. Completion guidance — tailored to what was actually installed.
if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "install: [dry-run] plan complete — nothing was written."
  exit 0
fi
if [[ "$WANT_FLOW" -eq 1 ]]; then
  cat <<EOF
install: done. The Claude Code hooks are wired automatically via
install:   .claude/settings.json — no plugin to register. Reload the project (or
install:   restart Claude Code) and the flow hooks fire on every turn; /flow works
install:   as a native project command from .claude/commands.
install:   Hook scripts self-resolve this project's flow state under
install:   \${CLAUDE_PROJECT_DIR}/.agents/skills/vibe.
install: the spec + vibe skills are installed as project files under .agents/skills/.
EOF
else
  cat <<EOF
install: done (spec framework only).
install: the spec skill is installed as a project file under .agents/skills/spec.
install: no flow harness or Claude adapter was installed (--only spec).
install: run without --only to add the flow harness + Claude Code adapter.
EOF
fi
