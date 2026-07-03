#!/usr/bin/env bash
# tests/adapters/run.sh — behaviour tests for agent-instructions (merge-agents.sh,
# template, manifest) and platform-adapters (three hooks, hooks.json, plugin.json,
# install.sh). Pure bash; no bats. Each test cites its plan unit ID.
#
# Hook tests run against a throwaway install in a temp dir (its own state.json),
# so the source repo's cursor is never touched.

# The `cond && pass || fail` reporting idiom is intentional and safe here:
# pass()/fail() always return 0, so fail never runs spuriously after pass.
# shellcheck disable=SC2015
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MERGE="$REPO_ROOT/.agents/skills/vibe/scripts/merge-agents.sh"
TEMPLATE="$REPO_ROOT/.agents/skills/vibe/reference/templates/AGENTS.md"
ADAPTERS_JSON="$REPO_ROOT/.agents/skills/vibe/reference/adapters.json"
HOOKS="$REPO_ROOT/.claude/hooks"
HOOKS_JSON="$HOOKS/hooks.json"
PLUGIN="$REPO_ROOT/.claude-plugin/plugin.json"
INSTALL="$REPO_ROOT/install.sh"

PASS=0
FAIL=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
assert_contains()     { if [[ "$3" == *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want contains: $4"; echo "        got: $3"; fi; }
assert_not_contains() { if [[ "$3" != *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want NOT contains: $4"; fi; }
assert_eq()           { if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want: $4"; echo "        got: $3"; fi; }
mktmp() { mktemp -d "${TMPDIR:-/tmp}/vibe-adapt.XXXXXX"; }

echo "=== agent-instructions/2 — template + manifest ==="
grep -qF '<!-- vibe:instructions:start -->' "$TEMPLATE" && grep -qF '<!-- vibe:active-rules:start -->' "$TEMPLATE" \
  && pass "agent-instructions/2" "template has instructions + active-rules markers" \
  || fail "agent-instructions/2" "template markers"
out="$(jq -r '.adapters[].file' "$ADAPTERS_JSON" 2>/dev/null | tr '\n' ' ')"
assert_contains "agent-instructions/2" "adapters.json lists CLAUDE.md and WARP.md" "$out" "CLAUDE.md"
assert_contains "agent-instructions/2" "adapters.json lists WARP.md" "$out" "WARP.md"

echo ""
echo "=== agent-instructions/3 — merge-agents.sh ==="
# create
d="$(mktmp)"; bash "$MERGE" "$d" >/dev/null
grep -qF '<!-- vibe:instructions:start -->' "$d/AGENTS.md" && pass "agent-instructions/3" "missing -> create from template" || fail "agent-instructions/3" "create"
# preserve preamble + replace inner + idempotent
d="$(mktmp)"; { echo "## Our Team"; echo "keep me"; echo ""; cat "$TEMPLATE"; } > "$d/AGENTS.md"
bash "$MERGE" "$d" >/dev/null
grep -qF "keep me" "$d/AGENTS.md" && pass "agent-instructions/3" "user preamble preserved (R2)" || fail "agent-instructions/3" "preamble"
out="$(bash "$MERGE" "$d")"; assert_contains "agent-instructions/3" "re-run is a no-op" "$out" "no-op"
# constitution migration
d="$(mktmp)"; printf '# R\n\n<!-- vibe:constitution:start -->\nold\n<!-- vibe:constitution:end -->\n' > "$d/AGENTS.md"
bash "$MERGE" "$d" >/dev/null
grep -qF '<!-- vibe:instructions:start -->' "$d/AGENTS.md" && ! grep -qF 'vibe:constitution' "$d/AGENTS.md" \
  && pass "agent-instructions/3" "constitution -> instructions migration" || fail "agent-instructions/3" "migration"
# divergent -> append + warn, preserve content
d="$(mktmp)"; printf '# Different\n\nmine\n' > "$d/AGENTS.md"
out="$(bash "$MERGE" "$d" 2>&1)"
{ grep -qF "mine" "$d/AGENTS.md" && grep -qF '<!-- vibe:instructions:start -->' "$d/AGENTS.md"; } \
  && assert_contains "agent-instructions/3" "divergent appends + warns" "$out" "append" \
  || fail "agent-instructions/3" "divergent append"
# reversed markers must be refused, not silently mangled (content-safety invariant)
d="$(mktmp)"; printf '# R\n<!-- vibe:instructions:end -->\nmid\n<!-- vibe:instructions:start -->\ntail\n' > "$d/AGENTS.md"
rmbefore="$(cat "$d/AGENTS.md")"
if bash "$MERGE" "$d" >/dev/null 2>&1; then fail "agent-instructions/3" "reversed markers must be refused"; else pass "agent-instructions/3" "reversed markers refused (content safety)"; fi
assert_eq "agent-instructions/3" "reversed-marker file left untouched" "$(cat "$d/AGENTS.md")" "$rmbefore"
# link: skip correct, block real file
d="$(mktmp)"; bash "$MERGE" "$d" >/dev/null
bash "$MERGE" link CLAUDE.md "$d" >/dev/null
out="$(bash "$MERGE" link CLAUDE.md "$d")"; assert_contains "agent-instructions/5" "symlink idempotent skip" "$out" "skip"
[[ -L "$d/CLAUDE.md" && "$(readlink "$d/CLAUDE.md")" == "AGENTS.md" ]] && pass "agent-instructions/5" "relative symlink to AGENTS.md" || fail "agent-instructions/5" "symlink target"
printf 'real\n' > "$d/WARP.md"
if bash "$MERGE" link WARP.md "$d" >/dev/null 2>&1; then fail "agent-instructions/5" "real file must not be clobbered"; else pass "agent-instructions/5" "real file blocked (R5)"; fi

echo ""
echo "=== platform-adapters/4,5 — wiring + manifest validity ==="
jq -e . "$HOOKS_JSON" >/dev/null 2>&1 && pass "platform-adapters/4" "hooks.json is valid JSON" || fail "platform-adapters/4" "hooks.json JSON"
for ev in UserPromptSubmit PreToolUse Stop; do
  jq -e --arg e "$ev" '.hooks[$e]' "$HOOKS_JSON" >/dev/null 2>&1 && pass "platform-adapters/4" "hooks.json wires $ev" || fail "platform-adapters/4" "missing $ev"
done
matcher="$(jq -r '.hooks.PreToolUse[0].matcher' "$HOOKS_JSON")"
assert_eq "platform-adapters/4" "PreToolUse matcher is Edit|Write|NotebookEdit" "$matcher" "Edit|Write|NotebookEdit"
# every referenced hook script exists and is executable; paths are plugin-root-relative
allok=1
while IFS= read -r cmd; do
  rel="$(sed -E 's#.*\$\{CLAUDE_PLUGIN_ROOT\}/([^"]+)".*#\1#' <<<"$cmd")"
  [[ -x "$REPO_ROOT/$rel" ]] || { allok=0; echo "        not executable / missing: $rel"; }
  # shellcheck disable=SC2016  # intentional: match the literal ${CLAUDE_PLUGIN_ROOT} token
  [[ "$cmd" == *'${CLAUDE_PLUGIN_ROOT}'* ]] || { allok=0; echo "        not plugin-root-relative: $cmd"; }
done < <(jq -r '.hooks[][].hooks[].command' "$HOOKS_JSON")
assert_eq "platform-adapters/4" "all hook commands are plugin-root-relative + executable" "$allok" "1"
jq -e . "$PLUGIN" >/dev/null 2>&1 && pass "platform-adapters/5" "plugin.json is valid JSON" || fail "platform-adapters/5" "plugin.json JSON"
name="$(jq -r '.name' "$PLUGIN")"; assert_eq "platform-adapters/5" "plugin name is vibe" "$name" "vibe"
# component paths must be relative with ./ and no ../
bad=0
while IFS= read -r p; do
  [[ "$p" == ./* ]] || { bad=1; echo "        path missing ./: $p"; }
  [[ "$p" == *..* ]] && { bad=1; echo "        path uses ..: $p"; }
done < <(jq -r '[.commands, .hooks] | flatten | .[] | select(type=="string")' "$PLUGIN")
assert_eq "platform-adapters/5" "plugin component paths are ./-relative, no .." "$bad" "0"

echo ""
echo "=== platform-adapters/1,2,3 — hooks against a real install ==="
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
export CLAUDE_PROJECT_DIR="$SB"
SS="$SB/.agents/skills/vibe/scripts/set-state.sh"
# inject
out="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh"; echo "rc=$?")"
assert_contains "platform-adapters/1" "inject prints idle orders, exit 0" "$out" "state=idle"
assert_contains "platform-adapters/1" "inject exit 0" "$out" "rc=0"
# guard: block lessons.md outside compound
out="$(printf '{"tool_name":"Write","tool_input":{"file_path":".spec/lessons.md"}}' | bash "$SB/.claude/hooks/pre-tool-use-guard.sh" 2>&1; echo "rc=$?")"
assert_contains "platform-adapters/2" "guard blocks lessons.md (exit 2)" "$out" "rc=2"
assert_contains "platform-adapters/2" "guard gives a reason" "$out" "BLOCKED"
# guard: state.json direct edit always blocked
out="$(printf '{"tool_name":"Write","tool_input":{"file_path":".agents/skills/vibe/state.json"}}' | bash "$SB/.claude/hooks/pre-tool-use-guard.sh" 2>&1; echo "rc=$?")"
assert_contains "platform-adapters/2" "guard blocks direct state.json edit" "$out" "rc=2"
# guard: allow src in impl
bash "$SS" feature.impl demo >/dev/null
out="$(printf '{"tool_name":"Write","tool_input":{"file_path":"src/x.sh"}}' | bash "$SB/.claude/hooks/pre-tool-use-guard.sh" 2>&1; echo "rc=$?")"
assert_contains "platform-adapters/2" "guard allows src/ in feature.impl (exit 0)" "$out" "rc=0"
assert_not_contains "platform-adapters/2" "guard silent on allow" "$out" "BLOCKED"
# guard: warn src in idle
bash "$SS" idle >/dev/null
out="$(printf '{"tool_name":"Write","tool_input":{"file_path":"src/x.sh"}}' | bash "$SB/.claude/hooks/pre-tool-use-guard.sh" 2>&1; echo "rc=$?")"
assert_contains "platform-adapters/2" "guard warns src/ in idle but exits 0" "$out" "rc=0"
assert_contains "platform-adapters/2" "guard warn carries reason" "$out" "warn"
# guard: graceful on empty stdin
out="$(printf '' | bash "$SB/.claude/hooks/pre-tool-use-guard.sh" 2>&1; echo "rc=$?")"
assert_contains "platform-adapters/2" "guard exits 0 on empty stdin" "$out" "rc=0"
# gate: always exit 0
bash "$SS" feature.verify demo >/dev/null
out="$(printf '{}' | bash "$SB/.claude/hooks/stop-gate.sh" 2>&1; echo "rc=$?")"
assert_contains "platform-adapters/3" "gate always exits 0" "$out" "rc=0"
assert_contains "platform-adapters/3" "gate emits a warn-only smell" "$out" "vibe-gate"
unset CLAUDE_PROJECT_DIR
rm -rf "$SB"

echo ""
echo "=== platform-adapters/6 — installer ==="
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
ok=1
for p in .agents/skills/vibe/scripts/orders.sh .agents/skills/vibe/SKILL.md .claude/hooks/hooks.json .claude-plugin/plugin.json AGENTS.md .agents/skills/vibe/state.json; do
  [[ -e "$SB/$p" ]] || { ok=0; echo "        missing $p"; }
done
assert_eq "platform-adapters/6" "install lays down core + adapter + cursor" "$ok" "1"
grep -qF '.agents/skills/vibe/state.json' "$SB/.gitignore" && pass "platform-adapters/6" "install gitignores the cursor" || fail "platform-adapters/6" "gitignore"
before="$(cat "$SB/AGENTS.md")"; bash "$INSTALL" "$SB" >/dev/null 2>&1; after="$(cat "$SB/AGENTS.md")"
assert_eq "platform-adapters/6" "re-install is idempotent (AGENTS.md unchanged)" "$before" "$after"
# a live cursor must survive a re-install — idempotency may not clobber flow state
bash "$SB/.agents/skills/vibe/scripts/set-state.sh" feature.impl widget >/dev/null
before_cur="$(cat "$SB/.agents/skills/vibe/state.json")"; bash "$INSTALL" "$SB" >/dev/null 2>&1; after_cur="$(cat "$SB/.agents/skills/vibe/state.json")"
assert_eq "platform-adapters/6" "re-install preserves a live cursor (feature.impl widget)" "$before_cur" "$after_cur"
[[ ! -e "$SB/.agents/flow" ]] && pass "platform-adapters/6" ".agents/flow does not exist after install" || fail "platform-adapters/6" ".agents/flow must not exist after install"
[[ -d "$SB/.agents/skills/vibe" && ! -L "$SB/.agents/skills/vibe" ]] && pass "platform-adapters/6" "skills/vibe is a real directory (not a symlink)" || fail "platform-adapters/6" "skills/vibe is a real directory (not a symlink)"
[[ -d "$SB/.agents/skills/spec" && ! -L "$SB/.agents/skills/spec" ]] && pass "platform-adapters/6" "skills/spec is a real directory (not a symlink)" || fail "platform-adapters/6" "skills/spec is a real directory (not a symlink)"
symlinks="$(find "$SB/.agents" -type l)"
[[ -z "$symlinks" ]] && pass "platform-adapters/6" "no symlinks anywhere in installed .agents tree" || fail "platform-adapters/6" "no symlinks anywhere in installed .agents tree"
agents_entries="$(ls "$SB/.agents/")"
assert_eq "platform-adapters/6" ".agents/ contains only skills/" "$agents_entries" "skills"
if bash "$INSTALL" "$REPO_ROOT" >/dev/null 2>&1; then fail "platform-adapters/6" "must refuse self-install"; else pass "platform-adapters/6" "refuses self-install"; fi
rm -rf "$SB"

echo ""
echo "=== install-tooling/1 — action model + --dry-run ==="
# A content+path fingerprint of a tree: cksum of every file, path-sorted. Empty
# tree -> empty string. Detects any write (new/removed/changed file).
tree_fp() { find "$1" -type f 2>/dev/null -exec cksum {} + | LC_ALL=C sort; }
# --dry-run against a fresh (empty) target: non-empty plan, zero writes.
SB="$(mktmp)"
before="$(tree_fp "$SB")"
out="$(bash "$INSTALL" "$SB" --dry-run 2>&1; echo "rc=$?")"
after="$(tree_fp "$SB")"
assert_contains "install-tooling/1" "dry-run exits 0" "$out" "rc=0"
assert_contains "install-tooling/1" "dry-run prints an action plan" "$out" "[dry-run] would"
assert_eq "install-tooling/1" "dry-run on a fresh target writes nothing" "$before" "$after"
[[ ! -e "$SB/.agents" && ! -e "$SB/AGENTS.md" && ! -e "$SB/.gitignore" ]] \
  && pass "install-tooling/1" "dry-run creates no managed files" \
  || fail "install-tooling/1" "dry-run creates no managed files"
# --dry-run against an already-installed target is also byte-identical.
bash "$INSTALL" "$SB" >/dev/null 2>&1
before="$(tree_fp "$SB")"
bash "$INSTALL" "$SB" --dry-run >/dev/null 2>&1
after="$(tree_fp "$SB")"
assert_eq "install-tooling/1" "dry-run on an installed target is byte-identical" "$before" "$after"
# unknown option is rejected, not treated as the target.
out="$(bash "$INSTALL" "$SB" --bogus 2>&1; echo "rc=$?")"
assert_contains "install-tooling/1" "unknown option exits non-zero" "$out" "rc=1"
assert_contains "install-tooling/1" "unknown option names itself" "$out" "unknown option"
rm -rf "$SB"

echo ""
echo "=== install-tooling/2 — --only spec|flow ==="
# --only spec: spec skill only, no flow/adapter/plugin trace.
SB="$(mktmp)"; bash "$INSTALL" "$SB" --only spec >/dev/null 2>&1
ok=1
[[ -d "$SB/.agents/skills/spec" ]] || { ok=0; echo "        missing spec skill"; }
[[ ! -e "$SB/.agents/skills/vibe" ]] || { ok=0; echo "        vibe present under --only spec"; }
[[ ! -e "$SB/.claude/hooks" ]] || { ok=0; echo "        .claude/hooks present under --only spec"; }
[[ ! -e "$SB/.claude-plugin/plugin.json" ]] || { ok=0; echo "        plugin manifest present under --only spec"; }
assert_eq "install-tooling/2" "--only spec installs the spec half alone" "$ok" "1"
rm -rf "$SB"
# --only flow: flow + adapter present, no spec skill.
SB="$(mktmp)"; bash "$INSTALL" "$SB" --only flow >/dev/null 2>&1
ok=1
[[ -d "$SB/.agents/skills/vibe" ]] || { ok=0; echo "        missing vibe skill"; }
[[ -e "$SB/.claude/hooks/hooks.json" ]] || { ok=0; echo "        missing adapter hooks"; }
[[ -e "$SB/.claude-plugin/plugin.json" ]] || { ok=0; echo "        missing plugin manifest"; }
[[ ! -e "$SB/.agents/skills/spec" ]] || { ok=0; echo "        spec present under --only flow"; }
assert_eq "install-tooling/2" "--only flow installs flow + adapter alone" "$ok" "1"
rm -rf "$SB"
# --only bogus: usage error, exit 1.
SB="$(mktmp)"
out="$(bash "$INSTALL" "$SB" --only bogus 2>&1; echo "rc=$?")"
assert_contains "install-tooling/2" "--only bogus exits 1" "$out" "rc=1"
assert_contains "install-tooling/2" "--only bogus names the valid values" "$out" "spec"
rm -rf "$SB"
# --only composes with --dry-run (still writes nothing).
SB="$(mktmp)"; before="$(tree_fp "$SB")"
bash "$INSTALL" "$SB" --only spec --dry-run >/dev/null 2>&1
after="$(tree_fp "$SB")"
assert_eq "install-tooling/2" "--only spec --dry-run writes nothing" "$before" "$after"
rm -rf "$SB"

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
