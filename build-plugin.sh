#!/usr/bin/env bash
# build-plugin.sh — assemble the vibe Claude Code plugin payload from the
# canonical spec/ + flow/ skill trees, so the marketplace ships a self-contained,
# per-user-installable plugin with a SINGLE source of truth (this script).
#
#   ./build-plugin.sh            # (re)build ./plugin and ./.claude-plugin/marketplace.json
#   ./build-plugin.sh --check    # verify the committed tree matches a fresh build
#                                #   (drift guard; exit 1 on mismatch). Writes nothing.
#
# Why a build step (not symlinks): the `claude plugin` skills scan mis-attributes
# symlinked skill dirs (a phantom extra skill); real copies discover cleanly. The
# duplication is the accepted cost and this script + the --check drift guard keep
# plugin/skills byte-identical to spec/ + flow/.
#
# What the plugin carries (the STATELESS, portable surface):
#   - skills/spec   the spec framework skill (operates on the project's ./.spec)
#   - skills/vibe   the vibe flow skill (working-model guidance)
#   - hooks/        a self-detecting SessionStart doctrine hook (read-only)
# The full STATEFUL flow (cursor writes, the write-guard, the Stop receipt tooth)
# is delivered per-repo by install.sh, not by the per-user plugin.

set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="0.2.0"

MODE="build"
case "${1:-}" in
  --check) MODE="check" ;;
  -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
  "") ;;
  *) echo "build-plugin: unknown option '$1'" >&2; exit 1 ;;
esac

# assemble DEST — write the full plugin payload (plugin/ + marketplace.json) rooted
# at DEST. Deterministic: same sources -> byte-identical tree.
assemble() {
  local dest="$1"
  local plug="$dest/plugin"
  rm -rf "$plug" "$dest/.claude-plugin"
  mkdir -p "$plug/.claude-plugin" "$plug/skills" "$plug/hooks" "$dest/.claude-plugin"

  # Skills: real copies of the canonical trees, minus the source-only artifacts
  # install.sh also strips (co-located tests, contributor AGENTS.md) and the
  # per-project runtime state a plugin must never ship (cursor, evidence receipts).
  cp -RL "$SRC/spec" "$plug/skills/spec"
  rm -rf "$plug/skills/spec/tests" "$plug/skills/spec/AGENTS.md"
  cp -RL "$SRC/flow" "$plug/skills/vibe"
  rm -rf "$plug/skills/vibe/tests" "$plug/skills/vibe/AGENTS.md" \
         "$plug/skills/vibe/evidence" "$plug/skills/vibe/state.json" \
         "$plug/skills/vibe/warnings.log"

  # Plugin manifest. NO "hooks" field — hooks/hooks.json at the plugin root is
  # auto-loaded; declaring it too double-loads and fails plugin load. NO "commands"
  # — /flow needs the per-repo cursor writer, which the plugin does not carry.
  cat > "$plug/.claude-plugin/plugin.json" <<JSON
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

  # Self-detecting SessionStart doctrine hook. Silent (exit 0) outside vibe repos so
  # the per-user plugin never adds noise elsewhere; inside one, it emits the
  # working-model doctrine + THIS project's cursor via the bundled doctrine.sh.
  cat > "$plug/hooks/hooks.json" <<'JSON'
{
  "hooks": {
    "SessionStart": [
      { "hooks": [ { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh\"" } ] }
    ]
  }
}
JSON

  cat > "$plug/hooks/session-start.sh" <<'SH'
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
  chmod +x "$plug/hooks/session-start.sh"
  find "$plug/skills" -name '*.sh' -exec chmod +x {} + 2>/dev/null || true

  # The repo IS the marketplace: `claude plugin marketplace add LennardZuendorf/vibe`
  # reads this, finds the plugin payload under ./plugin.
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
  assemble "$tmp"
  drift=0
  if ! diff -rq "$tmp/plugin" "$SRC/plugin" >/dev/null 2>&1; then drift=1; fi
  if ! diff -q "$tmp/.claude-plugin/marketplace.json" "$SRC/.claude-plugin/marketplace.json" >/dev/null 2>&1; then drift=1; fi
  if [[ "$drift" -eq 1 ]]; then
    echo "build-plugin: DRIFT — committed plugin/ is stale; run ./build-plugin.sh and commit." >&2
    diff -rq "$tmp/plugin" "$SRC/plugin" 2>&1 | sed 's/^/  /' >&2 || true
    exit 1
  fi
  echo "build-plugin: OK — committed plugin tree matches a fresh build."
  exit 0
fi

assemble "$SRC"
echo "build-plugin: built ./plugin (vibe $VERSION) and ./.claude-plugin/marketplace.json"
echo "build-plugin: validate with  claude plugin validate $SRC"
