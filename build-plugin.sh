#!/usr/bin/env bash
# build-plugin.sh — generate the vibe plugin scaffold: the host manifests, the
# self-detecting SessionStart doctrine hook, and the skill dirs each host can
# actually consume.
#
#   ./build-plugin.sh            # (re)generate ./plugin, ./.claude-plugin/marketplace.json,
#                                #   plugins/vibe/plugin.json, and the MATERIALIZED skill trees
#                                #   under plugins/vibe/skills and factory-plugin/skills
#   ./build-plugin.sh --check    # verify the committed scaffold matches a fresh build
#                                #   (exit 1 on mismatch). Writes nothing.
#
# Two delivery mechanisms, because the hosts differ and both were verified live:
#
#   - Claude (`./plugin`) keeps SYMLINKED skill dirs (skills/spec -> ../../spec,
#     skills/vibe -> ../../flow). `claude plugin install` DEREFERENCES them into
#     its per-user cache (verified against the live CLI: Skills (2) spec, vibe),
#     so the link costs nothing at runtime. A `commands/` dir is what mis-scans
#     as a phantom skill, so the plugin ships none.
#   - Codex (`plugins/vibe`) and Factory Droid (`factory-plugin`) ship REAL skill
#     dirs. `codex plugin add` drops a symlinked dir into its cache (leaving an
#     empty skills/) and `droid plugin install` does the same (vibe#71), so a
#     link is not a portable payload for them: the trees are materialized here
#     and pinned to spec/ + flow/ by --check.
#
# The materialized trees are the skill that SHIPS, not the contributor tree:
# `tests/` and the half's own AGENTS.md are pruned, exactly as install.sh prunes
# them ("source-only, never ships" — spec/AGENTS.md, flow/AGENTS.md). The file
# list comes from `git ls-files`, so gitignored runtime state (the flow cursor,
# evidence receipts, warnings.log) can never ride along, and `cp -p` keeps the
# shipped scripts executable.
#
# What the Claude plugin carries (the STATELESS, portable surface):
#   - skills/spec -> ../../spec   the spec framework skill
#   - skills/vibe -> ../../flow   the vibe flow skill (working-model guidance)
#   - skills/using-spec -> ../../spec/using-spec   the spec session-start skill
#   - agents -> ../spec/agents    the spec agents
#   - hooks/                       a self-detecting SessionStart doctrine hook (read-only)
#                                  plus the four spec hooks (session-start, guard, validate x2)
# The full STATEFUL flow (cursor writes, the write-guard, the Stop receipt tooth)
# is delivered per-repo by install.sh, not by the per-user plugin.

set -euo pipefail

SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Single source: package.json. A hardcoded literal drifted to 0.2.0 while the
# package said 0.3.0-dev, and --check pinned the stale value so CI enforced the
# drift. jq when present, a portable sed otherwise.
read_version() {
  local pj="$SRC/package.json"
  [[ -f "$pj" ]] || { echo "0.0.0"; return; }
  if command -v jq >/dev/null 2>&1; then
    jq -r '.version // "0.0.0"' "$pj"
  else
    sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$pj" | head -n1
  fi
}
# `|| true`: under `set -e` a command substitution's non-zero status aborts the
# script, so a malformed package.json killed the build with jq's parse error and
# the 0.0.0 fallback below was unreachable. Scripts here degrade with a warning,
# never hard-fail.
VERSION="$(read_version || true)"
if [[ -z "$VERSION" ]]; then
  echo "build-plugin: WARN — could not read .version from package.json; using 0.0.0" >&2
  VERSION="0.0.0"
fi

MODE="build"
case "${1:-}" in
  --check) MODE="check" ;;
  -h|--help)
    # Print the header comment block only (every leading line starting with #),
    # so the help text cannot drift out of the line range it used to name.
    awk 'NR > 1 && !/^#/ { exit } NR > 1 { sub(/^# ?/, ""); print }' "$0"
    exit 0 ;;
  "") ;;
  *) echo "build-plugin: unknown option '$1'" >&2; exit 1 ;;
esac

# ── materialized skill trees (Codex + Factory Droid) ─────────────────────────
# The two host packages that cannot consume a symlinked skill dir ship copies.
# SKILL_PKGS lists the package roots; SKILL_TREES maps <skill dir>:<source tree>.
SKILL_PKGS=("plugins/vibe" "factory-plugin")
SKILL_TREES=("spec:spec" "vibe:flow")

# copy_skill_tree SRC_REL DEST — materialize the tracked, shipped files of
# SRC_REL under DEST. `git ls-files` is the manifest (tracked only, so runtime
# state cannot leak in) and `cp -p` preserves the executable bit. Outside a git
# checkout (a tarball export) it warns and copies the tree, still applying the
# source-only prune, rather than failing the build.
copy_skill_tree() {
  local rel="$1" dest="$2" files f base
  if ! files="$( cd "$SRC" && git ls-files -- "$rel" 2>/dev/null )"; then
    files=""
  fi
  if [[ -z "$files" ]]; then
    echo "build-plugin: WARN — no tracked files under $rel (not a git checkout?); copying the tree as-is" >&2
    mkdir -p "$dest"
    cp -R "$SRC/$rel/." "$dest/" 2>/dev/null || return 1
    find "$dest" -depth -type d -name tests -exec rm -rf {} + 2>/dev/null || true
    rm -f "$dest/AGENTS.md"
    return 0
  fi
  while IFS= read -r f; do
    base="${f#"$rel"/}"
    case "$base" in
      tests/*|*/tests/*|AGENTS.md) continue ;;   # source-only artifacts
    esac
    mkdir -p "$dest/$(dirname "$base")"
    cp -p "$SRC/$f" "$dest/$base" || return 1
  done <<< "$files"
}

# emit_skill_trees DEST — materialize every package's skills/ tree under DEST.
emit_skill_trees() {
  local dest="$1" pkg pair name src
  for pkg in ${SKILL_PKGS[@]+"${SKILL_PKGS[@]}"}; do
    for pair in ${SKILL_TREES[@]+"${SKILL_TREES[@]}"}; do
      name="${pair%%:*}"
      src="${pair#*:}"
      copy_skill_tree "$src" "$dest/$pkg/skills/$name"
    done
  done
}

# sync_tree FRESH DEST — make DEST equal FRESH: drop what FRESH lacks (files AND
# symlinks, which is what replaces the committed links on the first build),
# refresh every file keeping its mode, then prune the dirs that emptied.
# Per-file, never a blanket `rm -rf` of the enclosing package.
sync_tree() {
  local fresh="$1" dest="$2" existing p rel
  existing="$(find "$dest" \( -type f -o -type l \) 2>/dev/null || true)"
  while IFS= read -r p; do
    [[ -n "$p" ]] || continue
    rel="${p#"$dest"/}"
    [[ -f "$fresh/$rel" ]] || rm -f "$p"
  done <<< "$existing"
  existing="$(find "$fresh" -type f 2>/dev/null || true)"
  while IFS= read -r p; do
    [[ -n "$p" ]] || continue
    rel="${p#"$fresh"/}"
    mkdir -p "$dest/$(dirname "$rel")"
    cp -p "$p" "$dest/$rel" || return 1
  done <<< "$existing"
  find "$dest" -mindepth 1 -type d -empty -delete 2>/dev/null || true
}

# The real files a build generates (everything except the symlinks).
GEN_FILES=(
  ".claude-plugin/marketplace.json"
  "plugin/.claude-plugin/plugin.json"
  "plugin/hooks/hooks.json"
  "plugin/hooks/session-start.sh"
  "plugins/vibe/plugin.json"
)

# emit_files DEST — write the generated (non-symlink) plugin files rooted at DEST.
emit_files() {
  local dest="$1"
  mkdir -p "$dest/.claude-plugin" "$dest/plugin/.claude-plugin" "$dest/plugin/hooks" "$dest/plugins/vibe"

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
      { "hooks": [ { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh\"", "timeout": 10 } ] },
      { "matcher": "startup|clear|compact", "hooks": [ { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/skills/spec/hooks/run-hook.sh\" session-start", "timeout": 10 } ] }
    ],
    "PreToolUse": [
      { "matcher": "Edit|Write|MultiEdit|NotebookEdit|Bash", "hooks": [ { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/skills/spec/hooks/run-hook.sh\" guard", "timeout": 10 } ] }
    ],
    "PostToolUse": [
      { "matcher": "Edit|Write|MultiEdit", "hooks": [ { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/skills/spec/hooks/run-hook.sh\" validate", "timeout": 10 } ] }
    ],
    "Stop": [
      { "hooks": [ { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/skills/spec/hooks/run-hook.sh\" validate", "timeout": 10 } ] }
    ]
  }
}
JSON

  cat > "$dest/plugin/hooks/session-start.sh" <<'SH'
#!/usr/bin/env bash
# vibe plugin SessionStart hook (per-user). Self-detects a repo with the flow
# installed and, when found, emits the working-model doctrine (no live state rides
# it). Silent (exit 0) in every other repo, a spec-only docs/spec repo included, so
# a per-user install never adds noise elsewhere; and silent in a repo whose
# settings wire its own session-start-doctrine hook and whose engine can run it,
# so the doctrine is emitted once, not twice or never.
set -euo pipefail
cat >/dev/null 2>&1 || true   # consume stdin; unused

ROOT="${CLAUDE_PROJECT_DIR:-}"
if [[ -z "$ROOT" ]]; then
  # No CLAUDE_PROJECT_DIR (a manual/non-Claude run): find the enclosing repo root by
  # upward marker search so detection works from a subdirectory; fall back to cwd.
  ROOT="$PWD"; d="$PWD"
  while [[ "$d" != "/" ]]; do
    if [[ -d "$d/docs/spec" || -d "$d/.spec" || -e "$d/.git" ]]; then ROOT="$d"; break; fi # literal
    d="$(dirname "$d")"
  done
fi
# A repo-local install (install.sh) already emits the doctrine via its own hook:
# defer to it only when settings.json/settings.local.json wire it AND it can
# answer. A copied but unwired script (no jq, invalid settings.json) never fires,
# and the wired one runs the Node engine with no bash fallback, so without node,
# without the engine, with a node too old for it or with a broken engine it
# prints nothing; none of those is a signal to stay silent. "Can answer" is
# probed, not inferred: the same engine command the repo hook runs must print.
ENGINE="${VIBE_ENGINE:-$ROOT/.agents/skills/vibe/engine}"
if command -v node >/dev/null 2>&1 && [[ -f "$ENGINE/cli.mjs" ]]; then
  for f in "$ROOT/.claude/settings.json" "$ROOT/.claude/settings.local.json"; do
    if [[ -f "$f" ]] && grep -qF 'session-start-doctrine.sh' "$f" 2>/dev/null; then
      probe="$(CLAUDE_PROJECT_DIR="$ROOT" node "$ENGINE/cli.mjs" hook session-start-doctrine </dev/null 2>/dev/null || true)"
      if [[ -n "$probe" ]]; then exit 0; fi
      break
    fi
  done
fi
# flow installed = the project carries the flow's state machine. A docs/spec tree
# alone is a spec repo, not a flow repo: the doctrine is flow doctrine.
[[ -f "$ROOT/.agents/skills/vibe/state-machine.json" ]] || exit 0

DOCTRINE="${CLAUDE_PLUGIN_ROOT:-}/skills/vibe/scripts/doctrine.sh"
[[ -f "$DOCTRINE" ]] || exit 0
# doctrine.sh reads no project state, self-degrades, exit 0.
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

  # The Codex package manifest. `version` is GENERATED from package.json for the
  # same reason as the Claude one: a hand-written second copy drifts, and a
  # manifest with no version makes Codex fall back to 1.0.0 (vibe#71) — the
  # per-user cache then names a version this repo never released. \$schema is
  # escaped: the heredoc is unquoted so $VERSION expands.
  cat > "$dest/plugins/vibe/plugin.json" <<JSON
{
  "\$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
  "name": "vibe",
  "description": "Spec-first workflow harness: the spec + vibe skills, packaged for Codex and Droid marketplace installs. Version comes from package.json — no second source of truth.",
  "version": "$VERSION"
}
JSON
}

if [[ "$MODE" == "check" ]]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  emit_files "$tmp"
  drift=0
  for f in ${GEN_FILES[@]+"${GEN_FILES[@]}"}; do
    if ! diff -q "$tmp/$f" "$SRC/$f" >/dev/null 2>&1; then
      drift=1; echo "build-plugin: stale $f" >&2
    fi
  done
  # The skill and agents symlinks must point at the canonical trees.
  [[ "$(readlink "$SRC/plugin/skills/spec" 2>/dev/null)" == "../../spec" ]] || { drift=1; echo "build-plugin: plugin/skills/spec is not a symlink -> ../../spec" >&2; }
  [[ "$(readlink "$SRC/plugin/skills/vibe" 2>/dev/null)" == "../../flow" ]] || { drift=1; echo "build-plugin: plugin/skills/vibe is not a symlink -> ../../flow" >&2; }
  [[ "$(readlink "$SRC/plugin/skills/using-spec" 2>/dev/null)" == "../../spec/using-spec" ]] || { drift=1; echo "build-plugin: plugin/skills/using-spec is not a symlink -> ../../spec/using-spec" >&2; }
  [[ "$(readlink "$SRC/plugin/agents" 2>/dev/null)" == "../spec/agents" ]] || { drift=1; echo "build-plugin: plugin/agents is not a symlink -> ../spec/agents" >&2; }

  # The Codex + Droid packages must carry a REAL tree equal to a fresh
  # materialization (content and executable bits), and no symlink at all: a
  # symlinked skill dir is exactly what both hosts drop from their cache.
  emit_skill_trees "$tmp"
  for pkg in ${SKILL_PKGS[@]+"${SKILL_PKGS[@]}"}; do
    for pair in ${SKILL_TREES[@]+"${SKILL_TREES[@]}"}; do
      name="${pair%%:*}"
      fresh="$tmp/$pkg/skills/$name"
      if ! diff -r "$fresh" "$SRC/$pkg/skills/$name" >/dev/null 2>&1; then
        drift=1
        echo "build-plugin: stale $pkg/skills/$name — committed tree differs from a fresh materialization" >&2
        continue
      fi
      while IFS= read -r f; do
        rel="${f#"$fresh"/}"
        committed="$SRC/$pkg/skills/$name/$rel"
        # diff -r compares content only; the shipped scripts must also still be
        # executable, so compare the bit against the fresh materialization.
        if [[ -x "$f" && ! -x "$committed" ]] || [[ ! -x "$f" && -x "$committed" ]]; then
          drift=1
          echo "build-plugin: $pkg/skills/$name/$rel has the wrong executable bit" >&2
        fi
      done < <(find "$fresh" -type f 2>/dev/null)
    done
  done
  linked="$(find "$SRC/plugins" "$SRC/factory-plugin" -type l 2>/dev/null || true)"
  if [[ -n "$linked" ]]; then
    drift=1
    while IFS= read -r l; do
      echo "build-plugin: symlink ${l#"$SRC"/} — codex and droid drop linked skills; materialize them" >&2
    done <<< "$linked"
  fi

  if [[ "$drift" -eq 1 ]]; then
    echo "build-plugin: DRIFT — run ./build-plugin.sh and commit." >&2
    exit 1
  fi
  echo "build-plugin: OK — committed plugin scaffold matches a fresh build."
  exit 0
fi

# Surgical clean, not `rm -rf plugin/ .claude-plugin/`: both are SHARED dirs a
# user or another tool may also write into, and this repo's own uninstall lesson
# forbids blanket-removing one. Delete exactly the generated set (already
# enumerated for --check) plus the symlinks, then prune what that
# emptied.
for f in ${GEN_FILES[@]+"${GEN_FILES[@]}"}; do
  rm -f "${SRC:?}/$f"
done
rm -f "$SRC/plugin/skills/spec" "$SRC/plugin/skills/vibe" "$SRC/plugin/skills/using-spec" "$SRC/plugin/agents"
find "$SRC/plugin" "$SRC/.claude-plugin" -type d -empty -delete 2>/dev/null || true

emit_files "$SRC"
mkdir -p "$SRC/plugin/skills"
ln -s ../../spec "$SRC/plugin/skills/spec"
ln -s ../../flow "$SRC/plugin/skills/vibe"
ln -s ../../spec/using-spec "$SRC/plugin/skills/using-spec"
ln -s ../spec/agents "$SRC/plugin/agents"

# Materialize the two host packages' skills. Built in a temp dir first, then
# synced in per file, so a skill deleted upstream disappears here too and the
# committed links of the old design are replaced by real trees.
fresh="$(mktemp -d)"
trap 'rm -rf "$fresh"' EXIT
emit_skill_trees "$fresh"
for pkg in ${SKILL_PKGS[@]+"${SKILL_PKGS[@]}"}; do
  for pair in ${SKILL_TREES[@]+"${SKILL_TREES[@]}"}; do
    name="${pair%%:*}"
    sync_tree "$fresh/$pkg/skills/$name" "$SRC/$pkg/skills/$name"
  done
done
echo "build-plugin: built ./plugin (vibe $VERSION, symlinked skills), ./.claude-plugin/marketplace.json, plugins/vibe/plugin.json"
echo "build-plugin: materialized skills in ${SKILL_PKGS[*]:-} (real dirs — codex and droid drop symlinks)"
echo "build-plugin: validate with  claude plugin validate $SRC"
