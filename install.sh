#!/usr/bin/env bash
# install.sh — install the vibe workflow harness. One command, two modes.
#
#   ./install.sh                 # single command: default to THIS repo; on a TTY,
#                                #   ask local vs global first.
#   ./install.sh <repo>          # local: full vibe into <repo> (explicit target).
#   ./install.sh --global        # per-user plugin (spec + vibe skills + doctrine,
#                                #   applies across every repo) via the claude CLI.
#   ./install.sh --with-plugins  # also offer companion plugins (superpowers, …).
#
# No checkout required — run straight from the web (installs into the current repo):
#   curl -fsSL https://raw.githubusercontent.com/LennardZuendorf/vibe/main/install.sh | bash
# Piped from curl (no skill bundle beside the script) it fetches a repo snapshot
# first (override with VIBE_REPO / VIBE_REF), then runs the local install.
#
# LOCAL install (copy the platform-neutral core; merge the instruction file with
# markers; never blind-overwrite user content):
#   1. Copy the core   .agents/skills/{spec,vibe} (skills + vibe flow engine).
#   2. Copy the Claude adapter   .claude/commands + .claude/hooks/*.sh scripts.
#   3. Wire the flow hooks into   .claude/settings.json (merge, never clobber).
#   4. Seed   .agents/skills/vibe/state.json from state.example.json if absent.
#   5. Ignore the mutable cursor in the target .gitignore.
#   6. Merge   AGENTS.md via the target's merge-agents.sh (agent-instructions).
#   7. Symlink requested adapters (CLAUDE.md, WARP.md) — opt-in, never clobber.
#   8. Report that hooks are live and /flow is a native project command.
#
# GLOBAL install installs the per-user vibe plugin (marketplace add + plugin
# install) so the skills + doctrine apply everywhere; the full STATEFUL flow
# (cursor, /flow, hooks) is a per-repo `--local` install.
#
# Flags:
#   --local            full per-repo install into the target (the default).
#   --global           per-user plugin install via the claude CLI.
#   --with-plugins     also install companion plugins (superpowers; feature-dev slot).
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

SRC="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd)" || SRC=""

# ── network bootstrap ────────────────────────────────────────────────────────
# Run straight from the web (curl … | bash) with no checkout: the skill bundle
# is not beside this script, so fetch a snapshot of the repo and re-run the real
# installer from the extracted tree (which then resolves like a normal checkout).
# Skipped whenever the bundle IS present, so a local ./install.sh never downloads.
VIBE_REPO="${VIBE_REPO:-LennardZuendorf/vibe}"
VIBE_REF="${VIBE_REF:-main}"
if [[ -z "${SRC:-}" || ! -f "$SRC/flow/state-machine.json" || ! -f "$SRC/spec/SKILL.md" ]]; then
  tarball="https://codeload.github.com/$VIBE_REPO/tar.gz/$VIBE_REF"
  boot_tmp="$(mktemp -d "${TMPDIR:-/tmp}/vibe-boot.XXXXXX")"
  trap 'rm -rf "$boot_tmp"' EXIT
  echo "install: fetching vibe ($VIBE_REPO@$VIBE_REF) …" >&2
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$tarball" | tar -xzf - -C "$boot_tmp"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$tarball" | tar -xzf - -C "$boot_tmp"
  else
    echo "install: ERROR: need curl or wget to install from the network." >&2
    exit 1
  fi
  boot_src="$(find "$boot_tmp" -mindepth 1 -maxdepth 1 -type d | head -n1)"
  if [[ -z "$boot_src" || ! -f "$boot_src/install.sh" ]]; then
    echo "install: ERROR: fetched archive is missing install.sh." >&2
    exit 1
  fi
  boot_rc=0
  bash "$boot_src/install.sh" "$@" || boot_rc=$?
  exit "$boot_rc"
fi

err() { echo "install: $1" >&2; }
note() { echo "install: $1"; }
# say — announce a mutating action. In dry-run every action is described but
# never performed, so the plan reads the same whether or not it is applied.
say() { if [[ "$DRY_RUN" -eq 1 ]]; then echo "install: [dry-run] would $1"; else echo "install: $1"; fi; }

# remove_shipped SRC_DIR DST_DIR [EXCLUDE_REL...] — delete from DST_DIR only the
# files that exist in SRC_DIR (the precise inverse of a copy), skipping any whose
# relative path equals or sits under an EXCLUDE_REL prefix (the artifacts install
# scrubs before shipping, or the per-project runtime state it must preserve), then
# prune emptied subdirs. Never touches co-located files the user added to a shared
# directory.
remove_shipped() {
  local src="$1" dst="$2"; shift 2
  local excludes=("$@") f rel ex skip
  [[ -d "$src" && -d "$dst" ]] || return 0
  while IFS= read -r f; do
    rel="${f#"$src"/}"
    skip=0
    for ex in "${excludes[@]}"; do
      if [[ "$rel" == "$ex" || "$rel" == "$ex"/* ]]; then skip=1; break; fi
    done
    [[ "$skip" -eq 1 ]] && continue
    rm -f "$dst/$rel"
  done < <(find -L "$src" -type f)
  find "$dst" -type d -empty -delete 2>/dev/null || true
}

# gi_append FILE LINE... — append the given lines to a .gitignore, separating them
# from prior content with exactly one blank line — but only when FILE already
# exists with content whose last line is non-blank. A freshly created (or
# blank-terminated) file gains no leading blank line.
gi_append() {
  local file="$1"; shift
  if [[ -s "$file" && -n "$(tail -n 1 "$file")" ]]; then
    printf '\n' >> "$file"
  fi
  printf '%s\n' "$@" >> "$file"
}

# register_skill NAME — link TARGET/.claude/skills/NAME -> ../../.agents/skills/NAME
# so the skill resolves where the docs say. Relative, so the tree stays portable.
# Idempotent (our own symlink is left as-is); never clobbers a user's real entry or
# a symlink they aimed elsewhere — that slot is theirs, warn once and skip.
register_skill() {
  local name="$1"
  local dir="$TARGET/.claude/skills"
  local link="$dir/$name"
  local rel="../../.agents/skills/$name"
  if [[ -L "$link" ]]; then
    [[ "$(readlink "$link")" == "$rel" ]] && return 0
    err "WARN: $link is a symlink to $(readlink "$link"), not $rel — leaving it untouched."
    return 0
  fi
  if [[ -e "$link" ]]; then
    err "WARN: $link already exists (not a vibe symlink) — leaving it untouched."
    return 0
  fi
  mkdir -p "$dir"
  ln -s "$rel" "$link"
}

# unregister_skill NAME — remove TARGET/.claude/skills/NAME only when it is exactly
# the symlink register_skill created (a symlink whose target is ../../.agents/skills/NAME).
# A user's real dir/file, or a symlink they aimed elsewhere, is never touched.
unregister_skill() {
  local name="$1"
  local link="$TARGET/.claude/skills/$name"
  local rel="../../.agents/skills/$name"
  if [[ -L "$link" && "$(readlink "$link")" == "$rel" ]]; then
    rm -f "$link"
  fi
}

# ── companion plugins (opt-in --with-plugins) ──────────────────────────────────
# superpowers is verified against the live marketplace. feature-dev has no stable
# public marketplace id, so it ships as a documented slot — fill it in when known.
# caveman is intentionally absent: it is an injected "caveman style" doctrine note,
# not a plugin. Each entry: "name@marketplace|marketplace-source".
VIBE_COMPANIONS=(
  "superpowers@superpowers-marketplace|obra/superpowers-marketplace"
  # "feature-dev@<marketplace>|<owner/repo>"   # add when its marketplace id is known
)

install_companion_plugins() {
  if ! command -v claude >/dev/null 2>&1; then
    err "WARN: --with-plugins needs the 'claude' CLI on PATH; skipping companion plugins."
    return 0
  fi
  local entry id src name
  for entry in "${VIBE_COMPANIONS[@]}"; do
    id="${entry%%|*}"; src="${entry#*|}"; name="${id%%@*}"
    if claude plugin list 2>/dev/null | grep -q "$name@"; then
      note "companion '$name' already installed — skipping"; continue
    fi
    say "add marketplace '$src' and install '$id' (user scope)"
    [[ "$DRY_RUN" -eq 1 ]] && continue
    if ! claude plugin marketplace add "$src" --scope user >/dev/null 2>&1; then
      err "WARN: could not add marketplace '$src' (offline?); skipping '$name'."; continue
    fi
    if claude plugin install "$id" --scope user >/dev/null 2>&1; then
      note "installed companion '$name'"
    else
      err "WARN: could not install '$id'; skipping."
    fi
  done
}

# ── global (per-user plugin) install ───────────────────────────────────────────
# Install the vibe plugin at user scope so its spec + vibe skills and the doctrine
# hook apply in every vibe-enabled repo. The full STATEFUL flow stays a per-repo
# --local install. Respects --dry-run and degrades without the claude CLI.
install_global() {
  local have_claude=0
  command -v claude >/dev/null 2>&1 && have_claude=1
  # A dry-run never touches anything, so it prints the plan even without the CLI. A
  # REAL global install needs the claude CLI to add the marketplace + install.
  if [[ "$DRY_RUN" -eq 0 && "$have_claude" -eq 0 ]]; then
    err "ERROR: --global needs the 'claude' CLI on PATH (it installs the vibe plugin per-user)."
    err "       Install Claude Code, or run a per-repo install: ./install.sh <repo> --local"
    exit 1
  fi
  say "add the vibe marketplace ($SRC) at user scope"
  say "install plugin vibe@vibe at user scope (applies across all your repos)"
  if [[ "$DRY_RUN" -eq 1 && "$have_claude" -eq 0 ]]; then
    note "(dry-run) note: the 'claude' CLI is not on PATH — a real --global run needs it."
  fi
  if [[ "$DRY_RUN" -eq 0 ]]; then
    claude plugin marketplace add "$SRC" --scope user >/dev/null 2>&1 \
      || err "WARN: could not add the vibe marketplace (already added?); continuing."
    if claude plugin install vibe@vibe --scope user >/dev/null 2>&1; then
      note "installed vibe@vibe (user scope)"
    else
      err "WARN: could not install vibe@vibe — run 'claude plugin install vibe@vibe' by hand."
    fi
  fi
  # Per-repo home for the spec framework: seed .spec/ in the current repo if it is a
  # git repo without one (the plugin carries the skills; each repo still needs its
  # own .spec/ memory). The full stateful flow is a separate --local install.
  if [[ -d "$TARGET/.git" && ! -d "$TARGET/.spec" ]]; then
    say "seed .spec/ in $TARGET (spec framework home for this repo)"
    if [[ "$DRY_RUN" -eq 0 && -f "$SRC/spec/scripts/setup.sh" ]]; then
      ( cd "$TARGET" && bash "$SRC/spec/scripts/setup.sh" ) >/dev/null 2>&1 \
        || err "WARN: .spec/ seed skipped (run 'bash spec/scripts/setup.sh' in the repo)."
    fi
  fi
  [[ "$WITH_PLUGINS" -eq 1 ]] && install_companion_plugins
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "install: [dry-run] global plan complete — nothing was written."
  else
    cat <<EOF
install: done (global / per-user plugin).
install:   vibe@vibe is installed at user scope — its spec + vibe skills and the
install:   SessionStart doctrine hook now apply in every vibe-enabled repo (one with
install:   a .spec/ tree). Restart Claude Code to load it.
install:   For the FULL stateful flow (cursor, /flow, hooks) in a repo, run:
install:     ./install.sh <repo> --local
EOF
  fi
  exit 0
}

# ── interactive mode prompt (bare TTY run only) ────────────────────────────────
prompt_mode() {
  local ans
  {
    echo "vibe install — choose a mode:"
    echo "  [1] local   full vibe into THIS repo ($TARGET): spec + flow + Claude adapter (default)"
    echo "  [2] global  per-user plugin (spec + vibe skills + doctrine, every repo)"
    printf 'Mode [1/2]? '
  } >&2
  read -r ans || ans=""
  case "$ans" in
    2|g|global) echo global ;;
    *)          echo local ;;
  esac
}

DRY_RUN=0
WANT_SPEC=1
WANT_FLOW=1
UNINSTALL=0
ASSUME_YES=0
MODE=""
WITH_PLUGINS=0
TARGET=""
TARGET_GIVEN=0
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
    --local)  MODE=local;  shift ;;
    --global) MODE=global; shift ;;
    --with-plugins) WITH_PLUGINS=1; shift ;;
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
    -h|--help) sed -n '2,/^set -euo pipefail/p' "$0" | sed '$d'; exit 0 ;;
    -*) err "ERROR: unknown option '$1'"; exit 1 ;;
    *) TARGET="$1"; TARGET_GIVEN=1; shift ;;
  esac
done

# Single-command resolution. A bare run (no target) defaults to THIS directory; an
# explicit target behaves exactly as before. Mode: --local/--global win; else a bare
# interactive run prompts (local vs global); else local. Explicit target or
# uninstall never prompts (backward-compatible with scripted/CI invocations).
if [[ "$TARGET_GIVEN" -eq 0 ]]; then
  # Bare run: install into the ENCLOSING repo, not just the cwd. Search upward for a
  # .spec/ or .git marker (same rule as the suites' _find_repo_root; `-e` catches a
  # .git file in worktrees/submodules), so `install.sh` from a subdirectory targets
  # the repo root. No marker at all (a brand-new project) falls back to the cwd.
  TARGET="$PWD"
  _d="$PWD"
  while [[ "$_d" != "/" ]]; do
    if [[ -d "$_d/.spec" || -e "$_d/.git" ]]; then TARGET="$_d"; break; fi
    _d="$(dirname "$_d")"
  done
fi
# An explicitly-passed empty/whitespace target (`install.sh ""`) must be rejected
# outright — never silently fall through to the current directory (that would run a
# --uninstall against cwd). Do not rely on a later `mkdir -p ""` failing.
if [[ -z "${TARGET//[[:space:]]/}" ]]; then
  err "ERROR: empty target. Pass a repo path, or run with NO argument to use the current directory."
  exit 1
fi
if [[ -z "$MODE" ]]; then
  if [[ "$TARGET_GIVEN" -eq 1 || "$UNINSTALL" -eq 1 ]]; then
    MODE=local
  elif [[ -t 0 && "$DRY_RUN" -eq 0 ]]; then
    MODE="$(prompt_mode)"
  else
    MODE=local
  fi
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

# A LOCAL install into the vibe source repo itself is meaningless (it already IS
# vibe). Global mode is fine from here — it installs the per-user plugin, not into
# this tree — so only guard the local path.
if [[ "$MODE" == "local" && "$UNINSTALL" -eq 0 && "$TARGET" == "$SRC" ]]; then
  err "ERROR: target is the vibe source repo itself; nothing to install."
  err "       Run from inside another repo, pass a target, or use --global."
  exit 1
fi

# Global (per-user plugin) install is a distinct, self-contained path.
if [[ "$MODE" == "global" && "$UNINSTALL" -eq 0 ]]; then
  install_global
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

  # Per-file inverse of the install copy (uninstall lesson): remove exactly the
  # relative paths the source bundle ships — minus the artifacts install scrubs
  # before shipping (tests/, contributor AGENTS.md) — and prune emptied dirs.
  # A user file dropped into either shared skills dir is never touched.
  if [[ "$WANT_SPEC" -eq 1 && -e "$TARGET/.agents/skills/spec" ]]; then
    say "remove the shipped files under .agents/skills/spec (user files preserved)"
    [[ "$DRY_RUN" -eq 1 ]] || remove_shipped \
      "$SRC/.agents/skills/spec" "$TARGET/.agents/skills/spec" tests AGENTS.md
  fi
  # Undo the spec skill registration — only our own .claude/skills/spec symlink.
  if [[ "$WANT_SPEC" -eq 1 ]]; then
    say "unregister .claude/skills/spec (only the vibe symlink; a user entry is kept)"
    [[ "$DRY_RUN" -eq 1 ]] || unregister_skill spec
  fi

  if [[ "$WANT_FLOW" -eq 1 ]]; then
    # Undo the vibe skill registration — only our own .claude/skills/vibe symlink.
    say "unregister .claude/skills/vibe (only the vibe symlink; a user entry is kept)"
    [[ "$DRY_RUN" -eq 1 ]] || unregister_skill vibe
    if [[ -e "$TARGET/.agents/skills/vibe" ]]; then
      # Exclude the same artifacts install scrubs (tests/, AGENTS.md, evidence/)
      # plus the per-project runtime state install never ships (state.json,
      # warnings.log): remove_shipped therefore leaves the cursor + receipts
      # intact by construction. --yes then removes those runtime files too.
      if [[ "$ASSUME_YES" -eq 0 ]]; then
        say "remove the shipped files under .agents/skills/vibe (preserving the flow cursor and evidence receipts; re-run with --yes to remove them)"
      else
        say "remove .agents/skills/vibe including the flow cursor and evidence receipts"
      fi
      if [[ "$DRY_RUN" -eq 0 ]]; then
        remove_shipped "$SRC/.agents/skills/vibe" "$TARGET/.agents/skills/vibe" \
          tests AGENTS.md evidence state.json warnings.log
        if [[ "$ASSUME_YES" -eq 1 ]]; then
          rm -f "$TARGET/.agents/skills/vibe/state.json" \
                "$TARGET/.agents/skills/vibe/warnings.log"
          rm -rf "$TARGET/.agents/skills/vibe/evidence"
          find "$TARGET/.agents/skills/vibe" -type d -empty -delete 2>/dev/null || true
        fi
      fi
    fi
    # --yes removes the cursor + receipts, so fully invert install: strip every
    # stanza install appended to .gitignore, leaving other ignore rules intact.
    if [[ "$ASSUME_YES" -eq 1 && -f "$TARGET/.gitignore" ]]; then
      say "strip the vibe cursor, evidence, and warnings stanzas from .gitignore"
      if [[ "$DRY_RUN" -eq 0 ]]; then
        GI="$TARGET/.gitignore"; GI_TMP="$(mktemp)"
        grep -vxF \
          -e '# vibe mutable flow cursor (runtime; version state-machine.json, not this)' \
          -e '.agents/skills/vibe/state.json' \
          -e '# vibe evidence receipts (runtime verification output, not memory)' \
          -e '.agents/skills/vibe/evidence/' \
          -e '# vibe warnings relay (runtime warn-first channel; surfaced then truncated)' \
          -e '.agents/skills/vibe/warnings.log' "$GI" 2>/dev/null \
          | awk 'NF{last=NR} {line[NR]=$0} END{for(i=1;i<=last;i++) print line[i]}' >"$GI_TMP" || true
        if [[ -s "$GI_TMP" ]]; then mv -f "$GI_TMP" "$GI"; else rm -f "$GI_TMP" "$GI"; fi
      fi
    fi
    # Unwire the settings.json entries FIRST — before deleting the hook scripts
    # they reference. If the unwire cannot complete (jq absent), the still-wired
    # settings.json would point at deleted scripts, so LEAVE the hook scripts in
    # place, warn once with the manual step, and continue the rest of uninstall.
    UNWIRE_OK=1
    say "unwire the flow hooks from .claude/settings.json (user settings preserved)"
    if [[ "$DRY_RUN" -eq 0 && -f "$SRC_MERGE_SETTINGS" ]]; then
      bash "$SRC_MERGE_SETTINGS" unmerge "$TARGET" || UNWIRE_OK=0
    fi
    say "remove the Claude adapter files vibe installed"
    if [[ "$DRY_RUN" -eq 0 ]]; then
      if [[ "$UNWIRE_OK" -eq 1 ]]; then
        remove_shipped "$SRC/.claude/commands" "$TARGET/.claude/commands"
        remove_shipped "$SRC/.claude/hooks" "$TARGET/.claude/hooks"
        # Retire artifacts a prior (plugin-based) install left behind.
        rm -f "$TARGET/.claude-plugin/plugin.json" "$TARGET/.claude/hooks/hooks.json"
        find "$TARGET/.claude/hooks" -type d -empty -delete 2>/dev/null || true
        rmdir "$TARGET/.claude/skills" "$TARGET/.claude-plugin" "$TARGET/.claude" 2>/dev/null || true
      else
        err "WARN: settings.json still wires the flow hooks (jq unavailable) — leaving .claude/hooks/*.sh and .claude/commands in place so the wiring keeps pointing at real files. Remove the vibe hook entries from .claude/settings.json by hand, then delete .claude/hooks and .claude/commands."
      fi
    fi
    # Remove the opt-in adapter symlinks vibe created — but ONLY when they are
    # symlinks that point at AGENTS.md. A user's real file of the same name (or a
    # symlink they aimed elsewhere) is left untouched.
    for adapter in CLAUDE.md WARP.md; do
      link="$TARGET/$adapter"
      if [[ -L "$link" && "$(readlink "$link")" == "AGENTS.md" ]]; then
        say "remove adapter symlink $adapter -> AGENTS.md"
        [[ "$DRY_RUN" -eq 1 ]] || rm -f "$link"
      fi
    done
    if [[ -f "$TARGET/AGENTS.md" ]]; then
      say "remove the managed vibe blocks from AGENTS.md (user prose preserved; a vibe-only stub is deleted)"
      [[ "$DRY_RUN" -eq 1 ]] || bash "$SRC_MERGE" unmerge "$TARGET" \
        || err "WARN: AGENTS.md blocks not removed (reversed markers?); left untouched."
    fi
  fi

  if [[ "$DRY_RUN" -eq 0 ]]; then
    rmdir "$TARGET/.claude/skills" "$TARGET/.claude" 2>/dev/null || true
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
    # Evidence receipts are per-project runtime state: preserve the target's
    # across a re-copy, but NEVER inherit the source's. A dirty source repo's
    # receipts would land in the target and silently satisfy stop-gate.sh's
    # receipt-existence check — a gate bypass.
    SAVED_EVID=""
    if [[ -d "$TARGET/.agents/skills/vibe/evidence" ]]; then
      SAVED_EVID="$(mktemp -d)"
      cp -R "$TARGET/.agents/skills/vibe/evidence/." "$SAVED_EVID/" 2>/dev/null || true
    fi
    cp -RL "$SRC/.agents/skills/vibe" "$TARGET/.agents/skills/"
    # Source-only artifacts (co-located tests, contributor AGENTS.md) and any
    # source-side evidence receipts never ship.
    rm -rf "$TARGET/.agents/skills/vibe/tests" \
           "$TARGET/.agents/skills/vibe/AGENTS.md" \
           "$TARGET/.agents/skills/vibe/evidence"
    if [[ -n "$SAVED_CURSOR" ]]; then
      mv -f "$SAVED_CURSOR" "$TARGET/.agents/skills/vibe/state.json"
      note "preserved existing flow cursor across re-install"
    else
      rm -f "$TARGET/.agents/skills/vibe/state.json"
    fi
    if [[ -n "$SAVED_EVID" ]]; then
      mkdir -p "$TARGET/.agents/skills/vibe/evidence"
      cp -R "$SAVED_EVID/." "$TARGET/.agents/skills/vibe/evidence/" 2>/dev/null || true
      rm -rf "$SAVED_EVID"
      note "preserved existing evidence receipts across re-install"
    fi
  fi
fi

# 1b. Register the copied skills under .claude/skills so /spec and the vibe skill
# resolve where the docs say. Each is a relative symlink into the core copied in
# step 1; a user entry already occupying the slot is preserved (register_skill
# warns + skips). Registration follows the half that was installed.
if [[ "$WANT_SPEC" -eq 1 ]]; then
  say "register spec skill at .claude/skills/spec -> ../../.agents/skills/spec"
  [[ "$DRY_RUN" -eq 1 ]] || register_skill spec
fi
if [[ "$WANT_FLOW" -eq 1 ]]; then
  say "register vibe skill at .claude/skills/vibe -> ../../.agents/skills/vibe"
  [[ "$DRY_RUN" -eq 1 ]] || register_skill vibe
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

# 5. Ignore the mutable runtime state (flow half only): the flow cursor, the
# evidence receipts, and the warnings relay log. Each line is guarded
# independently so re-runs never duplicate and an upgrade adds only the missing
# line. gi_append keeps a freshly created .gitignore free of a leading blank line.
if [[ "$WANT_FLOW" -eq 1 ]]; then
  GI="$TARGET/.gitignore"
  if ! { [[ -f "$GI" ]] && grep -qF ".agents/skills/vibe/state.json" "$GI"; }; then
    say "add .agents/skills/vibe/state.json to .gitignore"
    [[ "$DRY_RUN" -eq 1 ]] || gi_append "$GI" \
      "# vibe mutable flow cursor (runtime; version state-machine.json, not this)" \
      ".agents/skills/vibe/state.json"
  fi
  if ! { [[ -f "$GI" ]] && grep -qF ".agents/skills/vibe/evidence/" "$GI"; }; then
    say "add .agents/skills/vibe/evidence/ to .gitignore"
    [[ "$DRY_RUN" -eq 1 ]] || gi_append "$GI" \
      "# vibe evidence receipts (runtime verification output, not memory)" \
      ".agents/skills/vibe/evidence/"
  fi
  if ! { [[ -f "$GI" ]] && grep -qF ".agents/skills/vibe/warnings.log" "$GI"; }; then
    say "add .agents/skills/vibe/warnings.log to .gitignore"
    [[ "$DRY_RUN" -eq 1 ]] || gi_append "$GI" \
      "# vibe warnings relay (runtime warn-first channel; surfaced then truncated)" \
      ".agents/skills/vibe/warnings.log"
  fi
fi

# 6. Merge AGENTS.md via the copied merge script (agent-instructions). The merge
# script and its instruction template live in the flow half, so the merge runs
# only when the flow half is installed (--only spec stays a pure spec install).
MERGE="$TARGET/.agents/skills/vibe/scripts/merge-agents.sh"
if [[ "$WANT_FLOW" -eq 1 ]]; then
  if [[ "$DRY_RUN" -eq 1 ]]; then
    say "create or merge AGENTS.md via merge-agents.sh (managed markers only)"
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

# 7b. Companion plugins (opt-in --with-plugins). A user-level set applied across
# repos, installed via the claude CLI; graceful-degrade when it is absent.
if [[ "$WITH_PLUGINS" -eq 1 ]]; then
  install_companion_plugins
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
