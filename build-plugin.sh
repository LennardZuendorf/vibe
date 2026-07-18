#!/usr/bin/env bash
# build-plugin.sh — generate the vibe Claude Code plugin scaffold: the manifests,
# the self-detecting SessionStart doctrine hook, and SYMLINKED skill dirs pointing
# at the canonical spec/ + flow/ trees. Zero duplication, always in sync.
#
#   ./build-plugin.sh            # (re)generate ./plugin and ./.claude-plugin/marketplace.json
#   ./build-plugin.sh --check    # verify the committed scaffold matches a fresh build
#                                #   (exit 1 on mismatch). Writes nothing.
#
# Why symlinks (not copies): `claude plugin` mis-scans a plugin that also ships a
# commands/ dir (a phantom skill), but symlinked skill dirs discover cleanly, and
# `claude plugin install` DEREFERENCES them into the per-user cache as real dirs
# (verified against the live CLI: Skills (2) spec, vibe). So the repo commits two
# symlinks instead of ~6.7k duplicated lines, and there is nothing to drift.
# Note: `find` does not descend symlinks, so source-only artifacts (tests/,
# AGENTS.md) and gitignored runtime state (cursor, receipts) are never counted as
# committed payload; a clean marketplace fetch has no runtime state anyway.
#
# What the plugin carries (the STATELESS, portable surface):
#   - skills/spec -> ../../spec   the spec framework skill
#   - skills/vibe -> ../../flow   the vibe flow skill (working-model guidance)
#   - hooks/                       a self-detecting SessionStart doctrine hook (read-only)
# The full STATEFUL flow (cursor writes, the write-guard, the Stop receipt tooth)
# is delivered per-repo by install.sh, not by the per-user plugin.

set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="0.2.0"

MODE="build"
case "${1:-}" in
  --check) MODE="check" ;;
  -h|--help) sed -n '2,24p' "$0"; exit 0 ;;
  "") ;;
  *) echo "build-plugin: unknown option '$1'" >&2; exit 1 ;;
esac

# The real files a build generates (everything except the two skill symlinks).
GEN_FILES=(
  ".claude-plugin/marketplace.json"
  "plugin/.claude-plugin/plugin.json"
  "plugin/hooks/hooks.json"
  "plugin/hooks/session-start.sh"
)

# emit_files DEST — write the generated (non-symlink) plugin files rooted at DEST.
emit_files() {
  local dest="$1"
  mkdir -p "$dest/.claude-plugin" "$dest/plugin/.claude-plugin" "$dest/plugin/hooks"

  # Plugin manifest. NO "hooks" field — hooks/hooks.json at the plugin root is
  # auto-loaded; declaring it too double-loads and fails plugin load. NO "commands"
  # — /flow needs the per-repo cursor writer, which the plugin does not carry (its
  # presence would also trip the plugin skill scan).
  cat > "$dest/plugin/.claude-plugin/plugin.json" <<JSON
{
  "name": "vibe",
  "description": "Spec-first workflow harness: the spec + vibe skills and a self-detecting working-model doctrine hook.",
  "version": "$VERSION",
  "author": { "name": "Lennard Zuendorf", "url": "https://github.com/LennardZuendorf" },
  "homepage": "https://github.com/LennardZuendorf/vibe",
  "repository": "https://github.com/LennardZuendorf/vibe",
  "license": "MIT",
  "keywords": ["spec", "workflow", "vibe", "flow", "spec-driven"],
  "skills": "./skills/"
}
JSON

  cat > "$dest/plugin/hooks/hooks.json" <<'JSON'
{
  "hooks": {
    "SessionStart": [
      { "hooks": [ { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh\"" } ] }
    ]
  }
}
JSON

  cat > "$dest/plugin/hooks/session-start.sh" <<'SH'
#!/usr/bin/env bash
# vibe plugin SessionStart hook (per-user). Self-detects a vibe-enabled repo and,
# when found, emits the working-model doctrine + this project's cursor. Silent
# (exit 0) in non-vibe repos so a per-user install never adds noise elsewhere.
set -euo pipefail
cat >/dev/null 2>&1 || true   # consume stdin; unused

ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"
# vibe-enabled = the project carries a .spec/ tree or a flow cursor.
[[ -d "$ROOT/.spec" || -f "$ROOT/.agents/skills/vibe/state.json" ]] || exit 0

DOCTRINE="${CLAUDE_PLUGIN_ROOT:-}/skills/vibe/scripts/doctrine.sh"
[[ -f "$DOCTRINE" ]] || exit 0
# doctrine.sh reads the project cursor via CLAUDE_PROJECT_DIR, self-degrades, exit 0.
bash "$DOCTRINE" 2>/dev/null || true
exit 0
SH
  chmod +x "$dest/plugin/hooks/session-start.sh"

  # The repo IS the marketplace: `claude plugin marketplace add LennardZuendorf/vibe`
  # reads this, finds the plugin scaffold under ./plugin.
  cat > "$dest/.claude-plugin/marketplace.json" <<'JSON'
{
  "name": "vibe",
  "description": "vibe — spec-first workflow harness (spec + vibe skills, working-model doctrine hook).",
  "owner": { "name": "Lennard Zuendorf", "url": "https://github.com/LennardZuendorf" },
  "plugins": [
    { "name": "vibe", "source": "./plugin" }
  ]
}
JSON
}

if [[ "$MODE" == "check" ]]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  emit_files "$tmp"
  drift=0
  for f in "${GEN_FILES[@]}"; do
    if ! diff -q "$tmp/$f" "$SRC/$f" >/dev/null 2>&1; then
      drift=1; echo "build-plugin: stale $f" >&2
    fi
  done
  # The two skill symlinks must point at the canonical trees.
  [[ "$(readlink "$SRC/plugin/skills/spec" 2>/dev/null)" == "../../spec" ]] || { drift=1; echo "build-plugin: plugin/skills/spec is not a symlink -> ../../spec" >&2; }
  [[ "$(readlink "$SRC/plugin/skills/vibe" 2>/dev/null)" == "../../flow" ]] || { drift=1; echo "build-plugin: plugin/skills/vibe is not a symlink -> ../../flow" >&2; }
  if [[ "$drift" -eq 1 ]]; then
    echo "build-plugin: DRIFT — run ./build-plugin.sh and commit." >&2
    exit 1
  fi
  echo "build-plugin: OK — committed plugin scaffold matches a fresh build."
  exit 0
fi

rm -rf "$SRC/plugin" "$SRC/.claude-plugin"
emit_files "$SRC"
mkdir -p "$SRC/plugin/skills"
ln -s ../../spec "$SRC/plugin/skills/spec"
ln -s ../../flow "$SRC/plugin/skills/vibe"
echo "build-plugin: built ./plugin (vibe $VERSION, symlinked skills) and ./.claude-plugin/marketplace.json"
echo "build-plugin: validate with  claude plugin validate $SRC"
