#!/usr/bin/env bash
# flow/tests/adapters/run.sh — behaviour tests for agent-instructions (merge-agents.sh,
# template, manifest) and platform-adapters (three hooks, settings.json wiring,
# install.sh). Pure bash; no bats. Each test cites its plan unit ID.
#
# Hook tests run against a throwaway install in a temp dir (its own state.json),
# so the source repo's cursor is never touched.

# The `cond && pass || fail` reporting idiom is intentional and safe here:
# pass()/fail() always return 0, so fail never runs spuriously after pass.
# shellcheck disable=SC2015
set -uo pipefail

# Repo root by upward marker search (.spec / .git) — depth- and symlink-agnostic:
# resolves the physical path so real and symlinked invocations converge.
_find_repo_root() {
  local d; d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  while [[ "$d" != "/" ]]; do
    [[ -d "$d/.spec" || -e "$d/.git" ]] && { printf '%s\n' "$d"; return 0; }
    d="$(dirname "$d")"
  done
  return 1
}
REPO_ROOT="$(_find_repo_root)" || { echo "cannot locate repo root (.spec/.git)" >&2; exit 1; }
MERGE="$REPO_ROOT/.agents/skills/vibe/scripts/merge-agents.sh"
TEMPLATE="$REPO_ROOT/.agents/skills/vibe/reference/templates/AGENTS.md"
ADAPTERS_JSON="$REPO_ROOT/.agents/skills/vibe/reference/adapters.json"
INSTALL="$REPO_ROOT/install.sh"
# The vibe hook scripts are wired into a target's .claude/settings.json (issue
# #12: settings.json is the single firing path; no plugin manifest is shipped).
VIBE_HOOK_RE='\.claude/hooks/(session-start-doctrine|user-prompt-submit-inject|pre-tool-use-guard|stop-gate)\.sh'

PASS=0
FAIL=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
assert_contains()     { if [[ "$3" == *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want contains: $4"; echo "        got: $3"; fi; }
assert_not_contains() { if [[ "$3" != *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want NOT contains: $4"; fi; }
assert_eq()           { if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want: $4"; echo "        got: $3"; fi; }
mktmp() { mktemp -d "${TMPDIR:-/tmp}/vibe-adapt.XXXXXX"; }
# mkshim TOOL... — a PATH dir symlinking a broad toolset MINUS the named tools, to
# exercise graceful-degrade when an optional executor (jq / awk) is unavailable on
# a target. Prints the dir path.
mkshim() {
  local dir; dir="$(mktmp)"
  local excl=" $* " t p
  for t in bash sh mkdir dirname basename date mktemp mv cp rm rmdir sed grep head tail cat env awk find readlink ln chmod cmp diff sort cksum jq; do
    [[ "$excl" == *" $t "* ]] && continue
    p="$(command -v "$t" 2>/dev/null)" && ln -s "$p" "$dir/$t"
  done
  printf '%s\n' "$dir"
}

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
# awk-less unmerge (marker lookup is grep -n + sed, not awk): with awk absent the
# unmerge path still strips BOTH managed blocks. Discriminating — the old awk
# marker lookup exits 127 under set -e and leaves the block intact.
NOAWK="$(mkshim awk)"
d="$(mktmp)"; bash "$MERGE" "$d" >/dev/null
printf '\n## My Team\nkeep this prose\n' >> "$d/AGENTS.md"   # user prose => not a pure stub
PATH="$NOAWK" "$NOAWK/bash" "$MERGE" unmerge "$d" >/dev/null 2>&1
{ ! grep -qF 'vibe:instructions:start' "$d/AGENTS.md" && ! grep -qF 'vibe:active-rules:start' "$d/AGENTS.md" && grep -qF 'keep this prose' "$d/AGENTS.md"; } \
  && pass "agent-instructions/3" "awk-less unmerge strips both blocks, keeps user prose" \
  || fail "agent-instructions/3" "awk-less unmerge strips both blocks, keeps user prose"
# branded-title cleanup, scenario 1 (user prose outside markers): after stripping
# the blocks, the stranded '# AGENTS.md — vibe Engineering Guide' title vibe wrote
# is removed too, leaving only the user's prose.
grep -qF 'vibe Engineering Guide' "$d/AGENTS.md" \
  && fail "agent-instructions/3" "unmerge removes the stranded vibe-branded title (user-prose case)" \
  || pass "agent-instructions/3" "unmerge removes the stranded vibe-branded title (user-prose case)"
rm -rf "$NOAWK" "$d"
# branded-title cleanup, scenario 2 (active-rules regenerated post-install): the
# regenerated block makes the file diverge from the pristine stub, so the stub
# short-circuit does not fire; stripping the blocks strands the title. Removing it
# leaves only whitespace, so the vibe-created file is deleted (no orphan title).
d="$(mktmp)"; bash "$MERGE" "$d" >/dev/null
sed 's/_No lessons recorded yet\._/- do the thing/' "$d/AGENTS.md" > "$d/AGENTS.md.n" && mv "$d/AGENTS.md.n" "$d/AGENTS.md"
bash "$MERGE" unmerge "$d" >/dev/null 2>&1
[[ ! -e "$d/AGENTS.md" ]] \
  && pass "agent-instructions/3" "unmerge deletes the file when only the branded title would remain (regen case)" \
  || fail "agent-instructions/3" "unmerge deletes the file when only the branded title would remain (regen case)"
rm -rf "$d"
# branded-title cleanup, scenario 3 (title MID-FILE): a user who added prose ABOVE
# the vibe title still gets the orphaned title removed on unmerge — the strip is by
# FIRST exact match, wherever it sits, not line 1 only. Discriminating: the old
# head -n1 check sees the user's heading on line 1 and leaves a mid-file title.
d="$(mktmp)"; bash "$MERGE" "$d" >/dev/null
{ printf '## Top\nkeep top\n\n'; cat "$d/AGENTS.md"; printf '\n## Footer\nkeep bottom\n'; } > "$d/AGENTS.md.n" && mv "$d/AGENTS.md.n" "$d/AGENTS.md"
grep -qF '# AGENTS.md — vibe Engineering Guide' "$d/AGENTS.md" || fail "agent-instructions/3" "precondition: mid-file title present before unmerge"
bash "$MERGE" unmerge "$d" >/dev/null 2>&1
{ ! grep -qF 'vibe Engineering Guide' "$d/AGENTS.md" && grep -qF 'keep top' "$d/AGENTS.md" && grep -qF 'keep bottom' "$d/AGENTS.md"; } \
  && pass "agent-instructions/3" "unmerge strips a MID-FILE vibe title (prose above it), keeps user prose" \
  || fail "agent-instructions/3" "unmerge strips a MID-FILE vibe title (prose above it), keeps user prose"
rm -rf "$d"
# branded-title cleanup, scenario 4 (no title present): a file carrying managed
# blocks but NO vibe title line keeps its own line 1 — the title strip is a no-op.
# Discriminating: an unconditional 'sed 1d' would eat the user's first line.
d="$(mktmp)"
{ printf '## User Heading\nsome prose\n\n'; sed -n '/vibe:instructions:start/,/vibe:active-rules:end/p' "$TEMPLATE"; } > "$d/AGENTS.md"
bash "$MERGE" unmerge "$d" >/dev/null 2>&1
{ [[ "$(head -n1 "$d/AGENTS.md")" == "## User Heading" ]] && grep -qF 'some prose' "$d/AGENTS.md" \
  && ! grep -qF 'vibe:instructions:start' "$d/AGENTS.md"; } \
  && pass "agent-instructions/3" "unmerge leaves a no-title file's line 1 intact (title strip no-op)" \
  || fail "agent-instructions/3" "unmerge leaves a no-title file's line 1 intact (title strip no-op)"
rm -rf "$d"

echo ""
echo "=== agent-instructions — repo AGENTS.md block == shipped template (parity) ==="
# The repo's OWN vibe:instructions block must stay byte-identical to the template it
# ships, so the dogfood guide and every install target never drift. Proof: merging
# the template into a copy of the repo's AGENTS.md is a pure no-op (byte-identical).
REPO_AGENTS="$REPO_ROOT/AGENTS.md"
d="$(mktmp)"; cp "$REPO_AGENTS" "$d/AGENTS.md"
before="$(cksum < "$d/AGENTS.md")"
bash "$MERGE" "$d" >/dev/null 2>&1
after="$(cksum < "$d/AGENTS.md")"
assert_eq "agent-instructions/2" "repo AGENTS.md block is byte-identical to the shipped template (no drift)" "$after" "$before"
# Discriminating: drop a line from INSIDE the managed block; merge must repair it
# back to the pristine repo file — proving the parity assertion above would fail the
# moment the template gains (or loses) a line the repo block does not mirror.
d="$(mktmp)"; sed '/^## Degrade$/d' "$REPO_AGENTS" > "$d/AGENTS.md"
cmp -s "$d/AGENTS.md" "$REPO_AGENTS" && fail "agent-instructions/2" "precondition: corrupted block differs from pristine" || true
bash "$MERGE" "$d" >/dev/null 2>&1
cmp -s "$d/AGENTS.md" "$REPO_AGENTS" \
  && pass "agent-instructions/2" "merge repairs a drifted repo block back to the template (parity is enforced)" \
  || fail "agent-instructions/2" "merge repairs a drifted repo block back to the template (parity is enforced)"
rm -rf "$d"

echo ""
echo "=== platform-adapters/4,5 — settings.json hook wiring (auto-wired, no plugin) ==="
# A fresh install wires the three hooks into the target's .claude/settings.json —
# no plugin manifest, no hooks.json (issue #12: settings.json is the single path).
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
SETTINGS="$SB/.claude/settings.json"
jq -e . "$SETTINGS" >/dev/null 2>&1 && pass "platform-adapters/4" "install writes a valid settings.json" || fail "platform-adapters/4" "settings.json JSON"
# each event is wired to its script, via a $CLAUDE_PROJECT_DIR-relative command.
for pair in "SessionStart:session-start-doctrine.sh" "UserPromptSubmit:user-prompt-submit-inject.sh" "PreToolUse:pre-tool-use-guard.sh" "Stop:stop-gate.sh"; do
  ev="${pair%%:*}"; script="${pair#*:}"
  cmd="$(jq -r --arg e "$ev" '.hooks[$e][]?.hooks[]?.command // empty' "$SETTINGS" 2>/dev/null)"
  assert_contains "platform-adapters/4" "settings.json wires $ev -> $script" "$cmd" "$script"
  # shellcheck disable=SC2016  # intentional: match the literal $CLAUDE_PROJECT_DIR token
  assert_contains "platform-adapters/4" "$ev command is \$CLAUDE_PROJECT_DIR-relative" "$cmd" '$CLAUDE_PROJECT_DIR/.claude/hooks/'
done
matcher="$(jq -r '.hooks.PreToolUse[0].matcher' "$SETTINGS")"
assert_eq "platform-adapters/4" "PreToolUse matcher is Edit|Write|NotebookEdit|Bash" "$matcher" "Edit|Write|NotebookEdit|Bash"
# every wired command points at a hook script that exists and is executable.
allok=1
while IFS= read -r script; do
  [[ -z "$script" ]] && continue
  [[ -x "$SB/.claude/hooks/$script" ]] || { allok=0; echo "        not executable / missing: $script"; }
done < <(jq -r '.hooks[][]?.hooks[]?.command // empty' "$SETTINGS" | sed -E 's#.*/\.claude/hooks/([^"\\]+).*#\1#')
assert_eq "platform-adapters/4" "all wired hook scripts exist + executable in the install" "$allok" "1"
# auto-wired, not a plugin: neither legacy manifest is shipped.
[[ ! -e "$SB/.claude-plugin/plugin.json" ]] && pass "platform-adapters/5" "install ships no plugin.json (auto-wired via settings.json)" || fail "platform-adapters/5" "plugin.json must not be shipped"
[[ ! -e "$SB/.claude/hooks/hooks.json" ]] && pass "platform-adapters/5" "install ships no hooks.json manifest" || fail "platform-adapters/5" "hooks.json must not be shipped"
# idempotent: a second install leaves exactly one vibe group per event.
bash "$INSTALL" "$SB" >/dev/null 2>&1
dupok=1
for ev in SessionStart UserPromptSubmit PreToolUse Stop; do
  n="$(jq --arg e "$ev" --arg m "$VIBE_HOOK_RE" '[.hooks[$e][]? | select([.hooks[]?.command // empty] | any(test($m)))] | length' "$SETTINGS")"
  [[ "$n" == "1" ]] || { dupok=0; echo "        $ev has $n vibe groups (want 1)"; }
done
assert_eq "platform-adapters/4" "re-install does not duplicate vibe hook groups" "$dupok" "1"
rm -rf "$SB"
# --uninstall strips vibe's hook entries but preserves unrelated user settings.
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
SETTINGS="$SB/.claude/settings.json"
jq '. + {permissions:{allow:["Bash(git status:*)"]}}' "$SETTINGS" > "$SETTINGS.tmp" && mv "$SETTINGS.tmp" "$SETTINGS"
bash "$INSTALL" "$SB" --uninstall --yes >/dev/null 2>&1
if [[ -f "$SETTINGS" ]]; then
  wired="$(jq -r --arg m "$VIBE_HOOK_RE" '[.. | .command? // empty] | map(select(test($m))) | length' "$SETTINGS")"
  assert_eq "platform-adapters/5" "uninstall removes vibe hook entries from settings.json" "$wired" "0"
  kept="$(jq -r '.permissions.allow[0]' "$SETTINGS")"
  assert_eq "platform-adapters/5" "uninstall preserves unrelated user settings" "$kept" "Bash(git status:*)"
else
  fail "platform-adapters/5" "settings.json with a user key must survive uninstall"
fi
rm -rf "$SB"
# no-jq uninstall ordering: unwiring settings.json needs jq. If jq is absent the
# unwire cannot complete, so the hook scripts it references must be LEFT in place
# (deleting them would strand dead references). Discriminating — fails if uninstall
# deletes the hooks before/regardless of the failed unwire.
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
NOJQ="$(mkshim jq)"
PATH="$NOJQ" "$NOJQ/bash" "$INSTALL" "$SB" --uninstall --yes >/dev/null 2>&1
stillwired=0; grep -qF 'stop-gate.sh' "$SB/.claude/settings.json" 2>/dev/null && stillwired=1
hooksok=1
for h in session-start-doctrine user-prompt-submit-inject pre-tool-use-guard stop-gate; do
  [[ -f "$SB/.claude/hooks/$h.sh" ]] || { hooksok=0; echo "        deleted hook: $h.sh"; }
done
assert_eq "platform-adapters/5" "no-jq uninstall leaves settings.json wired" "$stillwired" "1"
assert_eq "platform-adapters/5" "no-jq uninstall keeps the four hook scripts (no dead refs)" "$hooksok" "1"
rm -rf "$SB" "$NOJQ"

echo ""
echo "=== flow-legibility/5 — SessionStart doctrine hook + wiring + doctor coverage ==="
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
# install wires SessionStart to the doctrine hook.
sscmd="$(jq -r '.hooks.SessionStart[]?.hooks[]?.command // empty' "$SB/.claude/settings.json" 2>/dev/null)"
assert_contains "flow-legibility/5" "install wires SessionStart -> session-start-doctrine.sh" "$sscmd" "session-start-doctrine.sh"
# the hook emits the doctrine (durable/ephemeral framing) + a cursor summary.
ssout="$(CLAUDE_PROJECT_DIR="$SB" bash "$SB/.claude/hooks/session-start-doctrine.sh" </dev/null 2>/dev/null)"
assert_contains "flow-legibility/5" "SessionStart hook emits the doctrine" "$ssout" "sessions are ephemeral"
assert_contains "flow-legibility/5" "SessionStart hook emits a cursor summary" "$ssout" "Cursor:"
# graceful degrade: resolver absent -> exit 0, no output.
rm -f "$SB/.agents/skills/vibe/scripts/doctrine.sh"
ss_rc=0; ssdeg="$(CLAUDE_PROJECT_DIR="$SB" bash "$SB/.claude/hooks/session-start-doctrine.sh" </dev/null 2>/dev/null)" || ss_rc=$?
assert_eq "flow-legibility/5" "SessionStart hook exits 0 when the resolver is absent" "$ss_rc" "0"
assert_eq "flow-legibility/5" "SessionStart hook emits nothing when the resolver is absent" "$ssdeg" ""
# doctor reports instruction coverage ok on a fresh install (block + hook wired).
docout="$(bash "$SB/.agents/skills/vibe/scripts/doctor.sh" "$SB" 2>&1)"
assert_contains "flow-legibility/5" "doctor reports instruction.coverage ok" "$docout" "ok   instruction.coverage"
rm -rf "$SB"

echo ""
echo "=== platform-adapters/1,2,3 — hooks against a real install ==="
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
export CLAUDE_PROJECT_DIR="$SB"
SS="$SB/.agents/skills/vibe/scripts/set-state.sh"
WLOG="$SB/.agents/skills/vibe/warnings.log"
# inject — behavioral: it emits the CURRENT cursor state's orders to stdout (the
# model-visible stream), not a fixed string. Asserting rc=0 from a hook that
# always exits 0 proves nothing, so pin the actual routed content instead.
bash "$SS" idle >/dev/null
out="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" 2>/dev/null)"
assert_contains "platform-adapters/1" "inject emits idle orders for an idle cursor" "$out" "state=idle"
bash "$SS" feature.impl demo >/dev/null
out="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" 2>/dev/null)"
assert_contains "platform-adapters/1" "inject emits the cursor state's orders (feature.impl)" "$out" "executing-plans"
assert_not_contains "platform-adapters/1" "inject does not emit a foreign state's orders" "$out" "state=idle"
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
# guard: warn src in idle -> exit 0, and the warn is routed to the MODEL-VISIBLE
# relay log (a warn on stderr with exit 0 is dropped by Claude Code). Assert on
# the log + rc, NOT on the guard's own 2>&1 (which would conflate the invisible
# stderr with the visible channel).
bash "$SS" idle >/dev/null
: > "$WLOG" 2>/dev/null || true
rc=0
printf '{"tool_name":"Write","tool_input":{"file_path":"src/x.sh"}}' \
  | bash "$SB/.claude/hooks/pre-tool-use-guard.sh" >/dev/null 2>&1 || rc=$?
assert_eq "platform-adapters/2" "guard warns src/ in idle but exits 0" "$rc" "0"
grep -qF "outside" "$WLOG" 2>/dev/null \
  && pass "platform-adapters/2" "guard queues the warn to the warnings relay log" \
  || fail "platform-adapters/2" "guard queues the warn to the warnings relay log"
# warnings relay (fix): inject drains the queued warns to STDOUT (the injected,
# model-visible stream) exactly once, prefixed vibe-warn:, then truncates the log
# so a warn never repeats. Assert on stdout, not stderr.
relay="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" 2>/dev/null)"
n="$(printf '%s\n' "$relay" | grep -c '^vibe-warn:' || true)"
assert_eq "platform-adapters/1" "inject relays the queued warn to stdout once" "$n" "1"
relay2="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" 2>/dev/null)"
n2="$(printf '%s\n' "$relay2" | grep -c '^vibe-warn:' || true)"
assert_eq "platform-adapters/1" "relay truncated after draining (warn not repeated)" "$n2" "0"
# guard: graceful on empty stdin
rc=0
printf '' | bash "$SB/.claude/hooks/pre-tool-use-guard.sh" >/dev/null 2>&1 || rc=$?
assert_eq "platform-adapters/2" "guard exits 0 on empty stdin" "$rc" "0"
# guard no-jq degrade: the three hard blocks still fire without jq (detect-context
# is pure bash; the path is extracted via sed). Assert exit 2 on a state.json edit.
NOJQ_BIN="$(mktmp)"
for _t in dirname date mktemp mv rm sed grep head cat bash env awk find; do
  _p="$(command -v "$_t" 2>/dev/null)" && ln -s "$_p" "$NOJQ_BIN/$_t"
done
rc=0
printf '{"tool_name":"Write","tool_input":{"file_path":".agents/skills/vibe/state.json"}}' \
  | PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SB/.claude/hooks/pre-tool-use-guard.sh" >/dev/null 2>&1 || rc=$?
assert_eq "platform-adapters/2" "guard blocks state.json without jq (sed path, exit 2)" "$rc" "2"
rc=0
printf '{"tool_name":"NotebookEdit","tool_input":{"notebook_path":"nb.ipynb"}}' \
  | PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SB/.claude/hooks/pre-tool-use-guard.sh" >/dev/null 2>&1 || rc=$?
assert_eq "platform-adapters/2" "guard allows a normal notebook_path without jq (exit 0)" "$rc" "0"
rm -rf "$NOJQ_BIN"
# gate: warn-only in a non-verify state -> exit 0, warn queued to the relay (not
# lost to stderr). No git changes in the temp install, so predicate 3 fires.
bash "$SS" feature.impl demo >/dev/null
: > "$WLOG" 2>/dev/null || true
rc=0
printf '{}' | bash "$SB/.claude/hooks/stop-gate.sh" >/dev/null 2>&1 || rc=$?
assert_eq "platform-adapters/3" "gate exits 0 in a non-verify state" "$rc" "0"
grep -q '^gate:' "$WLOG" 2>/dev/null \
  && pass "platform-adapters/3" "gate queues a warn-only smell to the relay log" \
  || fail "platform-adapters/3" "gate queues a warn-only smell to the relay log"
# flow-mvp/9 — the one promoted tooth: a *.verify state requires a fresh evidence
# receipt. Against a real install (not a git repo -> existence-only staleness).
bash "$SS" feature.verify demo >/dev/null
out="$(printf '{}' | bash "$SB/.claude/hooks/stop-gate.sh" 2>&1; echo "rc=$?")"
assert_contains "flow-mvp/9" "gate blocks feature.verify with no receipt (exit 2)" "$out" "rc=2"
assert_contains "flow-mvp/9" "block names the evidence receipt path" "$out" "evidence/feature-demo.md"
mkdir -p "$SB/.agents/skills/vibe/evidence"
printf 'commands + observed output per unit\n' > "$SB/.agents/skills/vibe/evidence/feature-demo.md"
out="$(printf '{}' | bash "$SB/.claude/hooks/stop-gate.sh" 2>&1; echo "rc=$?")"
assert_contains "flow-mvp/9" "gate passes feature.verify with a fresh receipt (exit 0)" "$out" "rc=0"
unset CLAUDE_PROJECT_DIR
rm -rf "$SB"

echo ""
echo "=== flow-mvp — set-state.sh jq-optional parity ==="
# jq is recommended, not required: without it set-state.sh still writes the cursor
# (via printf) and the output must be byte-identical to the jq path on the same
# transition. Run both against a fresh install with a jq-free PATH; normalize only
# the turn-varying `updated` timestamp before comparing.
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
SS="$SB/.agents/skills/vibe/scripts/set-state.sh"
CUR="$SB/.agents/skills/vibe/state.json"
NOJQ_BIN="$(mktmp)"
for _t in dirname date mktemp mv rm sed grep head cat bash env awk find; do
  _p="$(command -v "$_t" 2>/dev/null)" && ln -s "$_p" "$NOJQ_BIN/$_t"
done
norm_ts() { sed 's/"updated": "[^"]*"/"updated": "TS"/'; }
# string feature
bash "$SS" feature.design demo >/dev/null 2>&1; withjq="$(norm_ts < "$CUR")"
PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SS" feature.design demo >/dev/null 2>&1; nojq="$(norm_ts < "$CUR")"
assert_eq "flow-mvp" "cursor byte-identical with/without jq (string feature)" "$nojq" "$withjq"
# null feature (idle clears it)
bash "$SS" idle >/dev/null 2>&1; withjq="$(norm_ts < "$CUR")"
PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SS" idle >/dev/null 2>&1; nojq="$(norm_ts < "$CUR")"
assert_eq "flow-mvp" "cursor byte-identical with/without jq (null feature)" "$nojq" "$withjq"
# no-jq preserves the carried feature across a phase change
PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SS" feature.impl widget >/dev/null 2>&1
PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SS" feature.verify >/dev/null 2>&1
kept="$(sed -n 's/^[[:space:]]*"feature"[[:space:]]*:[[:space:]]*"\(.*\)".*/\1/p' "$CUR" | head -n1)"
assert_eq "flow-mvp" "no-jq set-state carries the feature across phases" "$kept" "widget"
# no-jq still rejects an unknown state
rc=0
PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SS" bogus.state >/dev/null 2>&1 || rc=$?
assert_eq "flow-mvp" "no-jq set-state rejects an unknown state" "$rc" "1"
# no-jq rejects a MACHINE META KEY masquerading as a state (discriminating: the
# old grep matched any '"key":' so top-level meta keys "style" / "version" passed
# validation as states). The tightened 4-space-indent + object-brace match rejects
# both; a real state (idle) still validates without jq.
rc=0
PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SS" style >/dev/null 2>&1 || rc=$?
assert_eq "review-fix" "no-jq set-state rejects the meta key 'style' (not a state)" "$rc" "1"
rc=0
PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SS" version >/dev/null 2>&1 || rc=$?
assert_eq "review-fix" "no-jq set-state rejects the meta key 'version' (not a state)" "$rc" "1"
rc=0
PATH="$NOJQ_BIN" "$NOJQ_BIN/bash" "$SS" idle >/dev/null 2>&1 || rc=$?
assert_eq "review-fix" "no-jq set-state still accepts a real state (idle)" "$rc" "0"
rm -rf "$NOJQ_BIN" "$SB"

echo ""
echo "=== platform-adapters/6 — installer ==="
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
ok=1
for p in .agents/skills/vibe/scripts/orders.sh .agents/skills/vibe/SKILL.md .claude/hooks/stop-gate.sh .claude/settings.json AGENTS.md .agents/skills/vibe/state.json; do
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
# Source-only artifacts must not ship — and the prune must not over-delete the
# skill payload. Both halves: co-located tests/ and the contributor AGENTS.md are
# stripped; SKILL.md (the real payload) still lands.
[[ ! -e "$SB/.agents/skills/spec/tests" && ! -e "$SB/.agents/skills/vibe/tests" ]] \
  && pass "platform-adapters/6" "install does not ship co-located tests/" \
  || fail "platform-adapters/6" "co-located tests/ must not ship into a target"
[[ ! -e "$SB/.agents/skills/spec/AGENTS.md" && ! -e "$SB/.agents/skills/vibe/AGENTS.md" ]] \
  && pass "platform-adapters/6" "install does not ship the contributor AGENTS.md" \
  || fail "platform-adapters/6" "per-half contributor AGENTS.md must not ship"
[[ -f "$SB/.agents/skills/spec/SKILL.md" && -f "$SB/.agents/skills/vibe/SKILL.md" ]] \
  && pass "platform-adapters/6" "prune keeps the skill payload (SKILL.md ships)" \
  || fail "platform-adapters/6" "prune over-deleted the skill payload"
symlinks="$(find "$SB/.agents" -type l)"
[[ -z "$symlinks" ]] && pass "platform-adapters/6" "no symlinks anywhere in installed .agents tree" || fail "platform-adapters/6" "no symlinks anywhere in installed .agents tree"
agents_entries="$(ls "$SB/.agents/")"
assert_eq "platform-adapters/6" ".agents/ contains only skills/" "$agents_entries" "skills"
if bash "$INSTALL" "$REPO_ROOT" >/dev/null 2>&1; then fail "platform-adapters/6" "must refuse self-install"; else pass "platform-adapters/6" "refuses self-install"; fi
rm -rf "$SB"
# EVIDENCE LEAK (discriminating): a source-side evidence receipt must NOT ship to
# the target — otherwise a dirty source's receipts silently satisfy the stop gate.
# Build a fake source tree, seed a receipt in it, install, assert it is absent.
FS="$(mktmp)"
cp "$INSTALL" "$FS/install.sh"
mkdir -p "$FS/.agents/skills" "$FS/.claude"
cp -RL "$REPO_ROOT/.agents/skills/spec" "$FS/.agents/skills/spec"
cp -RL "$REPO_ROOT/.agents/skills/vibe" "$FS/.agents/skills/vibe"
cp -RL "$REPO_ROOT/.claude/commands" "$FS/.claude/commands"
cp -RL "$REPO_ROOT/.claude/hooks" "$FS/.claude/hooks"
mkdir -p "$FS/.agents/skills/vibe/evidence"
printf 'LEAKED source receipt\n' > "$FS/.agents/skills/vibe/evidence/feature-demo.md"
SB="$(mktmp)"; bash "$FS/install.sh" "$SB" >/dev/null 2>&1
[[ ! -e "$SB/.agents/skills/vibe/evidence/feature-demo.md" ]] \
  && pass "platform-adapters/6" "source-side evidence receipt does not ship to the target" \
  || fail "platform-adapters/6" "source-side evidence receipt leaked into the target (gate bypass)"
# ...but the target's OWN receipts survive a re-install (runtime-state lesson).
mkdir -p "$SB/.agents/skills/vibe/evidence"
printf 'TARGET own receipt\n' > "$SB/.agents/skills/vibe/evidence/feature-widget.md"
bash "$FS/install.sh" "$SB" >/dev/null 2>&1
{ [[ -f "$SB/.agents/skills/vibe/evidence/feature-widget.md" ]] \
  && [[ ! -e "$SB/.agents/skills/vibe/evidence/feature-demo.md" ]]; } \
  && pass "platform-adapters/6" "target evidence survives re-install; source evidence still excluded" \
  || fail "platform-adapters/6" "re-install must preserve target evidence and exclude source evidence"
rm -rf "$SB" "$FS"
# gitignore nitpick: a freshly created .gitignore must NOT start with a blank line.
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
[[ -n "$(head -n1 "$SB/.gitignore")" ]] \
  && pass "platform-adapters/6" "fresh .gitignore has no leading blank line" \
  || fail "platform-adapters/6" "fresh .gitignore starts with a blank line"
grep -qF '.agents/skills/vibe/warnings.log' "$SB/.gitignore" \
  && pass "platform-adapters/6" "install gitignores the warnings relay log" \
  || fail "platform-adapters/6" "install gitignores the warnings relay log"
rm -rf "$SB"

echo ""
echo "=== platform-adapters/6 — skill registration under .claude/skills ==="
# Install registers /spec and the vibe skill where the docs say they live: relative
# symlinks under .claude/skills that resolve to the copied core (a real SKILL.md).
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
{ [[ -L "$SB/.claude/skills/spec" ]] && [[ "$(readlink "$SB/.claude/skills/spec")" == "../../.agents/skills/spec" ]] \
  && [[ -f "$SB/.claude/skills/spec/SKILL.md" ]]; } \
  && pass "platform-adapters/6" "install registers /spec at .claude/skills/spec (resolves)" \
  || fail "platform-adapters/6" "install registers /spec at .claude/skills/spec (resolves)"
{ [[ -L "$SB/.claude/skills/vibe" ]] && [[ "$(readlink "$SB/.claude/skills/vibe")" == "../../.agents/skills/vibe" ]] \
  && [[ -f "$SB/.claude/skills/vibe/SKILL.md" ]]; } \
  && pass "platform-adapters/6" "install registers the vibe skill at .claude/skills/vibe (resolves)" \
  || fail "platform-adapters/6" "install registers the vibe skill at .claude/skills/vibe (resolves)"
# DISCRIMINATING: a user's own skill dir in the SHARED .claude/skills survives
# uninstall, while EXACTLY the two vibe symlinks are removed (matched by target).
# Fails if uninstall blanket-removes .claude/skills or leaves the vibe links behind.
mkdir -p "$SB/.claude/skills/myskill"; printf 'mine\n' > "$SB/.claude/skills/myskill/SKILL.md"
bash "$INSTALL" "$SB" --uninstall --yes >/dev/null 2>&1
{ [[ -f "$SB/.claude/skills/myskill/SKILL.md" ]] \
  && [[ ! -L "$SB/.claude/skills/spec" && ! -e "$SB/.claude/skills/spec" ]] \
  && [[ ! -L "$SB/.claude/skills/vibe" && ! -e "$SB/.claude/skills/vibe" ]]; } \
  && pass "platform-adapters/6" "uninstall removes the vibe skill symlinks, keeps a user skill dir" \
  || fail "platform-adapters/6" "uninstall removes the vibe skill symlinks, keeps a user skill dir"
rm -rf "$SB"
# a user's REAL .claude/skills/spec directory must NOT be clobbered by registration.
SB="$(mktmp)"; mkdir -p "$SB/.claude/skills/spec"; printf 'USER real spec\n' > "$SB/.claude/skills/spec/SKILL.md"
bash "$INSTALL" "$SB" >/dev/null 2>&1
{ [[ ! -L "$SB/.claude/skills/spec" ]] && grep -qF 'USER real spec' "$SB/.claude/skills/spec/SKILL.md" \
  && [[ -L "$SB/.claude/skills/vibe" ]]; } \
  && pass "platform-adapters/6" "registration never clobbers a user's real .claude/skills/spec" \
  || fail "platform-adapters/6" "registration never clobbers a user's real .claude/skills/spec"
rm -rf "$SB"

echo ""
echo "=== install-tooling/1 — action model + --dry-run ==="
# A content+path fingerprint of a tree: cksum of every file, path-sorted. Empty
# tree -> empty string. Detects any write (new/removed/changed file).
tree_fp() { find "$1" -type f -exec cksum {} + 2>/dev/null | LC_ALL=C sort; }
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
[[ ! -e "$SB/.claude/settings.json" ]] || { ok=0; echo "        settings.json present under --only spec"; }
assert_eq "install-tooling/2" "--only spec installs the spec half alone" "$ok" "1"
rm -rf "$SB"
# --only flow: flow + adapter present, no spec skill.
SB="$(mktmp)"; bash "$INSTALL" "$SB" --only flow >/dev/null 2>&1
ok=1
[[ -d "$SB/.agents/skills/vibe" ]] || { ok=0; echo "        missing vibe skill"; }
[[ -e "$SB/.claude/hooks/stop-gate.sh" ]] || { ok=0; echo "        missing adapter hook scripts"; }
[[ -e "$SB/.claude/settings.json" ]] || { ok=0; echo "        missing settings.json wiring"; }
grep -qF 'stop-gate.sh' "$SB/.claude/settings.json" 2>/dev/null || { ok=0; echo "        settings.json not wired to hooks"; }
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
echo "=== install-tooling/3 — --uninstall ==="
# Install, add user content, then uninstall: managed files gone, user content kept.
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
mkdir -p "$SB/.spec"; printf 'user spec\n' > "$SB/.spec/product.md"
# user-authored files co-located in the SHARED adapter dirs must survive (the
# whole point of remove_shipped: surgical per-file removal, not rm -rf the dir).
printf 'my command\n' > "$SB/.claude/commands/mine.md"
printf 'my hook\n' > "$SB/.claude/hooks/custom.sh"
# ...and co-located in the SHARED SKILLS dirs too: uninstall must invert the copy
# per-file, never blanket rm -rf the skill dir (which would take these with it).
printf 'user note\n' > "$SB/.agents/skills/spec/mynote.md"
mkdir -p "$SB/.agents/skills/spec/sub"; printf 'nested\n' > "$SB/.agents/skills/spec/sub/deep.txt"
printf 'user flow note\n' > "$SB/.agents/skills/vibe/mynote.md"
printf '## My Team\nkeep this prose\n\n%s\n' "$(cat "$SB/AGENTS.md")" > "$SB/AGENTS.md.new" && mv "$SB/AGENTS.md.new" "$SB/AGENTS.md"
bash "$INSTALL" "$SB" --uninstall --yes >/dev/null 2>&1
ok=1
# Check the shipped payload is gone (the dirs themselves survive here because the
# test seeds user files into them — that survival is asserted separately below).
[[ ! -e "$SB/.agents/skills/spec/SKILL.md" ]] || { ok=0; echo "        spec skill survived uninstall"; }
[[ ! -e "$SB/.agents/skills/vibe/SKILL.md" ]] || { ok=0; echo "        vibe SKILL.md survived uninstall"; }
[[ ! -e "$SB/.claude/hooks/stop-gate.sh" ]] || { ok=0; echo "        adapter hook script survived uninstall"; }
assert_eq "install-tooling/3" "--uninstall removes managed artifacts" "$ok" "1"
# regression: co-located user files in shared adapter dirs are NOT deleted.
{ [[ -f "$SB/.claude/commands/mine.md" ]] && [[ -f "$SB/.claude/hooks/custom.sh" ]]; } \
  && pass "install-tooling/3" "co-located user files in shared adapter dirs survive uninstall" \
  || fail "install-tooling/3" "co-located user files in shared adapter dirs survive uninstall"
# DISCRIMINATING (per-file uninstall inverse): user files dropped into BOTH shared
# skills dirs survive, while the shipped payload is gone and emptied dirs pruned.
# Fails against the old blanket `rm -rf .agents/skills/{spec,vibe}` (which took the
# user files with it).
{ [[ -f "$SB/.agents/skills/spec/mynote.md" ]] && [[ -f "$SB/.agents/skills/spec/sub/deep.txt" ]] \
  && [[ -f "$SB/.agents/skills/vibe/mynote.md" ]]; } \
  && pass "install-tooling/3" "user files in shared skills dirs survive uninstall" \
  || fail "install-tooling/3" "user files in shared skills dirs survive uninstall"
{ [[ ! -e "$SB/.agents/skills/spec/SKILL.md" ]] && [[ ! -e "$SB/.agents/skills/vibe/SKILL.md" ]] \
  && [[ ! -e "$SB/.agents/skills/vibe/scripts" ]]; } \
  && pass "install-tooling/3" "shipped skill payload removed and emptied dirs pruned" \
  || fail "install-tooling/3" "shipped skill payload removed and emptied dirs pruned"
grep -qF "keep this prose" "$SB/AGENTS.md" && pass "install-tooling/3" "user AGENTS.md prose preserved" || fail "install-tooling/3" "user prose preserved"
grep -qF "vibe:instructions:start" "$SB/AGENTS.md" && fail "install-tooling/3" "managed AGENTS.md block removed" || pass "install-tooling/3" "managed AGENTS.md block removed"
# discriminating: the vibe:active-rules block must ALSO be stripped (unmerge used
# to leave it orphaned). Fails if only vibe:instructions is removed.
grep -qF "vibe:active-rules:start" "$SB/AGENTS.md" && fail "install-tooling/3" "managed active-rules block removed" || pass "install-tooling/3" "managed active-rules block removed"
[[ -f "$SB/.spec/product.md" ]] && pass "install-tooling/3" ".spec/ preserved across uninstall" || fail "install-tooling/3" ".spec/ preserved"
rm -rf "$SB"
# uninstall inverse (fix): adapter symlinks + vibe-created stub AGENTS.md.
# Fresh install with --adapters, nothing customized -> uninstall removes the
# CLAUDE.md/WARP.md symlinks AND the untouched-stub AGENTS.md (target had none).
SB="$(mktmp)"; bash "$INSTALL" "$SB" --adapters claude,warp >/dev/null 2>&1
[[ -L "$SB/CLAUDE.md" && -L "$SB/WARP.md" ]] || fail "install-tooling/3" "precondition: adapters symlinked"
bash "$INSTALL" "$SB" --uninstall --yes >/dev/null 2>&1
# -L not -e: the stub AGENTS.md is also deleted, so an un-removed symlink would
# merely DANGLE (-e false, -L true). Checking -L discriminates the real removal.
{ [[ ! -L "$SB/CLAUDE.md" && ! -e "$SB/CLAUDE.md" ]] && [[ ! -L "$SB/WARP.md" && ! -e "$SB/WARP.md" ]]; } \
  && pass "install-tooling/3" "uninstall removes adapter symlinks pointing at AGENTS.md" \
  || fail "install-tooling/3" "uninstall orphans adapter symlinks"
[[ ! -e "$SB/AGENTS.md" ]] \
  && pass "install-tooling/3" "uninstall deletes the vibe-created stub AGENTS.md (target had none)" \
  || fail "install-tooling/3" "uninstall orphans a vibe-created stub AGENTS.md"
rm -rf "$SB"
# discriminating: a user's REAL CLAUDE.md file (not a vibe symlink) must SURVIVE —
# uninstall may only remove a symlink that points at AGENTS.md.
SB="$(mktmp)"; printf 'my real claude guide\n' > "$SB/CLAUDE.md"
bash "$INSTALL" "$SB" >/dev/null 2>&1
bash "$INSTALL" "$SB" --uninstall --yes >/dev/null 2>&1
{ [[ -f "$SB/CLAUDE.md" && ! -L "$SB/CLAUDE.md" ]] && grep -qF "my real claude guide" "$SB/CLAUDE.md"; } \
  && pass "install-tooling/3" "a user's real CLAUDE.md file survives uninstall" \
  || fail "install-tooling/3" "uninstall clobbered a user's real CLAUDE.md file"
rm -rf "$SB"
# Live cursor + evidence receipt + no --yes -> both survive (flow-mvp verify fixes:
# discriminating test — fails if preservation is swapped for naive rm -rf).
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
bash "$SB/.agents/skills/vibe/scripts/set-state.sh" feature.impl widget >/dev/null 2>&1
mkdir -p "$SB/.agents/skills/vibe/evidence"
printf 'receipt: tests run\n' > "$SB/.agents/skills/vibe/evidence/feature-widget.md"
bash "$INSTALL" "$SB" --uninstall >/dev/null 2>&1
if [[ -f "$SB/.agents/skills/vibe/state.json" ]] && grep -qF widget "$SB/.agents/skills/vibe/state.json"; then
  pass "install-tooling/3" "live cursor survives uninstall without --yes"
else
  fail "install-tooling/3" "live cursor survives uninstall without --yes"
fi
if [[ -f "$SB/.agents/skills/vibe/evidence/feature-widget.md" ]]; then
  pass "flow-mvp/9" "evidence receipt survives uninstall without --yes"
else
  fail "flow-mvp/9" "evidence receipt survives uninstall without --yes"
fi
# ... and --yes removes both and fully inverts the gitignore stanzas.
bash "$INSTALL" "$SB" --uninstall --yes >/dev/null 2>&1
[[ ! -e "$SB/.agents/skills/vibe/state.json" ]] && pass "install-tooling/3" "--yes removes the cursor too" || fail "install-tooling/3" "--yes removes the cursor"
if [[ -f "$SB/.gitignore" ]] && grep -qF "evidence" "$SB/.gitignore"; then
  fail "flow-mvp/9" "--yes strips the evidence gitignore stanza"
else
  pass "flow-mvp/9" "--yes strips the evidence gitignore stanza"
fi
rm -rf "$SB"
# Reversed-marker AGENTS.md -> uninstall refuses to touch it (marker lesson regression).
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
printf '# R\n<!-- vibe:instructions:end -->\nmid\n<!-- vibe:instructions:start -->\ntail\n' > "$SB/AGENTS.md"
rmbefore="$(cat "$SB/AGENTS.md")"
bash "$INSTALL" "$SB" --uninstall --yes >/dev/null 2>&1
assert_eq "install-tooling/3" "reversed-marker AGENTS.md left byte-untouched by uninstall" "$(cat "$SB/AGENTS.md")" "$rmbefore"
rm -rf "$SB"
# --uninstall composes with --dry-run (writes nothing) and --only (one half).
SB="$(mktmp)"; bash "$INSTALL" "$SB" >/dev/null 2>&1
before="$(tree_fp "$SB")"
bash "$INSTALL" "$SB" --uninstall --dry-run >/dev/null 2>&1
after="$(tree_fp "$SB")"
assert_eq "install-tooling/3" "--uninstall --dry-run writes nothing" "$before" "$after"
bash "$INSTALL" "$SB" --uninstall --only spec --yes >/dev/null 2>&1
[[ ! -e "$SB/.agents/skills/spec" && -e "$SB/.agents/skills/vibe/SKILL.md" ]] \
  && pass "install-tooling/3" "--uninstall --only spec removes just the spec half" \
  || fail "install-tooling/3" "--uninstall --only spec removes just the spec half"
rm -rf "$SB"

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
