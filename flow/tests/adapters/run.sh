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
# macOS sets $TMPDIR WITH a trailing slash, so the naive template yields a
# literal `…/T//vibe-adapt.XXXXXX` that BSD mktemp hands back verbatim — while
# install.sh normalizes its target through `cd … && pwd`, which collapses the
# `//`. Same directory, different string, two spurious failures. Strip it once.
mktmp() { local t="${TMPDIR:-/tmp}"; mktemp -d "${t%/}/vibe-adapt.XXXXXX"; }
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
# mkhosts HOST... — a HERMETIC PATH farm for host-DETECTION legs: mkshim's broad
# toolset (which contains none of the four host CLIs, so a bare mkshim PATH is an
# inherent no-detect farm) plus executable stub CLIs for the named hosts. A
# detection leg must never read the ambient PATH — this sandbox has all four
# CLIs installed, CI need not, and a leg that passes only because the runner
# happens to have a CLI proves nothing about detection.
mkhosts() {
  local dir; dir="$(mkshim)"
  local h
  for h in "$@"; do
    printf '#!/bin/sh\nexit 0\n' > "$dir/$h"
    chmod +x "$dir/$h"
  done
  printf '%s\n' "$dir"
}
# The historical install invocation, pinned. Before host-support/2 the installer
# always laid down the Claude adapter; under detection-default a machine without
# a `claude` CLI installs host-neutral only, so every legacy leg states the
# explicit selection it always implicitly had. The migration only ADDS a flag —
# it weakens and deletes no assertion. Detection and explicit-selection legs
# (host-support/2) call $INSTALL directly, on a hermetic farm.

echo "=== agent-instructions/2 — template + manifest ==="
grep -qF '<!-- vibe:instructions:start -->' "$TEMPLATE" && grep -qF '<!-- vibe:active-rules:start -->' "$TEMPLATE" \
  && pass "agent-instructions/2" "template has instructions + active-rules markers" \
  || fail "agent-instructions/2" "template markers"
out="$(jq -r '.adapters[].file' "$ADAPTERS_JSON" 2>/dev/null | tr '\n' ' ')"
assert_contains "agent-instructions/2" "adapters.json lists CLAUDE.md and WARP.md" "$out" "CLAUDE.md"
assert_contains "agent-instructions/2" "adapters.json lists WARP.md" "$out" "WARP.md"

# The catalogue is DATA, and this is what makes that true rather than aspirational.
# adapters.json says of itself "Extend this list to add runtimes without editing
# skill prose" while install.sh hardcoded `case claude) … warp)` and uninstall
# hardcoded `for adapter in CLAUDE.md WARP.md`, so a third entry changed nothing.
# Discriminating by construction: this plants an adapter that appears in NO
# source file, so it can only work by being read from the manifest.
if command -v jq >/dev/null 2>&1; then
  SBA="$(mktmp)"; SRCA="$(mktmp)"
  # A source tree with one extra adapter. Copy (not symlink) so the real
  # manifest is never touched.
  tar -cf - --exclude=.git -C "$REPO_ROOT" . 2>/dev/null | tar -xf - -C "$SRCA"
  jq '.adapters += [{"file":"ZZTEST.md","target":"AGENTS.md","runtime":"zztest","default":false}]' \
     "$REPO_ROOT/flow/reference/adapters.json" > "$SRCA/flow/reference/adapters.json"
  bash "$SRCA/install.sh" "$SBA" --platform claude --adapters zztest >/dev/null 2>&1
  { [[ -L "$SBA/ZZTEST.md" && "$(readlink "$SBA/ZZTEST.md")" == "AGENTS.md" ]]; } \
    && pass "agent-instructions/2" "a manifest-only adapter is linked by install (catalogue is live data)" \
    || fail "agent-instructions/2" "a manifest-only adapter is linked by install (catalogue is live data)"
  bash "$SRCA/install.sh" "$SBA" --platform claude --uninstall --yes >/dev/null 2>&1
  [[ ! -e "$SBA/ZZTEST.md" ]] \
    && pass "agent-instructions/2" "uninstall removes a manifest-only adapter from the same data" \
    || fail "agent-instructions/2" "uninstall removes a manifest-only adapter from the same data"
  # An unknown name reports the manifest's own key list, not a hardcoded one.
  outa="$(bash "$SRCA/install.sh" "$SBA" --platform claude --adapters nope 2>&1 || true)"
  assert_contains "agent-instructions/2" "an unknown adapter lists the manifest's keys" "$outa" "zztest"
  rm -rf "$SBA" "$SRCA"
else
  pass "agent-instructions/2" "manifest-driven adapter checks skipped (no jq)"
fi

# …and without jq the catalogue degrades to the two shipped pairs rather than to
# nothing: --adapters claude must still link.
SBJ="$(mktmp)"; NOJQ_A="$(mkshim jq)"
PATH="$NOJQ_A" "$NOJQ_A/bash" "$INSTALL" "$SBJ" --platform claude --adapters claude >/dev/null 2>&1
{ [[ -L "$SBJ/CLAUDE.md" && "$(readlink "$SBJ/CLAUDE.md")" == "AGENTS.md" ]]; } \
  && pass "agent-instructions/2" "jq-less: --adapters claude still links via the fallback pairs" \
  || fail "agent-instructions/2" "jq-less: --adapters claude still links via the fallback pairs"
rm -rf "$SBJ" "$NOJQ_A"

echo ""
echo "=== agent-instructions/3 — merge-agents.sh ==="
# inject-triggers/6 fix round 2 — STDERR IS ASSERTED, not discarded.
#
# Of the 17 $MERGE invocations in this file, 8 discarded stderr and the one that
# captured it only asserted a warning was PRESENT. A `tmp: unbound variable`
# regression (a RETURN trap firing a second time with its locals out of scope)
# therefore printed on every single run while this suite reported 184 passed,
# 0 failed — the script "worked", noisily, and nothing could tell.
#
# merge_run runs merge-agents.sh with stdout and stderr captured SEPARATELY,
# leaves stdout in $MERGE_STDOUT for the caller, and asserts stderr is exactly
# empty. The divergent-append path is the one case that must warn; it keeps
# using a plain call with 2>&1 and asserts the warning's presence.
MERGE_STDOUT=""
merge_run() {
  local desc="$1"; shift
  local errf rc
  errf="$(mktmp)/stderr"
  MERGE_STDOUT="$(bash "$MERGE" "$@" 2>"$errf")"
  rc=$?
  local errtxt; errtxt="$(cat "$errf")"; rm -f "$errf"
  if [[ -z "$errtxt" ]]; then
    pass "inject-triggers/6" "stderr clean: $desc"
  else
    fail "inject-triggers/6" "stderr clean: $desc"
    echo "        stderr: $errtxt"
  fi
  return $rc
}
# create
d="$(mktmp)"; merge_run "merge case 1 (create from template)" "$d"
merge_run "merge case 1 re-run (create -> no-op)" "$d"
grep -qF '<!-- vibe:instructions:start -->' "$d/AGENTS.md" && pass "agent-instructions/3" "missing -> create from template" || fail "agent-instructions/3" "create"
# preserve preamble + replace inner + idempotent
d="$(mktmp)"; { echo "## Our Team"; echo "keep me"; echo ""; cat "$TEMPLATE"; } > "$d/AGENTS.md"
merge_run "merge case 2 (replace the instructions block)" "$d"
grep -qF "keep me" "$d/AGENTS.md" && pass "agent-instructions/3" "user preamble preserved (R2)" || fail "agent-instructions/3" "preamble"
merge_run "merge case 2 re-run (no-op)" "$d"
assert_contains "agent-instructions/3" "re-run is a no-op" "$MERGE_STDOUT" "no-op"
# constitution migration
d="$(mktmp)"; printf '# R\n\n<!-- vibe:constitution:start -->\nold\n<!-- vibe:constitution:end -->\n' > "$d/AGENTS.md"
merge_run "merge case 3 (constitution -> instructions migration)" "$d"
merge_run "merge case 3 re-run (no-op)" "$d"
grep -qF '<!-- vibe:instructions:start -->' "$d/AGENTS.md" && ! grep -qF 'vibe:constitution' "$d/AGENTS.md" \
  && pass "agent-instructions/3" "constitution -> instructions migration" || fail "agent-instructions/3" "migration"
# merge case 4 — no markers, but the body IS the template's instructions body:
# wrap it in markers instead of appending a second copy. Built with the same
# marker/comment stripping merge's own `tcore` does, since `normalize()` only
# compares blank-squeezed text.
d="$(mktmp)"
awk '/vibe:instructions:start/{f=1;next} /vibe:instructions:end/{f=0} f' "$TEMPLATE" \
  | awk '/^<!-- Managed by vibe/{c=1} c&&/-->$/{c=0;next} !c' > "$d/AGENTS.md"
merge_run "merge case 4 (wrap an unmarked-equivalent body)" "$d"
assert_contains "agent-instructions/3" "unmarked-equivalent body is WRAPPED, not appended twice" "$MERGE_STDOUT" "wrapped"
assert_eq "agent-instructions/3" "wrap does not duplicate the body" \
  "$(grep -c '^## Working model' "$d/AGENTS.md")" "1"
merge_run "merge case 4 re-run (no-op)" "$d"
rm -rf "$d"
# merge case 5 — divergent -> append + warn, preserve content. THE ONE PATH THAT
# IS SUPPOSED TO WRITE TO STDERR, so it is asserted by content rather than by
# emptiness: exactly the two merge-agents WARN lines, nothing else. Anything
# unexpected on stderr here (a `tmp: unbound variable`, a stray tool error) fails
# this the same way it fails the merge_run cases above.
d="$(mktmp)"; printf '# Different\n\nmine\n' > "$d/AGENTS.md"
errf="$(mktmp)/err"; out="$(bash "$MERGE" "$d" 2>"$errf")"
{ grep -qF "mine" "$d/AGENTS.md" && grep -qF '<!-- vibe:instructions:start -->' "$d/AGENTS.md"; } \
  && assert_contains "agent-instructions/3" "divergent appends + warns" "$out" "append" \
  || fail "agent-instructions/3" "divergent append"
assert_eq "inject-triggers/6" "divergent path writes exactly 2 stderr lines, both merge-agents WARNs" \
  "$(wc -l < "$errf" | tr -d ' ')/$(grep -c '^merge-agents: WARN — ' "$errf")" "2/2"
# reversed markers must be refused, not silently mangled (content-safety invariant)
d="$(mktmp)"; printf '# R\n<!-- vibe:instructions:end -->\nmid\n<!-- vibe:instructions:start -->\ntail\n' > "$d/AGENTS.md"
rmbefore="$(cat "$d/AGENTS.md")"
if bash "$MERGE" "$d" >/dev/null 2>&1; then fail "agent-instructions/3" "reversed markers must be refused"; else pass "agent-instructions/3" "reversed markers refused (content safety)"; fi
assert_eq "agent-instructions/3" "reversed-marker file left untouched" "$(cat "$d/AGENTS.md")" "$rmbefore"
# link: skip correct, block real file
d="$(mktmp)"; merge_run "merge before link (create)" "$d"
merge_run "link CLAUDE.md" link CLAUDE.md "$d"
merge_run "link CLAUDE.md re-run (idempotent skip)" link CLAUDE.md "$d"
assert_contains "agent-instructions/5" "symlink idempotent skip" "$MERGE_STDOUT" "skip"
[[ -L "$d/CLAUDE.md" && "$(readlink "$d/CLAUDE.md")" == "AGENTS.md" ]] && pass "agent-instructions/5" "relative symlink to AGENTS.md" || fail "agent-instructions/5" "symlink target"
printf 'real\n' > "$d/WARP.md"
if bash "$MERGE" link WARP.md "$d" >/dev/null 2>&1; then fail "agent-instructions/5" "real file must not be clobbered"; else pass "agent-instructions/5" "real file blocked (R5)"; fi
# awk-less unmerge (marker lookup is grep -n + sed, not awk): with awk absent the
# unmerge path still strips BOTH managed blocks. Discriminating — the old awk
# marker lookup exits 127 under set -e and leaves the block intact.
NOAWK="$(mkshim awk)"
d="$(mktmp)"; merge_run "merge before awk-less unmerge (create)" "$d"
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
d="$(mktmp)"; merge_run "merge before active-rules regen (create)" "$d"
sed 's/_No lessons recorded yet\._/- do the thing/' "$d/AGENTS.md" > "$d/AGENTS.md.n" && mv "$d/AGENTS.md.n" "$d/AGENTS.md"
merge_run "unmerge (regen case)" unmerge "$d"
[[ ! -e "$d/AGENTS.md" ]] \
  && pass "agent-instructions/3" "unmerge deletes the file when only the branded title would remain (regen case)" \
  || fail "agent-instructions/3" "unmerge deletes the file when only the branded title would remain (regen case)"
rm -rf "$d"
# branded-title cleanup, scenario 3 (title MID-FILE): a user who added prose ABOVE
# the vibe title still gets the orphaned title removed on unmerge — the strip is by
# FIRST exact match, wherever it sits, not line 1 only. Discriminating: the old
# head -n1 check sees the user's heading on line 1 and leaves a mid-file title.
d="$(mktmp)"; merge_run "merge before mid-file-title unmerge (create)" "$d"
{ printf '## Top\nkeep top\n\n'; cat "$d/AGENTS.md"; printf '\n## Footer\nkeep bottom\n'; } > "$d/AGENTS.md.n" && mv "$d/AGENTS.md.n" "$d/AGENTS.md"
grep -qF '# AGENTS.md — vibe Engineering Guide' "$d/AGENTS.md" || fail "agent-instructions/3" "precondition: mid-file title present before unmerge"
merge_run "unmerge (mid-file title)" unmerge "$d"
{ ! grep -qF 'vibe Engineering Guide' "$d/AGENTS.md" && grep -qF 'keep top' "$d/AGENTS.md" && grep -qF 'keep bottom' "$d/AGENTS.md"; } \
  && pass "agent-instructions/3" "unmerge strips a MID-FILE vibe title (prose above it), keeps user prose" \
  || fail "agent-instructions/3" "unmerge strips a MID-FILE vibe title (prose above it), keeps user prose"
rm -rf "$d"
# branded-title cleanup, scenario 4 (no title present): a file carrying managed
# blocks but NO vibe title line keeps its own line 1 — the title strip is a no-op.
# Discriminating: an unconditional 'sed 1d' would eat the user's first line.
d="$(mktmp)"
{ printf '## User Heading\nsome prose\n\n'; sed -n '/vibe:instructions:start/,/vibe:active-rules:end/p' "$TEMPLATE"; } > "$d/AGENTS.md"
merge_run "unmerge (no title present)" unmerge "$d"
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
merge_run "merge into the repo's own AGENTS.md (parity no-op)" "$d"
after="$(cksum < "$d/AGENTS.md")"
assert_eq "agent-instructions/2" "repo AGENTS.md block is byte-identical to the shipped template (no drift)" "$after" "$before"
# Discriminating: drop a line from INSIDE the managed block; merge must repair it
# back to the pristine repo file — proving the parity assertion above would fail the
# moment the template gains (or loses) a line the repo block does not mirror.
d="$(mktmp)"; sed '/^## Degrade$/d' "$REPO_AGENTS" > "$d/AGENTS.md"
cmp -s "$d/AGENTS.md" "$REPO_AGENTS" && fail "agent-instructions/2" "precondition: corrupted block differs from pristine" || true
merge_run "merge repairs a drifted repo block" "$d"
cmp -s "$d/AGENTS.md" "$REPO_AGENTS" \
  && pass "agent-instructions/2" "merge repairs a drifted repo block back to the template (parity is enforced)" \
  || fail "agent-instructions/2" "merge repairs a drifted repo block back to the template (parity is enforced)"
rm -rf "$d"

echo ""
echo "=== platform-adapters/4,5 — settings.json hook wiring (auto-wired, no plugin) ==="
# A fresh install wires the three hooks into the target's .claude/settings.json —
# no plugin manifest, no hooks.json (issue #12: settings.json is the single path).
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
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
bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
dupok=1
for ev in SessionStart UserPromptSubmit PreToolUse Stop; do
  n="$(jq --arg e "$ev" --arg m "$VIBE_HOOK_RE" '[.hooks[$e][]? | select([.hooks[]?.command // empty] | any(test($m)))] | length' "$SETTINGS")"
  [[ "$n" == "1" ]] || { dupok=0; echo "        $ev has $n vibe groups (want 1)"; }
done
assert_eq "platform-adapters/4" "re-install does not duplicate vibe hook groups" "$dupok" "1"
rm -rf "$SB"
# --uninstall strips vibe's hook entries but preserves unrelated user settings.
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
SETTINGS="$SB/.claude/settings.json"
jq '. + {permissions:{allow:["Bash(git status:*)"]}}' "$SETTINGS" > "$SETTINGS.tmp" && mv "$SETTINGS.tmp" "$SETTINGS"
bash "$INSTALL" "$SB" --platform claude --uninstall --yes >/dev/null 2>&1
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
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
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
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
# install wires SessionStart to the doctrine hook.
sscmd="$(jq -r '.hooks.SessionStart[]?.hooks[]?.command // empty' "$SB/.claude/settings.json" 2>/dev/null)"
assert_contains "flow-legibility/5" "install wires SessionStart -> session-start-doctrine.sh" "$sscmd" "session-start-doctrine.sh"
# the hook emits the doctrine (durable/ephemeral framing) and NO live state:
# SessionStart output is REPLAYED verbatim on --resume, so a cursor printed here
# goes stale the moment the cursor moves (inject-triggers/5, R4). The
# doctrine assertion is this negative's population floor — there is a payload,
# and it names no state. Discriminating: before R4 this hook printed
# "Cursor: idle." on the line after the block.
ssout="$(CLAUDE_PROJECT_DIR="$SB" bash "$SB/.claude/hooks/session-start-doctrine.sh" </dev/null 2>/dev/null)"
assert_contains "flow-legibility/5" "SessionStart hook emits the doctrine" "$ssout" "sessions are ephemeral"
assert_not_contains "inject-triggers/5" "SessionStart hook emits no cursor summary" "$ssout" "Cursor:"
# doctor reports instruction coverage ok on a fresh install (block + hook wired).
docout="$(bash "$SB/.agents/skills/vibe/scripts/doctor.sh" "$SB" 2>&1)"
assert_contains "flow-legibility/5" "doctor reports instruction.coverage ok" "$docout" "ok   instruction.coverage"
rm -rf "$SB"

# graceful degrade: resolver absent -> exit 0, no output. Isolated in its own
# sandbox (js-core/7 review round 1, Finding 5) so mutilating it (removing
# SKILL.md) can never leak into the doctor.sh coverage check above — a prior
# version reused $SB for both and only passed by accident, because
# instruction.coverage happens not to read SKILL.md. The hook is node-first
# now: its real resolver is the vibe skill's SKILL.md (engine/commands/
# doctrine.mjs reads it directly, no bash script involved), not doctrine.sh —
# remove both so the degrade fires regardless of which implementation runs.
SBD="$(mktmp)"; bash "$INSTALL" "$SBD" --platform claude >/dev/null 2>&1
rm -f "$SBD/.agents/skills/vibe/scripts/doctrine.sh" "$SBD/.agents/skills/vibe/SKILL.md"
ss_rc=0; ssdeg="$(CLAUDE_PROJECT_DIR="$SBD" bash "$SBD/.claude/hooks/session-start-doctrine.sh" </dev/null 2>/dev/null)" || ss_rc=$?
assert_eq "flow-legibility/5" "SessionStart hook exits 0 when the resolver is absent" "$ss_rc" "0"
assert_eq "flow-legibility/5" "SessionStart hook emits nothing when the resolver is absent" "$ssdeg" ""
rm -rf "$SBD"

echo ""
echo "=== flow-legibility/6 — drift-first nudge in the inject hook ==="
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
git init -q "$SB" 2>/dev/null && git -C "$SB" config user.email t@t 2>/dev/null && git -C "$SB" config user.name t 2>/dev/null
rm -f "$SB/.agents/skills/vibe/state.json"      # idle cursor
mkdir -p "$SB/src"; printf 'x\n' > "$SB/src/app.sh"
inj="$(CLAUDE_PROJECT_DIR="$SB" bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" </dev/null 2>/dev/null)"
first="$(printf '%s\n' "$inj" | head -n1)"
# The orders are identified by a phrase ONLY they carry (inject-triggers/4 fix
# round 1, Important). `state=idle` no longer discriminates: the level line
# deliberately opens `state=<state>`, so an assertion on that substring passes
# even with the orders permanently deleted — it was reading the level line and
# reporting on the orders.
IDLE_ORDERS="no active flow"
assert_contains "flow-legibility/6" "inject prepends the drift nudge as line 1 (idle + src edit)" "$first" "vibe-drift:"
assert_contains "flow-legibility/6" "inject still emits the orders after the drift line" "$inj" "$IDLE_ORDERS"
# clean tree -> no drift line; the level line is the first content. The orders
# are EDGE-cadence now: the turn above was the first inject after install (an
# edge by definition), this one is a quiet turn in a cursor that has not moved,
# so the full payload is deliberately absent.
rm -f "$SB/src/app.sh"
inj2="$(CLAUDE_PROJECT_DIR="$SB" bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" </dev/null 2>/dev/null)"
first2="$(printf '%s\n' "$inj2" | head -n1)"
assert_not_contains "flow-legibility/6" "no drift line when no src/tests change" "$inj2" "vibe-drift:"
assert_contains "flow-legibility/6" "the cursor line is the first content when no drift" "$first2" "state=idle · transition:"
assert_not_contains "flow-legibility/6" "the edge payload does not repeat on a settled cursor" "$inj2" "$IDLE_ORDERS"
rm -rf "$SB"

echo ""
echo "=== platform-adapters/1,2,3 — hooks against a real install ==="
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
export CLAUDE_PROJECT_DIR="$SB"
SS="$SB/.agents/skills/vibe/scripts/set-state.sh"
WLOG="$SB/.agents/skills/vibe/warnings.log"
# inject — behavioral: it emits the CURRENT cursor state's orders to stdout (the
# model-visible stream), not a fixed string. Asserting rc=0 from a hook that
# always exits 0 proves nothing, so pin the actual routed content instead.
# Each state's orders are pinned by a phrase unique to THEM, never by the
# `state=<state>` prefix the level line also carries (inject-triggers/4 fix
# round 1, Important).
IDLE_ORDERS="no active flow"
IMPL_ORDERS="skill=vibe · delegate executing-plans"
bash "$SS" idle >/dev/null
out="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" 2>/dev/null)"
assert_contains "platform-adapters/1" "inject emits idle orders for an idle cursor" "$out" "$IDLE_ORDERS"
bash "$SS" feature.impl demo >/dev/null
out="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" 2>/dev/null)"
assert_contains "platform-adapters/1" "inject emits the cursor state's orders (feature.impl)" "$out" "$IMPL_ORDERS"
assert_not_contains "platform-adapters/1" "inject does not emit a foreign state's orders" "$out" "$IDLE_ORDERS"
# inject-triggers/4 — the cadence contract, end to end on a real install: the
# orders ride the turn AFTER the cursor moves and not again, and two settled
# turns are byte-identical (what keeps the prompt cache warm). The turn above
# was the transition turn; these two are the settled ones.
quiet1="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" 2>/dev/null)"
quiet2="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" 2>/dev/null)"
assert_not_contains "inject-triggers/4" "the edge payload rides the move once, not every turn" "$quiet1" "$IMPL_ORDERS"
assert_contains "inject-triggers/4" "the cursor line still rides every turn" "$quiet1" "state=feature.impl · transition:"
assert_eq "inject-triggers/4" "two settled turns are byte-identical" "$quiet1" "$quiet2"
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
# `node` is included (js-core/7): the hook is node-first now, so a farm that
# omits it would accidentally test the UNRELATED "Node absent" degrade (R4,
# always exit 0) instead of the jq-absent path this fixture means to exercise.
NOJQ_BIN="$(mktmp)"
for _t in dirname date mktemp mv rm sed grep head cat bash env awk find node; do
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
# gate: a non-verify state is SILENT since inject-triggers/5 (R6) deleted
# predicate 3 — the stuck-phase nudge said what the per-turn level channel now
# says every turn, and queued a relay line per Stop to say it. Discriminating:
# before R6 this queued 'gate: still in feature.impl ...'.
bash "$SS" feature.impl demo >/dev/null
: > "$WLOG" 2>/dev/null || true
rc=0
printf '{}' | bash "$SB/.claude/hooks/stop-gate.sh" >/dev/null 2>&1 || rc=$?
assert_eq "platform-adapters/3" "gate exits 0 in a non-verify state" "$rc" "0"
grep -q '^gate:' "$WLOG" 2>/dev/null \
  && fail "inject-triggers/5" "gate queues no stuck-phase nudge (predicate 3 deleted)" \
  || pass "inject-triggers/5" "gate queues no stuck-phase nudge (predicate 3 deleted)"
# CONTROL for that negative: the gate can still reach the relay from this very
# install. feature.verify with no feature named is the surviving warn-only path.
bash "$SS" idle >/dev/null; bash "$SS" feature.verify >/dev/null 2>&1
: > "$WLOG" 2>/dev/null || true
rc=0
printf '{}' | bash "$SB/.claude/hooks/stop-gate.sh" >/dev/null 2>&1 || rc=$?
assert_eq "platform-adapters/3" "gate exits 0 when it cannot resolve a receipt path" "$rc" "0"
grep -q '^gate:' "$WLOG" 2>/dev/null \
  && pass "platform-adapters/3" "control: the gate still queues a warn-only smell to the relay log" \
  || fail "platform-adapters/3" "control: the gate still queues a warn-only smell to the relay log"
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

# inject-triggers/6 — the harness must not wedge itself. In a real GIT repo the
# staleness scan runs for real, and the inject hook's own edge-detection marker
# (.vibe/last-inject) used to end up in `git status --porcelain` as the
# collapsed `?? .vibe/` line, which the gate read as "changed after the receipt
# was written" and BLOCKED — every *.verify state, on a path the human never
# touched. Belt (install gitignores it) and braces (the scan skips vibe's own
# runtime writes) are both exercised here, end to end, against a real install.
SBG="$(mktmp)"
if git -C "$SBG" init -q >/dev/null 2>&1 \
   && git -C "$SBG" config user.email t@t >/dev/null 2>&1 \
   && git -C "$SBG" config user.name t >/dev/null 2>&1; then
  bash "$INSTALL" "$SBG" --platform claude >/dev/null 2>&1
  grep -qxF '.vibe/last-inject' "$SBG/.gitignore" \
    && pass "inject-triggers/6" "install gitignores the inject edge-detection marker" \
    || fail "inject-triggers/6" "install gitignores the inject edge-detection marker"
  git -C "$SBG" add -A >/dev/null 2>&1; git -C "$SBG" commit -qm install >/dev/null 2>&1
  CLAUDE_PROJECT_DIR="$SBG" bash "$SBG/.agents/skills/vibe/scripts/set-state.sh" quick.verify >/dev/null 2>&1
  mkdir -p "$SBG/.agents/skills/vibe/evidence"
  printf 'commands + observed output\n' > "$SBG/.agents/skills/vibe/evidence/quick.md"
  sleep 1
  # One inject turn — this is what writes .vibe/last-inject.
  printf '{}' | CLAUDE_PROJECT_DIR="$SBG" bash "$SBG/.claude/hooks/user-prompt-submit-inject.sh" >/dev/null 2>&1
  [[ -f "$SBG/.vibe/last-inject" ]] \
    && pass "inject-triggers/6" "one inject turn writes the marker (population floor for the two checks below)" \
    || fail "inject-triggers/6" "one inject turn writes the marker (population floor for the two checks below)"
  assert_eq "inject-triggers/6" "the marker is invisible to git after install (belt)" \
    "$(git -C "$SBG" status --porcelain)" ""
  out="$(printf '{}' | CLAUDE_PROJECT_DIR="$SBG" bash "$SBG/.claude/hooks/stop-gate.sh" 2>&1; echo "rc=$?")"
  assert_contains "inject-triggers/6" "gate passes quick.verify after an inject turn (fresh receipt)" "$out" "rc=0"
  # Braces: strip the ignore line (a target installed before the marker existed)
  # so git reports the collapsed `?? .vibe/` — the exact wedge, now excluded by
  # the scan itself.
  grep -vxF '.vibe/last-inject' "$SBG/.gitignore" > "$SBG/.gi" && mv "$SBG/.gi" "$SBG/.gitignore"
  # Commit that edit: an uncommitted .gitignore is itself newer than the receipt
  # and would block for its own, unrelated reason — leaving `?? .vibe/` as the
  # ONLY thing the scan has to judge, which is the point of this leg.
  git -C "$SBG" add .gitignore >/dev/null 2>&1; git -C "$SBG" commit -qm legacy >/dev/null 2>&1
  # Asserted with the SAME argv the gate itself runs (`-uall`): the marker is one
  # enumerated row, not the collapsed `?? .vibe/` directory a plain --porcelain
  # would report. That enumeration is what lets the exclusion name the marker
  # without swallowing whatever else lives under .vibe/ (fix round 1).
  assert_eq "inject-triggers/6" "control: with the ignore line stripped git enumerates the marker itself" \
    "$(git -C "$SBG" status --porcelain -uall)" "?? .vibe/last-inject"
  out="$(printf '{}' | CLAUDE_PROJECT_DIR="$SBG" bash "$SBG/.claude/hooks/stop-gate.sh" 2>&1; echo "rc=$?")"
  assert_contains "inject-triggers/6" "gate still passes with the marker VISIBLE to git (braces)" "$out" "rc=0"
  # ... and an authored block beside the marker, in that same untracked
  # directory, DOES stale the receipt: it changes what every later turn injects.
  sleep 1; mkdir -p "$SBG/.vibe/blocks"; printf 'authored\n' > "$SBG/.vibe/blocks/team.md"
  out="$(printf '{}' | CLAUDE_PROJECT_DIR="$SBG" bash "$SBG/.claude/hooks/stop-gate.sh" 2>&1; echo "rc=$?")"
  assert_contains "inject-triggers/6" "an authored .vibe/blocks file newer than the receipt BLOCKS" "$out" "rc=2"
  assert_contains "inject-triggers/6" "the block names the authored file, not the directory" "$out" ".vibe/blocks/team.md"
  rm -rf "$SBG/.vibe/blocks"
  # ... and the tooth is undamaged: a real source file touched after the receipt
  # still blocks, from this same install, this same turn.
  sleep 1; printf 'edited\n' > "$SBG/src.txt"
  out="$(printf '{}' | CLAUDE_PROJECT_DIR="$SBG" bash "$SBG/.claude/hooks/stop-gate.sh" 2>&1; echo "rc=$?")"
  assert_contains "inject-triggers/6" "control: a real source file newer than the receipt still BLOCKS" "$out" "rc=2"
  assert_contains "inject-triggers/6" "the block names the real changed path, not vibe runtime state" "$out" "src.txt"
else
  fail "inject-triggers/6" "could not init a git repo for the receipt-staleness fixture"
fi
rm -rf "$SBG"

# js-core/7 review round 1, Finding 1 (Important): R4 had zero regression
# coverage — the brief's own test-first steps ("with node shimmed away, each
# hook exits 0 and emits nothing"; "guard with node absent exits 0, not 2")
# were never added. mkshim's curated tool list never includes `node`, so a
# plain mkshim() PATH already IS a node-absent farm. All four hooks, all
# must exit 0 with empty stdout; the guard specifically must NOT invert a
# real block into exit 2.
NONODE_BIN="$(mkshim)"
bash "$SS" feature.verify demo >/dev/null
for h in session-start-doctrine user-prompt-submit-inject; do
  out="$(printf '{}' | PATH="$NONODE_BIN" bash "$SB/.claude/hooks/$h.sh" 2>&1; echo "rc=$?")"
  assert_eq "js-core/7" "node-absent: $h exits 0 silently" "$out" "rc=0"
done
# The two HARD-BLOCK hooks must NOT lose their teeth without node. They fall back
# to flow/hooks-fallback/*.sh — the frozen pre-port bash implementations, which
# install ships into the target — so a node-less machine is still enforced.
# `command -v node || exit 0` used to drop both blocks silently while
# detect-context.sh's header still promised "a target without node must still be
# enforced"; these two assertions are what keep that promise true.
out="$(printf '{"tool_name":"Write","tool_input":{"file_path":".agents/skills/vibe/state.json"}}' \
  | PATH="$NONODE_BIN" bash "$SB/.claude/hooks/pre-tool-use-guard.sh" 2>&1; echo "rc=$?")"
assert_contains "js-core/7" "node-absent: guard still BLOCKS via the bash fallback" "$out" "rc=2"
assert_contains "js-core/7" "node-absent: the block names the guarded path" "$out" "state.json"
# …and an ALLOWED write is still allowed: the fallback must not block everything.
out="$(printf '{"tool_name":"Write","tool_input":{"file_path":"README.md"}}' \
  | PATH="$NONODE_BIN" bash "$SB/.claude/hooks/pre-tool-use-guard.sh" 2>&1; echo "rc=$?")"
assert_contains "js-core/7" "node-absent: guard still ALLOWS an unguarded path" "$out" "rc=0"
# The Stop gate's evidence tooth, same story. Remove any receipt an earlier
# section left behind first — the tooth fires on its ABSENCE, so a stale receipt
# would make this assertion pass for the wrong reason.
rm -f "$SB/.agents/skills/vibe/evidence/feature-demo.md"
out="$(printf '{}' | PATH="$NONODE_BIN" bash "$SB/.claude/hooks/stop-gate.sh" 2>&1; echo "rc=$?")"
assert_contains "js-core/7" "node-absent: Stop gate still BLOCKS without a receipt" "$out" "rc=2"
assert_contains "js-core/7" "node-absent: the Stop block names the expected receipt" "$out" "feature-demo.md"
# …and with the receipt present it does NOT block — the fallback is a gate, not a wall.
mkdir -p "$SB/.agents/skills/vibe/evidence"
printf 'ran: bash tests/run.sh -> ALL SUITES PASSED\n' > "$SB/.agents/skills/vibe/evidence/feature-demo.md"
out="$(printf '{}' | PATH="$NONODE_BIN" bash "$SB/.claude/hooks/stop-gate.sh" 2>&1; echo "rc=$?")"
assert_contains "js-core/7" "node-absent: Stop gate passes once the receipt exists" "$out" "rc=0"
rm -f "$SB/.agents/skills/vibe/evidence/feature-demo.md"

# js-core/7 review round 1, Finding 2 (Important): node present but the
# engine directory missing (a target installed before the engine shipped, or
# a moved/broken symlink) must degrade the same way, not crash with a raw
# Node MODULE_NOT_FOUND stack trace. Move the engine aside, not delete —
# restore it immediately after so the rest of the suite is unaffected.
#
# js-core/8: trap-protected. Without this, a future assertion helper that
# hard-exits (or a stray `set -e` creeping into this section) could abort
# the script between the mv-aside and the mv-back below, leaving $SB
# permanently engine-less for any LATER section that reuses this trick —
# and silently, since nothing downstream would notice engine.bak sitting
# there instead of engine. The EXIT trap makes the restore unconditional;
# `trap - EXIT` immediately after the normal mv-back disarms it so it never
# double-fires (harmless either way — the second mv would just no-op via
# `|| true` once engine.bak is already gone) and never lingers to interact
# with any EXIT trap a later section might legitimately want to install.
trap 'mv -f "$SB/.agents/skills/vibe/engine.bak" "$SB/.agents/skills/vibe/engine" 2>/dev/null || true' EXIT
mv "$SB/.agents/skills/vibe/engine" "$SB/.agents/skills/vibe/engine.bak"
for h in session-start-doctrine user-prompt-submit-inject; do
  out="$(printf '{}' | bash "$SB/.claude/hooks/$h.sh" 2>&1; echo "rc=$?")"
  assert_eq "js-core/7" "engine-absent: $h exits 0 silently (no MODULE_NOT_FOUND crash)" "$out" "rc=0"
done
out="$(printf '{"tool_name":"Write","tool_input":{"file_path":".agents/skills/vibe/state.json"}}' \
  | bash "$SB/.claude/hooks/pre-tool-use-guard.sh" 2>&1; echo "rc=$?")"
assert_contains "js-core/7" "engine-absent: guard still BLOCKS via the bash fallback" "$out" "rc=2"
assert_not_contains "js-core/7" "engine-absent: guard prints no Node stack trace" "$out" "MODULE_NOT_FOUND"
# A PRESENT but BROKEN engine is the case the old `[[ -f cli.mjs ]]` guard missed:
# `exec node` printed a raw stack trace on every prompt and tool call, and the
# guard failed open. Now the shim keeps only exit 0 and 2 as verdicts.
mkdir -p "$SB/.agents/skills/vibe/engine"
printf 'this is not valid javascript (((\n' > "$SB/.agents/skills/vibe/engine/cli.mjs"
out="$(printf '{"tool_name":"Write","tool_input":{"file_path":".agents/skills/vibe/state.json"}}' \
  | bash "$SB/.claude/hooks/pre-tool-use-guard.sh" 2>&1; echo "rc=$?")"
assert_contains "js-core/7" "engine-broken: guard still BLOCKS via the bash fallback" "$out" "rc=2"
assert_not_contains "js-core/7" "engine-broken: no raw Node stack trace reaches the user" "$out" "at compileSourceTextModule"
out="$(printf '{}' | bash "$SB/.claude/hooks/user-prompt-submit-inject.sh" 2>&1; echo "rc=$?")"
assert_contains "js-core/7" "engine-broken: inject exits 0, no stack trace" "$out" "rc=0"
assert_not_contains "js-core/7" "engine-broken: inject prints no raw stack trace" "$out" "at compileSourceTextModule"
rm -rf "$SB/.agents/skills/vibe/engine"
mv "$SB/.agents/skills/vibe/engine.bak" "$SB/.agents/skills/vibe/engine"
trap - EXIT


# inject-triggers/6 fix round 1, Important 2 — a NODE-LESS install must still
# receive the write rules. They are no longer restated by hand in the
# instructions block (they render from content/policy.json), and
# `render agents-md --write` needs node; on a hookless host AGENTS.md is the
# only carrier there is, so a missing block would be a straight regression.
# merge-agents.sh seeds the template's pre-rendered copy, and the template's
# copy is byte-identical to the render (flow/tests/run.sh proves that), so the
# same target re-installed WITH node re-renders to "no change".
SBNN="$(mktmp)"
PATH="$NONODE_BIN" "$NONODE_BIN/bash" "$INSTALL" "$SBNN" --platform claude --only flow >/dev/null 2>&1
grep -qxF '<!-- vibe:rules -->' "$SBNN/AGENTS.md" \
  && pass "inject-triggers/6" "node-absent install still ships the vibe:rules block" \
  || fail "inject-triggers/6" "node-absent install still ships the vibe:rules block"
grep -qF 'writable only during feature.compound' "$SBNN/AGENTS.md" \
  && pass "inject-triggers/6" "node-absent install carries the real enumerated write rules" \
  || fail "inject-triggers/6" "node-absent install carries the real enumerated write rules"
grep -qF 'Delegating to sub-agents' "$SBNN/AGENTS.md" \
  && pass "inject-triggers/6" "node-absent install carries the rest of the agents-md channel too" \
  || fail "inject-triggers/6" "node-absent install carries the rest of the agents-md channel too"
if command -v node >/dev/null 2>&1; then
  out="$( cd "$SBNN" && node "$SBNN/.agents/skills/vibe/engine/cli.mjs" render agents-md --write 2>&1 )"
  assert_contains "inject-triggers/6" "re-rendering that target with node is a NO-OP (the shipped copy is the render)" \
    "$out" "no change"
else
  echo "  SKIP [inject-triggers/6] re-render idempotence (node not on PATH)"
fi
rm -rf "$SBNN"

unset CLAUDE_PROJECT_DIR
rm -rf "$SB"

echo ""
echo "=== flow-mvp — set-state.sh jq-optional parity ==="
# jq is recommended, not required: without it set-state.sh still writes the cursor
# (via printf) and the output must be byte-identical to the jq path on the same
# transition. Run both against a fresh install with a jq-free PATH; normalize only
# the turn-varying `updated` timestamp before comparing.
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
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
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
ok=1
for p in .agents/skills/vibe/scripts/orders.sh .agents/skills/vibe/SKILL.md .claude/hooks/stop-gate.sh .claude/settings.json AGENTS.md .agents/skills/vibe/state.json; do
  [[ -e "$SB/$p" ]] || { ok=0; echo "        missing $p"; }
done
assert_eq "platform-adapters/6" "install lays down core + adapter + cursor" "$ok" "1"
grep -qF '.agents/skills/vibe/state.json' "$SB/.gitignore" && pass "platform-adapters/6" "install gitignores the cursor" || fail "platform-adapters/6" "gitignore"
before="$(cat "$SB/AGENTS.md")"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1; after="$(cat "$SB/AGENTS.md")"
assert_eq "platform-adapters/6" "re-install is idempotent (AGENTS.md unchanged)" "$before" "$after"
# a live cursor must survive a re-install — idempotency may not clobber flow state
bash "$SB/.agents/skills/vibe/scripts/set-state.sh" feature.impl widget >/dev/null
before_cur="$(cat "$SB/.agents/skills/vibe/state.json")"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1; after_cur="$(cat "$SB/.agents/skills/vibe/state.json")"
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
SB="$(mktmp)"; bash "$FS/install.sh" "$SB" --platform claude >/dev/null 2>&1
[[ ! -e "$SB/.agents/skills/vibe/evidence/feature-demo.md" ]] \
  && pass "platform-adapters/6" "source-side evidence receipt does not ship to the target" \
  || fail "platform-adapters/6" "source-side evidence receipt leaked into the target (gate bypass)"
# ...but the target's OWN receipts survive a re-install (runtime-state lesson).
mkdir -p "$SB/.agents/skills/vibe/evidence"
printf 'TARGET own receipt\n' > "$SB/.agents/skills/vibe/evidence/feature-widget.md"
bash "$FS/install.sh" "$SB" --platform claude >/dev/null 2>&1
{ [[ -f "$SB/.agents/skills/vibe/evidence/feature-widget.md" ]] \
  && [[ ! -e "$SB/.agents/skills/vibe/evidence/feature-demo.md" ]]; } \
  && pass "platform-adapters/6" "target evidence survives re-install; source evidence still excluded" \
  || fail "platform-adapters/6" "re-install must preserve target evidence and exclude source evidence"
rm -rf "$SB" "$FS"
# SCOPE CONTRACT (discriminating): scrub_source_only must remove $dst/AGENTS.md
# only when $src carries one — today's real source always ships one, so this
# needs a fake source, at a path (AGENTS.md) the SOURCE genuinely lacks, to be
# discriminating. Mirrors flow/+spec/ top-level (not just the .agents/skills/*
# symlink targets the FS fixture above uses) so the network-bootstrap check at
# the top of install.sh (which requires flow/state-machine.json + spec/SKILL.md)
# sees a real local checkout and never fetches over the network.
FS2="$(mktmp)"
cp "$INSTALL" "$FS2/install.sh"
mkdir -p "$FS2/.agents/skills" "$FS2/.claude"
cp -RL "$REPO_ROOT/flow" "$FS2/flow"
cp -RL "$REPO_ROOT/spec" "$FS2/spec"
ln -s ../../flow "$FS2/.agents/skills/vibe"
ln -s ../../spec "$FS2/.agents/skills/spec"
cp -RL "$REPO_ROOT/.claude/commands" "$FS2/.claude/commands"
cp -RL "$REPO_ROOT/.claude/hooks" "$FS2/.claude/hooks"
rm -f "$FS2/flow/AGENTS.md"
SB="$(mktmp)"; bash "$FS2/install.sh" "$SB" --platform claude >/dev/null 2>&1
printf 'mine, not the contributor guide\n' > "$SB/.agents/skills/vibe/AGENTS.md"
[[ -f "$SB/.agents/skills/vibe/AGENTS.md" ]] \
  && pass "platform-adapters/6" "fixture: target-owned AGENTS.md exists before re-install (population floor)" \
  || fail "platform-adapters/6" "fixture: target-owned AGENTS.md exists before re-install (population floor)"
bash "$FS2/install.sh" "$SB" --platform claude >/dev/null 2>&1
[[ -f "$SB/.agents/skills/vibe/AGENTS.md" ]] \
  && pass "platform-adapters/6" "target-owned AGENTS.md survives a re-install once source stops shipping one" \
  || fail "platform-adapters/6" "target-owned AGENTS.md must survive a re-install once source stops shipping one"
rm -rf "$SB" "$FS2"
# gitignore nitpick: a freshly created .gitignore must NOT start with a blank line.
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
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
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
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
bash "$INSTALL" "$SB" --platform claude --uninstall --yes >/dev/null 2>&1
{ [[ -f "$SB/.claude/skills/myskill/SKILL.md" ]] \
  && [[ ! -L "$SB/.claude/skills/spec" && ! -e "$SB/.claude/skills/spec" ]] \
  && [[ ! -L "$SB/.claude/skills/vibe" && ! -e "$SB/.claude/skills/vibe" ]]; } \
  && pass "platform-adapters/6" "uninstall removes the vibe skill symlinks, keeps a user skill dir" \
  || fail "platform-adapters/6" "uninstall removes the vibe skill symlinks, keeps a user skill dir"
rm -rf "$SB"
# a user's REAL .claude/skills/spec directory must NOT be clobbered by registration.
SB="$(mktmp)"; mkdir -p "$SB/.claude/skills/spec"; printf 'USER real spec\n' > "$SB/.claude/skills/spec/SKILL.md"
bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
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
out="$(bash "$INSTALL" "$SB" --platform claude --dry-run 2>&1; echo "rc=$?")"
after="$(tree_fp "$SB")"
assert_contains "install-tooling/1" "dry-run exits 0" "$out" "rc=0"
assert_contains "install-tooling/1" "dry-run prints an action plan" "$out" "[dry-run] would"
assert_eq "install-tooling/1" "dry-run on a fresh target writes nothing" "$before" "$after"
[[ ! -e "$SB/.agents" && ! -e "$SB/AGENTS.md" && ! -e "$SB/.gitignore" ]] \
  && pass "install-tooling/1" "dry-run creates no managed files" \
  || fail "install-tooling/1" "dry-run creates no managed files"
# --dry-run against an already-installed target is also byte-identical.
bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
before="$(tree_fp "$SB")"
bash "$INSTALL" "$SB" --platform claude --dry-run >/dev/null 2>&1
after="$(tree_fp "$SB")"
assert_eq "install-tooling/1" "dry-run on an installed target is byte-identical" "$before" "$after"
# unknown option is rejected, not treated as the target.
out="$(bash "$INSTALL" "$SB" --platform claude --bogus 2>&1; echo "rc=$?")"
assert_contains "install-tooling/1" "unknown option exits non-zero" "$out" "rc=1"
assert_contains "install-tooling/1" "unknown option names itself" "$out" "unknown option"
rm -rf "$SB"

echo ""
echo "=== install-tooling/2 — --only spec|flow ==="
# --only spec: spec skill only, no flow/adapter/plugin trace.
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude --only spec >/dev/null 2>&1
ok=1
[[ -d "$SB/.agents/skills/spec" ]] || { ok=0; echo "        missing spec skill"; }
[[ ! -e "$SB/.agents/skills/vibe" ]] || { ok=0; echo "        vibe present under --only spec"; }
[[ ! -e "$SB/.claude/hooks" ]] || { ok=0; echo "        .claude/hooks present under --only spec"; }
[[ ! -e "$SB/.claude/settings.json" ]] || { ok=0; echo "        settings.json present under --only spec"; }
assert_eq "install-tooling/2" "--only spec installs the spec half alone" "$ok" "1"
rm -rf "$SB"
# --only flow: flow + adapter present, no spec skill.
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude --only flow >/dev/null 2>&1
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
out="$(bash "$INSTALL" "$SB" --platform claude --only bogus 2>&1; echo "rc=$?")"
assert_contains "install-tooling/2" "--only bogus exits 1" "$out" "rc=1"
assert_contains "install-tooling/2" "--only bogus names the valid values" "$out" "spec"
rm -rf "$SB"
# --only composes with --dry-run (still writes nothing).
SB="$(mktmp)"; before="$(tree_fp "$SB")"
bash "$INSTALL" "$SB" --platform claude --only spec --dry-run >/dev/null 2>&1
after="$(tree_fp "$SB")"
assert_eq "install-tooling/2" "--only spec --dry-run writes nothing" "$before" "$after"
rm -rf "$SB"

echo ""
echo "=== install-tooling/3 — --uninstall ==="
# Install, add user content, then uninstall: managed files gone, user content kept.
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
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
# content-layer: the install rendered the agents-md channel into AGENTS.md's
# vibe:rules block (needs node; skipped honestly when the runner has none, so an
# absent runtime never reads as a passing assertion).
if command -v node >/dev/null 2>&1; then
  grep -qF '<!-- vibe:rules -->' "$SB/AGENTS.md" \
    && pass "content-layer" "install renders the agents-md channel into AGENTS.md" \
    || fail "content-layer" "install renders the agents-md channel into AGENTS.md"
  grep -qF 'Delegating to sub-agents' "$SB/AGENTS.md" \
    && pass "content-layer" "the shipped delegation ruleset reaches AGENTS.md" \
    || fail "content-layer" "the shipped delegation ruleset reaches AGENTS.md"
else
  echo "  SKIP [content-layer] agents-md render (node not on PATH)"
fi
bash "$INSTALL" "$SB" --platform claude --uninstall --yes >/dev/null 2>&1
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
# content-layer: the rendered vibe:rules block is vibe-authored prose too — an
# uninstall that strips only the two older blocks leaves it stranded.
grep -qF "vibe:rules" "$SB/AGENTS.md" && fail "content-layer" "managed vibe:rules block removed on uninstall" || pass "content-layer" "managed vibe:rules block removed on uninstall"
[[ -f "$SB/.spec/product.md" ]] && pass "install-tooling/3" ".spec/ preserved across uninstall" || fail "install-tooling/3" ".spec/ preserved"
rm -rf "$SB"
# uninstall inverse (fix): adapter symlinks + vibe-created stub AGENTS.md.
# Fresh install with --adapters, nothing customized -> uninstall removes the
# CLAUDE.md/WARP.md symlinks AND the untouched-stub AGENTS.md (target had none).
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude --adapters claude,warp >/dev/null 2>&1
[[ -L "$SB/CLAUDE.md" && -L "$SB/WARP.md" ]] || fail "install-tooling/3" "precondition: adapters symlinked"
bash "$INSTALL" "$SB" --platform claude --uninstall --yes >/dev/null 2>&1
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
bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
bash "$INSTALL" "$SB" --platform claude --uninstall --yes >/dev/null 2>&1
{ [[ -f "$SB/CLAUDE.md" && ! -L "$SB/CLAUDE.md" ]] && grep -qF "my real claude guide" "$SB/CLAUDE.md"; } \
  && pass "install-tooling/3" "a user's real CLAUDE.md file survives uninstall" \
  || fail "install-tooling/3" "uninstall clobbered a user's real CLAUDE.md file"
rm -rf "$SB"
# Live cursor + evidence receipt + no --yes -> both survive (flow-mvp verify fixes:
# discriminating test — fails if preservation is swapped for naive rm -rf).
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
bash "$SB/.agents/skills/vibe/scripts/set-state.sh" feature.impl widget >/dev/null 2>&1
mkdir -p "$SB/.agents/skills/vibe/evidence"
printf 'receipt: tests run\n' > "$SB/.agents/skills/vibe/evidence/feature-widget.md"
# The inject marker is runtime state like the cursor — seeded here so the
# --yes assertions below have a population to examine, and so the no---yes
# leg proves preservation rather than absence.
mkdir -p "$SB/.vibe"; printf 'feature.impl widget\n' > "$SB/.vibe/last-inject"
bash "$INSTALL" "$SB" --platform claude --uninstall >/dev/null 2>&1
[[ -f "$SB/.vibe/last-inject" ]] \
  && pass "inject-triggers/6" "inject marker survives uninstall without --yes" \
  || fail "inject-triggers/6" "inject marker survives uninstall without --yes"
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
bash "$INSTALL" "$SB" --platform claude --uninstall --yes >/dev/null 2>&1
[[ ! -e "$SB/.agents/skills/vibe/state.json" ]] && pass "install-tooling/3" "--yes removes the cursor too" || fail "install-tooling/3" "--yes removes the cursor"
if [[ -f "$SB/.gitignore" ]] && grep -qF "evidence" "$SB/.gitignore"; then
  fail "flow-mvp/9" "--yes strips the evidence gitignore stanza"
else
  pass "flow-mvp/9" "--yes strips the evidence gitignore stanza"
fi
# ... including the inject marker stanza, and the marker file itself.
if [[ -f "$SB/.gitignore" ]] && grep -qF ".vibe/last-inject" "$SB/.gitignore"; then
  fail "inject-triggers/6" "--yes strips the inject-marker gitignore stanza"
else
  pass "inject-triggers/6" "--yes strips the inject-marker gitignore stanza"
fi
[[ ! -e "$SB/.vibe/last-inject" ]] \
  && pass "inject-triggers/6" "--yes removes the inject marker (runtime state, like the cursor)" \
  || fail "inject-triggers/6" "--yes removes the inject marker (runtime state, like the cursor)"
rm -rf "$SB"
# Reversed-marker AGENTS.md -> uninstall refuses to touch it (marker lesson regression).
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
printf '# R\n<!-- vibe:instructions:end -->\nmid\n<!-- vibe:instructions:start -->\ntail\n' > "$SB/AGENTS.md"
rmbefore="$(cat "$SB/AGENTS.md")"
bash "$INSTALL" "$SB" --platform claude --uninstall --yes >/dev/null 2>&1
assert_eq "install-tooling/3" "reversed-marker AGENTS.md left byte-untouched by uninstall" "$(cat "$SB/AGENTS.md")" "$rmbefore"
rm -rf "$SB"
# --uninstall composes with --dry-run (writes nothing) and --only (one half).
SB="$(mktmp)"; bash "$INSTALL" "$SB" --platform claude >/dev/null 2>&1
before="$(tree_fp "$SB")"
bash "$INSTALL" "$SB" --platform claude --uninstall --dry-run >/dev/null 2>&1
after="$(tree_fp "$SB")"
assert_eq "install-tooling/3" "--uninstall --dry-run writes nothing" "$before" "$after"
bash "$INSTALL" "$SB" --platform claude --uninstall --only spec --yes >/dev/null 2>&1
[[ ! -e "$SB/.agents/skills/spec" && -e "$SB/.agents/skills/vibe/SKILL.md" ]] \
  && pass "install-tooling/3" "--uninstall --only spec removes just the spec half" \
  || fail "install-tooling/3" "--uninstall --only spec removes just the spec half"
rm -rf "$SB"

echo ""
echo "=== js-core/8 — source-only artifacts never reach an install target ==="
# Final review, I1: the scrub named `$TARGET/.agents/skills/vibe/tests` LITERALLY,
# so js-core's `flow/engine/tests` — one level deeper — shipped 17 files / 396K of
# oracle-spawning test code into every user repo. That contradicted the comment
# directly above it AND falsified the premise the R1 primitive scan's one `tests/`
# exemption is argued on ("install.sh scrubs tests/").
#
# STRUCTURAL, deliberately not a list of the two directory names that exist today:
# any file under any directory named `tests`, and any file NAMED like a test,
# anywhere in the target, is a leak. A third co-located suite would be caught
# without editing this block.
SBt="$(mktmp)"; bash "$INSTALL" "$SBt" --platform claude >/dev/null 2>&1
# Precondition, so an empty `find` below can never read as a vacuous pass: the
# engine really is in the target, i.e. there was something to leak FROM.
[[ -f "$SBt/.agents/skills/vibe/engine/cli.mjs" ]] \
  && pass "js-core/8" "precondition: the install target really carries the engine" \
  || fail "js-core/8" "precondition: the install target really carries the engine"
leak_dirs="$(cd "$SBt" && find . -type d -name tests | sed 's#^\./##' | sort | tr '\n' ' ')"
assert_eq "js-core/8" "no directory named tests reaches an install target" "$leak_dirs" ""
leak_files="$(cd "$SBt" && find . -type f -path '*/tests/*' | sed 's#^\./##' | sort | tr '\n' ' ')"
assert_eq "js-core/8" "no file under a tests/ directory reaches an install target" "$leak_files" ""
leak_named="$(cd "$SBt" && find . -type f \( -name '*.test.*' -o -name 'test_*' -o -name '*_test.*' \) | sed 's#^\./##' | sort | tr '\n' ' ')"
assert_eq "js-core/8" "no test-NAMED file reaches an install target (wherever it sits)" "$leak_named" ""
# The scrub is source-enumerated, so a user's own file inside the skill tree at a
# path the SOURCE does not have must survive a re-install untouched.
echo 'mine' > "$SBt/.agents/skills/vibe/engine/user-note.txt"
bash "$INSTALL" "$SBt" --platform claude >/dev/null 2>&1
[[ -f "$SBt/.agents/skills/vibe/engine/user-note.txt" ]] \
  && pass "js-core/8" "re-install's scrub never touches a user file the source does not have" \
  || fail "js-core/8" "re-install's scrub never touches a user file the source does not have"
rm -rf "$SBt"
# LEGACY cleanup, the other leg of the same family: a target installed BEFORE the
# scrub was fixed already carries engine/tests. --uninstall must still remove it —
# remove_shipped's exclude list is top-level-only precisely so those files are
# cleaned up rather than stranded.
SBl="$(mktmp)"; bash "$INSTALL" "$SBl" --platform claude >/dev/null 2>&1
mkdir -p "$SBl/.agents/skills/vibe/engine/tests"
cp "$REPO_ROOT/flow/engine/tests/json.test.mjs" "$SBl/.agents/skills/vibe/engine/tests/json.test.mjs"
[[ -f "$SBl/.agents/skills/vibe/engine/tests/json.test.mjs" ]] \
  && pass "js-core/8" "precondition: legacy-shaped target carries engine/tests" \
  || fail "js-core/8" "precondition: legacy-shaped target carries engine/tests"
bash "$INSTALL" "$SBl" --platform claude --uninstall --yes >/dev/null 2>&1
legacy_left="$(cd "$SBl" && find . -type f -path '*/tests/*' 2>/dev/null | sed 's#^\./##' | sort | tr '\n' ' ')"
assert_eq "js-core/8" "--uninstall cleans up a legacy target's shipped engine/tests" "$legacy_left" ""
rm -rf "$SBl"

echo ""
echo "=== install.sh — single-command modes ==="
# --help renders the new two-mode usage (the help prints the leading comment block).
assert_contains "install-modes" "--help documents the two modes" "$(bash "$INSTALL" --help 2>&1)" "One command, two modes"
# Bare run (no target) defaults to the current directory — the single-command path.
SBc="$(mktmp)"; git -C "$SBc" init -q >/dev/null 2>&1
bare_out="$( cd "$SBc" && bash "$INSTALL" --dry-run --local --platform none 2>&1 )"
assert_contains "install-modes" "bare run targets the current directory" "$bare_out" "$SBc/.agents/skills"
rm -rf "$SBc"
# Bare run from a SUBDIRECTORY targets the ENCLOSING repo root by marker search, not
# the subdir (review: install.sh must not naively default to $PWD from a subdir).
SBsub="$(mktmp)"; git -C "$SBsub" init -q >/dev/null 2>&1; mkdir -p "$SBsub/pkg/src"
subout="$( cd "$SBsub/pkg/src" && bash "$INSTALL" --dry-run --local --platform none 2>&1 )"
assert_contains "install-modes" "bare run from a subdir targets the repo root" "$subout" "$SBsub/.agents/skills"
assert_not_contains "install-modes" "bare run from a subdir skips the subdir" "$subout" "pkg/src/.agents"
rm -rf "$SBsub"
# --global --dry-run writes nothing and describes the per-user plugin plan.
SBg="$(mktmp)"
gout="$( cd "$SBg" && bash "$INSTALL" --global --dry-run 2>&1 )"
assert_contains "install-modes" "--global plans a per-user plugin install" "$gout" "install plugin vibe@vibe at user scope"
{ [[ ! -e "$SBg/.agents" && ! -e "$SBg/.claude" ]] && pass "install-modes" "--global --dry-run writes nothing"; } || fail "install-modes" "--global dry-run wrote files"
rm -rf "$SBg"
# A REAL --global without the claude CLI errors clearly (graceful, not a stack
# trace) — and exits before any mutation. A dry-run prints the plan without the CLI
# (it touches nothing), so the error path is exercised with a real run here.
noclaude="$(mkshim claude)"
SBn="$(mktmp)"
nout="$( cd "$SBn" && PATH="$noclaude" bash "$INSTALL" --global 2>&1 || true )"
assert_contains "install-modes" "real --global without claude CLI errors clearly" "$nout" "needs the 'claude' CLI"
rm -rf "$noclaude" "$SBn"
# An explicitly-empty target must be REJECTED, never fall through to cwd — else
# `install.sh "" --uninstall` would strip vibe from the current directory. Regression
# for a dogfood near-miss (an unset shell var expanded to an empty argument).
SBe="$(mktmp)"
eout="$( cd "$SBe" && bash "$INSTALL" "" --uninstall --yes 2>&1; echo "rc=$?" )"
assert_contains "install-modes" "empty target is rejected outright" "$eout" "empty target"
assert_not_contains "install-modes" "empty target does not reach uninstall" "$eout" "uninstalling vibe"
rm -rf "$SBe"

echo "=== vibe-plugin — marketplace + plugin payload ==="
PLUGIN_JSON="$REPO_ROOT/plugin/.claude-plugin/plugin.json"
MARKET_JSON="$REPO_ROOT/.claude-plugin/marketplace.json"
BUILD_PLUGIN="$REPO_ROOT/build-plugin.sh"
if command -v jq >/dev/null 2>&1; then
  jq -e . "$PLUGIN_JSON" >/dev/null 2>&1 && pass "vibe-plugin" "plugin.json is valid JSON" || fail "vibe-plugin" "plugin.json invalid/missing"
  jq -e . "$MARKET_JSON" >/dev/null 2>&1 && pass "vibe-plugin" "marketplace.json is valid JSON" || fail "vibe-plugin" "marketplace.json invalid/missing"
  # plugin.json MUST NOT declare `hooks` (hooks/hooks.json auto-loads; declaring it
  # too double-loads and fails plugin load) nor `commands` (/flow needs the per-repo
  # cursor writer the plugin does not carry). Both verified live against the CLI.
  assert_eq "vibe-plugin" "plugin.json declares no hooks field (auto-loaded)" "$(jq -r 'has("hooks")' "$PLUGIN_JSON")" "false"
  assert_eq "vibe-plugin" "plugin.json declares no commands field" "$(jq -r 'has("commands")' "$PLUGIN_JSON")" "false"
  assert_eq "vibe-plugin" "plugin.json bundles ./skills/" "$(jq -r '.skills' "$PLUGIN_JSON")" "./skills/"
else
  { [[ -f "$PLUGIN_JSON" && -f "$MARKET_JSON" ]] && pass "vibe-plugin" "manifests present (no jq)"; } || fail "vibe-plugin" "manifests missing"
fi
# Both skills resolve to SKILL.md through symlinks to the canonical spec/ + flow/
# trees (zero duplication; `claude plugin install` dereferences them into real cache
# dirs — verified live: Skills (2) spec, vibe). A commands/ dir is what mis-scans as
# a phantom skill, so the plugin ships none.
{ [[ -L "$REPO_ROOT/plugin/skills/spec" && -L "$REPO_ROOT/plugin/skills/vibe" \
     && -f "$REPO_ROOT/plugin/skills/spec/SKILL.md" && -f "$REPO_ROOT/plugin/skills/vibe/SKILL.md" ]] \
  && pass "vibe-plugin" "spec + vibe skills are symlinks resolving to SKILL.md"; } || fail "vibe-plugin" "skills not symlinked/resolvable"
# No per-project runtime state may ship in the plugin. TWO checks, because the
# plugin's skills are symlinks and the two halves of "the payload" differ:
#
#  (1) the GENERATED scaffold — plain `find`, which stops at the symlinks. That is
#      the half build-plugin.sh writes itself.
#  (2) what a marketplace fetch actually delivers — `claude plugin install`
#      DEREFERENCES the skill symlinks, so the real payload is the committed
#      content of spec/ + flow/. Asked with plain `find` that was vacuously true
#      (it never descended), which is how runtime state could have shipped
#      unnoticed. Ask git instead: it answers for the tree a fetch gets, and stays
#      stable on a developer's dirty checkout, where flow/warnings.log legitimately
#      exists and is gitignored.
#
# Co-located tests/ ARE part of the dereferenced payload — an accepted, documented
# consequence of the symlink design (build-plugin.sh's header); install.sh's
# scrub_source_only is what keeps them out of a per-repo install.
strays="$(find "$REPO_ROOT/plugin" \( -name state.json -o -name warnings.log -o -name '*.verify' -o -name evidence \) 2>/dev/null | wc -l | tr -d ' ')"
assert_eq "vibe-plugin" "generated plugin scaffold carries no runtime state" "$strays" "0"
if git -C "$REPO_ROOT" rev-parse --git-dir >/dev/null 2>&1; then
  tracked_state="$(git -C "$REPO_ROOT" ls-files spec flow 2>/dev/null \
    | grep -Ec '(^|/)(state\.json|warnings\.log|evidence/)' || true)"
  assert_eq "vibe-plugin" "dereferenced payload (spec/ + flow/) commits no runtime state" "$tracked_state" "0"
else
  pass "vibe-plugin" "dereferenced-payload check skipped (not a git checkout)"
fi
# Drift guard: the committed tree must equal a fresh build from spec/ + flow/, so a
# skill edit without a rebuild cannot silently ship a stale plugin.
if [[ -f "$BUILD_PLUGIN" ]] && bash "$BUILD_PLUGIN" --check >/dev/null 2>&1; then
  pass "vibe-plugin" "committed plugin/ matches a fresh build (no drift)"
else
  fail "vibe-plugin" "plugin/ is stale — run ./build-plugin.sh and commit"
fi

echo ""
echo "=== host-support/1 — native host packages: skills + invocation surfaces ==="
# The supported host surface matrix is DATA: flow/reference/host-surfaces.json
# enumerates the four hosts x their documented project skill paths x their one
# user-invoked entry mechanism. Consuming it here is what makes "the supported
# host surface matrix is complete" an assertion about data rather than prose: a
# new or removed host, or a moved skill/command path, must be declared there or
# these legs fail. tech.md's Host surface contract is the authority for each
# pinned path; each host entry records the live doc URL it was verified against.
SURFACES="$REPO_ROOT/flow/reference/host-surfaces.json"
OC_CMD="$REPO_ROOT/.opencode/commands/flow.md"
DROID_CMD="$REPO_ROOT/factory-plugin/commands/flow.md"
CLAUDE_CMD="$REPO_ROOT/.claude/commands/flow.md"
CODEX_PKG="$REPO_ROOT/plugins/vibe"
DROID_PKG="$REPO_ROOT/factory-plugin"

# jq-free legs first: these run on every machine, so this section is never vacuous.
hp_missing=""
for f in "$SURFACES" "$OC_CMD" "$DROID_CMD" \
         "$CODEX_PKG/plugin.json" "$REPO_ROOT/.agents/plugins/marketplace.json" \
         "$DROID_PKG/.factory-plugin/plugin.json" "$REPO_ROOT/.factory-plugin/marketplace.json"; do
  [[ -e "$f" ]] || hp_missing="$hp_missing ${f#"$REPO_ROOT/"}"
done
assert_eq "host-support/1" "every host package file this unit ships exists" "$hp_missing" ""

# One thin wrapper per host that has a distinct command surface, byte-identical
# bodies: the procedure itself stays single-copy in the `vibe` skill, and the
# wrapper only DELEGATES. A copy-paste fork of Claude's step list fails below.
if [[ -f "$OC_CMD" && -f "$DROID_CMD" ]]; then
  cmp -s "$OC_CMD" "$DROID_CMD" \
    && pass "host-support/1" "OpenCode + Factory /flow wrappers are byte-identical (single delegated body)" \
    || fail "host-support/1" "OpenCode + Factory /flow wrappers are byte-identical (single delegated body)"
  hp_wrap="$(cat "$OC_CMD")"
  for tok in ".agents/skills/vibe/SKILL.md" ".agents/skills/vibe/state-machine.json" ".agents/skills/vibe/scripts/set-state.sh"; do
    assert_contains "host-support/1" "host /flow wrapper delegates to the shared $tok" "$hp_wrap" "$tok"
  done
  # Discriminating against a restated procedure: these step labels exist ONLY in
  # Claude's full command file; a wrapper that duplicates the workflow gains them.
  assert_not_contains "host-support/1" "host /flow wrapper does not restate Claude's procedure ('Gate teeth')" "$hp_wrap" "Gate teeth"
  assert_not_contains "host-support/1" "host /flow wrapper does not restate Claude's procedure ('Membership')" "$hp_wrap" "Membership."
else
  fail "host-support/1" "host /flow command files exist for OpenCode and Factory"
fi

# Claude keeps the surfaces it already shipped: the full procedure in its command
# file (asserted by content below, not by byte-compare) and the four hook scripts
# still on disk — this unit adds no hook integration anywhere and changes none.
assert_contains "host-support/1" "Claude /flow command still carries the transition procedure" \
  "$(cat "$CLAUDE_CMD" 2>/dev/null)" "set-state.sh <target> [feature]"
# …and points at the SAME shared artifacts as the host wrappers, so every entry
# point provably reaches one writer/machine pair.
for tok in "state-machine.json" "set-state.sh"; do
  assert_contains "host-support/1" "Claude /flow still uses the shared $tok path" "$(cat "$CLAUDE_CMD" 2>/dev/null)" "$tok"
done
hp_hooks=1
for h in session-start-doctrine user-prompt-submit-inject pre-tool-use-guard stop-gate; do
  [[ -f "$REPO_ROOT/.claude/hooks/$h.sh" ]] || { hp_hooks=0; echo "        missing Claude hook: $h.sh"; }
done
assert_eq "host-support/1" "existing Claude Code hook scripts are intact" "$hp_hooks" "1"

# Codex: skill invocation WITHOUT a fabricated slash command (tech.md + the Codex
# skills doc: repo skills at .agents/skills, invoked via the picker/$-mention).
[[ ! -e "$CODEX_PKG/commands" ]] \
  && pass "host-support/1" "Codex package ships no fabricated slash-command dir" \
  || fail "host-support/1" "Codex package ships no fabricated slash-command dir"
# Open Question 1: the Codex package LINKS the canonical halves instead of
# carrying a plugin-local copy, so a divergent workflow body is impossible.
hp_links=1
for pair in "spec:../../../spec" "vibe:../../../flow"; do
  name="${pair%%:*}"; want="${pair#*:}"
  if [[ -L "$CODEX_PKG/skills/$name" && "$(readlink "$CODEX_PKG/skills/$name")" == "$want" && -f "$CODEX_PKG/skills/$name/SKILL.md" ]]; then
    : # resolves to the canonical SKILL.md
  else
    hp_links=0; echo "        Codex skills/$name is not a symlink -> $want resolving to SKILL.md"
  fi
done
assert_eq "host-support/1" "Codex package links both skills to the canonical halves (no copy)" "$hp_links" "1"
# Factory package: same linked-not-copied shape for its native plugin skills.
hp_dlinks=1
for pair in "spec:../../spec" "vibe:../../flow"; do
  name="${pair%%:*}"; want="${pair#*:}"
  if [[ -L "$DROID_PKG/skills/$name" && "$(readlink "$DROID_PKG/skills/$name")" == "$want" && -f "$DROID_PKG/skills/$name/SKILL.md" ]]; then
    : # resolves to the canonical SKILL.md
  else
    hp_dlinks=0; echo "        Factory skills/$name is not a symlink -> $want resolving to SKILL.md"
  fi
done
assert_eq "host-support/1" "Factory package links both skills to the canonical halves (no copy)" "$hp_dlinks" "1"

# The spec workflow stays skill-invoked on EVERY host — no /spec command exists.
hp_specmd=0
for d in "$REPO_ROOT/.opencode/commands" "$DROID_PKG/commands" "$REPO_ROOT/.claude/commands"; do
  [[ -e "$d/spec.md" ]] && hp_specmd=1
done
assert_eq "host-support/1" "no /spec command file exists on any host" "$hp_specmd" "0"

# Structural negative over the NEW host surfaces: no hook integration and no
# native agent/droid registration ships for OpenCode, Codex, or Factory Droid.
# Population floor first, so an empty find cannot read as a clean pass.
hp_dirs=""
for d in "$REPO_ROOT/.opencode" "$REPO_ROOT/plugins" "$REPO_ROOT/factory-plugin" "$REPO_ROOT/.agents/plugins" "$REPO_ROOT/.factory-plugin"; do
  [[ -d "$d" ]] && hp_dirs="$hp_dirs $d"
done
hp_pop=0
for d in $hp_dirs; do hp_pop=$((hp_pop + 1)); done
assert_eq "host-support/1" "population floor: all five new host surface dirs exist" "$hp_pop" "5"
hp_files=0
for d in $hp_dirs; do hp_files=$((hp_files + $(find "$d" -type f 2>/dev/null | wc -l | tr -d ' '))); done
[[ "$hp_files" -ge 6 ]] \
  && pass "host-support/1" "population floor: new host dirs carry package files (examined $hp_files files)" \
  || fail "host-support/1" "population floor: new host dirs carry package files (examined $hp_files files)"
if [[ "$hp_pop" -eq 5 ]]; then
  hp_badreg=""
  for d in $hp_dirs; do
    m="$(find "$d" \( -path '*/hooks/*' -o -name 'hooks.json' -o -name droids -o -name agents \) -print 2>/dev/null)"
    [[ -n "$m" ]] && hp_badreg="$hp_badreg $m"
  done
  assert_eq "host-support/1" "no hook integration or agent/droid registration in the new host packages" "$hp_badreg" ""
else
  # Never scan a default path when the intended roots are absent: `find` with an
  # empty word-split list searches `.` and would "prove" something about the repo.
  fail "host-support/1" "no hook integration or agent/droid registration in the new host packages (roots absent — not scanned)"
fi

if command -v jq >/dev/null 2>&1; then
  # --- matrix completeness, asserted against the data file.
  jq -e . "$SURFACES" >/dev/null 2>&1 \
    && pass "host-support/1" "host surface matrix is valid JSON" \
    || fail "host-support/1" "host surface matrix is valid JSON"
  assert_eq "host-support/1" "matrix declares the four supported hosts" \
    "$(jq -r '.hosts | length' "$SURFACES" 2>/dev/null)" "4"
  assert_eq "host-support/1" "matrix host ids are exactly claude/opencode/codex/droid" \
    "$(jq -r '.hosts[].id' "$SURFACES" 2>/dev/null | sort | tr '\n' ',')" "claude,codex,droid,opencode,"
  assert_eq "host-support/1" "matrix declares exactly the two shared workflows" \
    "$(jq -r '.workflow.skills | sort | join(",")' "$SURFACES" 2>/dev/null)" "spec,vibe"
  # Every declared skill path resolves to a real SKILL.md, for BOTH workflows —
  # the "each host discovers both shared workflows" scenario, per host, from data.
  # The population floor is data-derived, not a hand-written count: every host
  # declares at least one skill path, and the workflow declares exactly the two
  # shared skills, so fewer than hosts x 2 examined paths means the matrix (or
  # this loop) lost rows — folded into the verdict so an empty iteration can never
  # read as "checked and clean".
  hp_hosts="$(jq -r '.hosts | length' "$SURFACES" 2>/dev/null)"
  [[ "$hp_hosts" =~ ^[0-9]+$ ]] || hp_hosts=0
  hp_skillmiss=""; hp_skilln=0
  while IFS= read -r tpl; do
    for s in spec vibe; do
      hp_skilln=$((hp_skilln + 1))
      p="${tpl//\{name\}/$s}"
      [[ -f "$REPO_ROOT/$p/SKILL.md" ]] || hp_skillmiss="$hp_skillmiss $p"
    done
  done < <(jq -r '.hosts[].skills[]' "$SURFACES" 2>/dev/null)
  [[ "$hp_skilln" -lt $((hp_hosts * 2)) ]] && hp_skillmiss="$hp_skillmiss (floor: $hp_skilln paths examined < $hp_hosts hosts x 2 skills)"
  assert_eq "host-support/1" "every declared host skill path resolves to SKILL.md (floor = hosts x 2)" "$hp_skillmiss" ""
  # The matrix must declare EVERY surface the deploy really creates, not a subset:
  # install deploys plugins/vibe/skills/{spec,vibe} for Codex, so a matrix listing
  # only .agents/skills/{name} was a data/doc/report three-way drift — the report
  # and both READMEs named the package surface, the matrix did not.
  assert_contains "host-support/1" "the matrix's codex skills include the deployed package surface" \
    "$(jq -r '.hosts[] | select(.id == "codex") | .skills | join(",")' "$SURFACES" 2>/dev/null)" \
    "plugins/vibe/skills/{name}"
  # Every declared native command file exists.
  hp_cmdmiss=""; hp_cmdn=0
  while IFS= read -r p; do
    [[ -z "$p" ]] && continue
    hp_cmdn=$((hp_cmdn + 1))
    [[ -f "$REPO_ROOT/$p" ]] || hp_cmdmiss="$hp_cmdmiss $p"
  done < <(jq -r '.hosts[].entry | select(.kind == "command") | .path' "$SURFACES" 2>/dev/null)
  [[ "$hp_cmdn" -ne 3 ]] && hp_cmdmiss="$hp_cmdmiss (floor: $hp_cmdn command-surface hosts examined, want 3)"
  assert_eq "host-support/1" "every declared native command file exists (floor 3)" "$hp_cmdmiss" ""
  # Codex invokes the skill, never a synthetic slash command.
  assert_eq "host-support/1" "Codex entry mechanism is skill invocation" \
    "$(jq -r '.hosts[] | select(.id == "codex") | .entry.kind' "$SURFACES" 2>/dev/null)" "skill"
  assert_eq "host-support/1" "no Codex command path is declared" \
    "$(jq -r '[.hosts[] | select(.id == "codex") | .entry.path | select(. != null)] | length' "$SURFACES" 2>/dev/null)" "0"
  assert_eq "host-support/1" "Codex invocation is a skill token, not a slash command" \
    "$(jq -r '.hosts[] | select(.id == "codex") | .entry.invocation' "$SURFACES" 2>/dev/null | cut -c1)" "\$"
  # Data-driven "each host exposes the documented invocation mechanism": a command
  # surface must be a slash command, a skill surface must be a skill token, and any
  # other kind is unknown-and-therefore-failed (no silently new mechanism).
  hp_invbad=""; hp_invn=0
  while IFS=$'\t' read -r kind inv; do
    [[ -z "$kind" ]] && continue
    hp_invn=$((hp_invn + 1))
    case "$kind" in
      command) [[ "$inv" == /* ]] || hp_invbad="$hp_invbad command:$inv" ;;
      skill)   [[ "$inv" == \$* ]] || hp_invbad="$hp_invbad skill:$inv" ;;
      *)       hp_invbad="$hp_invbad unknown-kind:$kind" ;;
    esac
  done < <(jq -r '.hosts[] | [.entry.kind, .entry.invocation] | @tsv' "$SURFACES" 2>/dev/null)
  [[ "$hp_invn" -ne 4 ]] && hp_invbad="$hp_invbad (floor: $hp_invn entries examined, want 4)"
  assert_eq "host-support/1" "every host declares a matching invocation mechanism (command=/x, skill=\$x)" "$hp_invbad" ""
  # Package manifests + marketplaces: valid JSON, named plugin, source resolves.
  hp_pkgn=0
  while IFS= read -r m; do
    [[ -z "$m" ]] && continue
    hp_pkgn=$((hp_pkgn + 1))
    jq -e '.name == "vibe"' "$REPO_ROOT/$m" >/dev/null 2>&1 \
      && pass "host-support/1" "package manifest $m is valid JSON naming the vibe plugin" \
      || fail "host-support/1" "package manifest $m is valid JSON naming the vibe plugin"
  done < <(jq -r '.hosts[] | .package?.manifest // empty' "$SURFACES" 2>/dev/null)
  assert_eq "host-support/1" "exactly the three plugin-carrier hosts declare a manifest" "$hp_pkgn" "3"
  # Codex's manifest is the portable Agent Plugins manifest (tech.md), not an overlay.
  jq -e 'has("$schema")' "$CODEX_PKG/plugin.json" >/dev/null 2>&1 \
    && pass "host-support/1" "Codex package uses the portable Agent Plugins manifest" \
    || fail "host-support/1" "Codex package uses the portable Agent Plugins manifest"
  hp_mktmiss=""; hp_mktn=0
  while IFS= read -r m; do
    [[ -z "$m" ]] && continue
    hp_mktn=$((hp_mktn + 1))
    if ! jq -e . "$REPO_ROOT/$m" >/dev/null 2>&1; then
      hp_mktmiss="$hp_mktmiss $m(invalid-json)"; continue
    fi
    src="$(jq -r '.plugins[0].source | if type == "object" then .path else . end' "$REPO_ROOT/$m" 2>/dev/null)"
    [[ -n "$src" && -e "$REPO_ROOT/${src#./}" ]] || hp_mktmiss="$hp_mktmiss $m($src)"
  done < <(jq -r '.hosts[] | .package?.marketplace // empty' "$SURFACES" 2>/dev/null)
  [[ "$hp_mktn" -ne 3 ]] && hp_mktmiss="$hp_mktmiss (floor: $hp_mktn marketplaces examined, want 3)"
  assert_eq "host-support/1" "every marketplace lists a plugin whose source path resolves (floor 3)" "$hp_mktmiss" ""
  # No hook/agent surface is declared anywhere in the matrix.
  assert_eq "host-support/1" "the matrix declares no hook or agent/droid surface" \
    "$(jq -r '[.hosts[] | paths | map(tostring) | join(".")] | map(select(test("hook|subagent|droids"))) | length' "$SURFACES" 2>/dev/null)" "0"
  # Population floor for the negative above: the matrix really carries the
  # surface keys it claims to have checked.
  assert_eq "host-support/1" "population floor: each host declares skills + entry" \
    "$(jq -r '[.hosts[] | select((.skills | length) > 0 and (.entry.kind | length) > 0)] | length' "$SURFACES" 2>/dev/null)" "4"
else
  echo "  SKIP [host-support/1] host surface matrix JSON legs (jq not on PATH)"
fi

echo ""
echo "=== host-support/2 — installer lifecycle: selection, roots, manifest, removal ==="
# The unit's contract in five parts: (1) --platform grammar, (2) hermetic
# detection + explicit override, (3) root resolution from nested repo / worktree /
# plain dirs, (4) the install manifest as the ownership floor, (5) dry-run,
# idempotent re-install, surgical uninstall, and the capability report.
HOST_DIRS=".claude .opencode plugins factory-plugin .factory-plugin"

# --- (1) --platform grammar -------------------------------------------------
SBpg="$(mktmp)"
pout="$(bash "$INSTALL" "$SBpg" --platform bogus 2>&1; echo "rc=$?")"
assert_contains "host-support/2" "--platform rejects an unknown host token (usage error)" "$pout" "rc=1"
for v in claude opencode codex droid none; do
  assert_contains "host-support/2" "--platform usage error names the valid value '$v'" "$pout" "$v"
done
pout="$(bash "$INSTALL" "$SBpg" --platform "" 2>&1; echo "rc=$?")"
assert_contains "host-support/2" "--platform with an empty value is a usage error" "$pout" "rc=1"
# …and the `--platform=` spelling must be rejected the same way. `IFS=',' read -a`
# on an empty string iterates ZERO tokens, so the `--platform=` form (or
# --platform="$VAR" with VAR unset) used to be accepted as an empty selection —
# silently degrading an unset variable to "host-neutral" instead of failing.
pout="$(bash "$INSTALL" "$SBpg" --platform= 2>&1; echo "rc=$?")"
assert_contains "host-support/2" "--platform= (empty --platform= form) is a usage error" "$pout" "rc=1"
assert_contains "host-support/2" "--platform= names the valid values, like --platform ''" "$pout" "none"
pout="$(bash "$INSTALL" "$SBpg" --platform claude,none 2>&1; echo "rc=$?")"
assert_contains "host-support/2" "--platform none cannot be combined with a host" "$pout" "rc=1"
[[ ! -e "$SBpg/.agents" && ! -e "$SBpg/.claude" && ! -e "$SBpg/.vibe" ]] \
  && pass "host-support/2" "a rejected --platform writes nothing" \
  || fail "host-support/2" "a rejected --platform wrote files"
rm -rf "$SBpg"
# --help prints the leading comment block, so the new selector must be documented
# there with its valid values.
helpout="$(bash "$INSTALL" --help 2>&1)"
assert_contains "host-support/2" "--help documents --platform" "$helpout" "--platform LIST"
assert_contains "host-support/2" "--help names the --platform valid values" "$helpout" "claude, opencode, codex, droid, none"

# --- (2) detection is hermetic, presence-only, and overridable --------------
# mkshim's toolset contains NONE of the four host CLIs, so a bare mkshim PATH is
# an inherent no-detect farm; mkhosts adds stubs for the named hosts. Neither
# reads the ambient PATH, which is what makes these legs evidence on any runner.
FARM_NONE="$(mkshim)"
FARM_ALL="$(mkhosts claude opencode codex droid)"
SBno="$(mktmp)"
outno="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBno" 2>&1)"
nn=1
for p in .agents/skills/spec/SKILL.md .agents/skills/vibe/SKILL.md AGENTS.md .gitignore .agents/skills/vibe/state.json; do
  [[ -e "$SBno/$p" ]] || { nn=0; echo "        missing host-neutral $p"; }
done
assert_eq "host-support/2" "no host CLI detected: the host-neutral core still installs" "$nn" "1"
nohost=1
for p in $HOST_DIRS; do
  [[ -e "$SBno/$p" ]] && { nohost=0; echo "        configured $p with no host CLI on PATH"; }
done
assert_eq "host-support/2" "no host CLI detected: no native host surface is configured" "$nohost" "1"
assert_contains "host-support/2" "no host CLI detected: the summary says so" "$outno" "no supported host CLI detected"
assert_contains "host-support/2" "no host CLI detected: the summary prints the --platform selection syntax" "$outno" "--platform claude,codex"
rm -rf "$SBno"

# Only the CLIs present are configured, and nothing else is.
FARM_CO="$(mkhosts codex opencode)"
SBco="$(mktmp)"
outco="$(PATH="$FARM_CO" "$FARM_CO/bash" "$INSTALL" "$SBco" 2>&1)"
det=1
[[ -e "$SBco/.opencode/commands/flow.md" ]] || { det=0; echo "        opencode detected but not configured"; }
[[ -e "$SBco/plugins/vibe/plugin.json" ]] || { det=0; echo "        codex detected but not configured"; }
[[ -e "$SBco/.agents/plugins/marketplace.json" ]] || { det=0; echo "        codex marketplace not configured"; }
assert_eq "host-support/2" "detection configures the detected hosts (opencode + codex)" "$det" "1"
det_only=1
for p in .claude factory-plugin .factory-plugin; do
  [[ -e "$SBco/$p" ]] && { det_only=0; echo "        configured undetected $p"; }
done
assert_eq "host-support/2" "detection configures ONLY the detected hosts" "$det_only" "1"
assert_contains "host-support/2" "the summary names the detected host opencode" "$outco" "opencode"
assert_contains "host-support/2" "the summary names the detected host codex" "$outco" "codex"
assert_not_contains "host-support/2" "the summary names no undetected host surface" "$outco" ".claude/commands/flow.md"
assert_not_contains "host-support/2" "a non-Claude install claims no hook support (R5)" "$outco" "hook"
rm -rf "$SBco"

# An explicit --platform overrides detection entirely: with every CLI present…
SBsel="$(mktmp)"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBsel" --platform codex >/dev/null 2>&1
ex=1
[[ -e "$SBsel/plugins/vibe/plugin.json" ]] || { ex=0; echo "        codex not configured"; }
for p in .claude .opencode factory-plugin .factory-plugin; do
  [[ -e "$SBsel/$p" ]] && { ex=0; echo "        configured $p despite --platform codex"; }
done
assert_eq "host-support/2" "explicit --platform wins with every host CLI present" "$ex" "1"
# …and with NO CLI present.
SBabs="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBabs" --platform droid >/dev/null 2>&1
ab=1
[[ -e "$SBabs/factory-plugin/.factory-plugin/plugin.json" ]] || { ab=0; echo "        droid not configured"; }
[[ -e "$SBabs/.factory-plugin/marketplace.json" ]] || { ab=0; echo "        droid marketplace not configured"; }
for p in .claude .opencode plugins; do
  [[ -e "$SBabs/$p" ]] && { ab=0; echo "        configured $p despite --platform droid"; }
done
assert_eq "host-support/2" "explicit --platform wins with no host CLI present" "$ab" "1"
# `none` = host-neutral only, even with all four CLIs present.
SBnon="$(mktmp)"
outnon="$(PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBnon" --platform none 2>&1)"
non=1
for p in $HOST_DIRS; do
  [[ -e "$SBnon/$p" ]] && { non=0; echo "        --platform none configured $p"; }
done
assert_eq "host-support/2" "--platform none configures no host surface with all CLIs present" "$non" "1"
[[ -e "$SBnon/.agents/skills/vibe/SKILL.md" ]] \
  && pass "host-support/2" "--platform none still installs the host-neutral core" \
  || fail "host-support/2" "--platform none still installs the host-neutral core"
assert_not_contains "host-support/2" "--platform none claims no hooks" "$outnon" "hook"
# last-wins, like --only.
SBwin="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBwin" --platform claude --platform opencode >/dev/null 2>&1
{ [[ -e "$SBwin/.opencode/commands/flow.md" && ! -e "$SBwin/.claude" ]]; } \
  && pass "host-support/2" "--platform is last-wins (claude then opencode -> opencode only)" \
  || fail "host-support/2" "--platform is last-wins (claude then opencode -> opencode only)"
rm -rf "$SBsel" "$SBabs" "$SBnon" "$SBwin"

# --- (3) root resolution ----------------------------------------------------
SBroot="$(mktmp)"; git -C "$SBroot" init -q >/dev/null 2>&1; mkdir -p "$SBroot/pkg/src"
rout="$( cd "$SBroot/pkg/src" && bash "$INSTALL" --dry-run --local --platform claude 2>&1 )"
assert_contains "host-support/2" "a nested repository path resolves to the repository root" "$rout" "$SBroot/.agents/skills"
assert_not_contains "host-support/2" "a nested repository path does not target the subdirectory" "$rout" "pkg/src/.agents"
( cd "$SBroot/pkg/src" && bash "$INSTALL" --platform codex >/dev/null 2>&1 )
{ [[ -e "$SBroot/plugins/vibe/plugin.json" && ! -e "$SBroot/pkg/src/plugins" ]]; } \
  && pass "host-support/2" "a real install from a nested path lands host surfaces at the repository root" \
  || fail "host-support/2" "a real install from a nested path lands host surfaces at the repository root"
rm -rf "$SBroot"
# A worktree: `.git` is a FILE there (the `-e` marker catches it), so the search
# must not walk past it to the main repository.
WT="$(mktmp)"; git -C "$WT" init -q >/dev/null 2>&1
printf 'seed\n' > "$WT/seed.txt"
git -C "$WT" add -A >/dev/null 2>&1
git -C "$WT" -c user.email=t@t -c user.name=t commit -qm seed >/dev/null 2>&1
WT2="$(mktmp)"; rmdir "$WT2" 2>/dev/null || true
if git -C "$WT" worktree add -q "$WT2" -b vibe-wt >/dev/null 2>&1; then
  [[ -f "$WT2/.git" && ! -d "$WT2/.git" ]] \
    && pass "host-support/2" "worktree fixture: .git is a FILE there (population floor)" \
    || fail "host-support/2" "worktree fixture: .git is a FILE there (population floor)"
  mkdir -p "$WT2/deep/nested"
  wout="$( cd "$WT2/deep/nested" && bash "$INSTALL" --dry-run --local --platform claude 2>&1 )"
  assert_contains "host-support/2" "a nested worktree directory resolves to the worktree root" "$wout" "$WT2/.agents/skills"
  assert_not_contains "host-support/2" "a nested worktree directory does not resolve to the main repo" "$wout" "$WT/.agents/skills"
else
  fail "host-support/2" "worktree fixture: git worktree add"
fi
rm -rf "$WT" "$WT2"
# A plain workspace: no Git ancestor, so the current directory IS the root. The
# fixture asserts its own cleanliness, so an unsuitable TMPDIR fails loudly
# instead of silently passing for the wrong reason.
SBpl="$(mktmp)"; mkdir -p "$SBpl/ws"
anc=0; _pd="$SBpl/ws"
while [[ "$_pd" != "/" ]]; do
  [[ -d "$_pd/.spec" || -e "$_pd/.git" ]] && anc=1
  _pd="$(dirname "$_pd")"
done
assert_eq "host-support/2" "plain-workspace fixture: no Git/.spec ancestor (population floor)" "$anc" "0"
plout="$( cd "$SBpl/ws" && bash "$INSTALL" --dry-run --local --platform none 2>&1 )"
assert_contains "host-support/2" "a plain workspace with no Git ancestor targets the current directory" "$plout" "$SBpl/ws/.agents/skills"
( cd "$SBpl/ws" && bash "$INSTALL" --platform none >/dev/null 2>&1 )
[[ -e "$SBpl/ws/AGENTS.md" ]] \
  && pass "host-support/2" "a real install in a plain workspace writes into the current directory" \
  || fail "host-support/2" "a real install in a plain workspace writes into the current directory"
rm -rf "$SBpl"

# --- (4) the install manifest (ownership floor) -----------------------------
SBm="$(mktmp)"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBm" >/dev/null 2>&1
MAN="$SBm/.vibe/install-manifest.json"
[[ -f "$MAN" ]] && pass "host-support/2" "install writes an install manifest" || fail "host-support/2" "install writes an install manifest"
if command -v jq >/dev/null 2>&1; then
  jq -e . "$MAN" >/dev/null 2>&1 && pass "host-support/2" "the install manifest is valid JSON" || fail "host-support/2" "the install manifest is valid JSON"
else
  grep -qF '"actions": [' "$MAN" && pass "host-support/2" "the install manifest carries an actions array (no jq)" || fail "host-support/2" "the install manifest carries an actions array (no jq)"
fi
grep -qF 'install-manifest.json' "$SBm/.gitignore" 2>/dev/null \
  && fail "host-support/2" "the install manifest must NOT be gitignored (a fresh clone must still uninstall)" \
  || pass "host-support/2" "the install manifest is not gitignored (a fresh clone must still uninstall)"
# FLOOR — asserted BEFORE any uninstall leg runs, and read with sed (not jq) so it
# holds on a jq-less runner too: at least one managed action per selected host.
man_n() { sed -n "/^ *{\"host\": \"$1\"/p" "$MAN" | wc -l | tr -d ' '; }
for h in claude opencode codex droid; do
  n="$(man_n "$h")"
  [[ "$n" -ge 1 ]] && pass "host-support/2" "manifest floor: >=1 managed action for $h (got $n)" \
    || fail "host-support/2" "manifest floor: >=1 managed action for $h (got $n)"
done
# …and the manifest reflects the SELECTION, not "everything that could exist".
SBoc="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBoc" --platform opencode >/dev/null 2>&1
MAN="$SBoc/.vibe/install-manifest.json"
assert_eq "host-support/2" "the manifest records nothing for an unselected host" \
  "$(man_n claude)$(man_n codex)$(man_n droid)" "000"
assert_eq "host-support/2" "the manifest records the selected host" \
  "$([[ "$(man_n opencode)" -ge 1 ]] && echo yes || echo no)" "yes"
rm -rf "$SBoc"
MAN="$SBm/.vibe/install-manifest.json"
man_before="$(cat "$MAN")"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBm" >/dev/null 2>&1
assert_eq "host-support/2" "a re-install leaves the manifest byte-identical (preserved, not re-stamped)" \
  "$(cat "$MAN")" "$man_before"

# --- (5) dry-run, re-install, uninstall, capability report ------------------
SBd="$(mktmp)"; before="$(tree_fp "$SBd")"
dout="$(PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBd" --dry-run 2>&1; echo "rc=$?")"
after="$(tree_fp "$SBd")"
assert_contains "host-support/2" "dry-run exits 0" "$dout" "rc=0"
for want in ".opencode/commands/flow.md" "plugins/vibe" ".agents/plugins/marketplace.json" \
            "factory-plugin/commands/flow.md" ".factory-plugin/marketplace.json" ".vibe/install-manifest.json"; do
  assert_contains "host-support/2" "dry-run reports the planned host path $want" "$dout" "$want"
done
assert_eq "host-support/2" "dry-run on a fresh target writes nothing (host surfaces included)" "$before" "$after"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBd" >/dev/null 2>&1
before="$(tree_fp "$SBd")"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBd" --dry-run >/dev/null 2>&1
after="$(tree_fp "$SBd")"
assert_eq "host-support/2" "dry-run on an installed target is byte-identical" "$before" "$after"
# …and composes with an explicit selection: the plan names exactly that host's
# surfaces and nothing else.
SBdc="$(mktmp)"
dcout="$(PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBdc" --dry-run --platform codex 2>&1)"
assert_contains "host-support/2" "dry-run composes with --platform (codex plan names its package file)" "$dcout" "plugins/vibe/plugin.json"
assert_not_contains "host-support/2" "the codex dry-run plan excludes another host's surface" "$dcout" ".opencode/commands/flow.md"
[[ ! -e "$SBdc/plugins" && ! -e "$SBdc/.opencode" ]] \
  && pass "host-support/2" "the composed dry-run created nothing" \
  || fail "host-support/2" "the composed dry-run created files"
rm -rf "$SBdc"
# --dry-run must plan what a REAL run would do. With a colliding user entry the
# real run refuses to overwrite (WARN + skip), but deploy_file returned from its
# dry-run branch BEFORE the collision checks, so the plan claimed a deploy that
# the real run refuses — a plan that lies about the one case the user cares about.
SBc6="$(mktmp)"
mkdir -p "$SBc6/.opencode/commands"
printf 'USER flow command, not vibe\n' > "$SBc6/.opencode/commands/flow.md"
before6="$(tree_fp "$SBc6")"
c6out="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBc6" --dry-run --platform opencode 2>&1)"
assert_contains "host-support/2" "dry-run plans the colliding entry as skipped, not deployed" \
  "$c6out" "skip .opencode/commands/flow.md"
assert_not_contains "host-support/2" "dry-run does not claim to deploy the colliding entry" \
  "$c6out" "deploy .opencode/commands/flow.md"
assert_eq "host-support/2" "dry-run with a colliding entry writes nothing" "$before6" "$(tree_fp "$SBc6")"
# …and the real run does exactly what the plan foretold: WARN, leave the user's
# file byte-identical, install the rest of the package.
c6real="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBc6" --platform opencode 2>&1)"
assert_contains "host-support/2" "the real run reports the collision the plan predicted" "$c6real" "move it aside and re-run"
grep -qF 'USER flow command, not vibe' "$SBc6/.opencode/commands/flow.md" \
  && pass "host-support/2" "the collided entry survives the real run byte-for-byte" \
  || fail "host-support/2" "the collided entry was overwritten by the real run"
rm -rf "$SBc6"
# …and the same for a user-owned slot inside a host PACKAGE (the link loop).
SBl6="$(mktmp)"; mkdir -p "$SBl6/plugins/vibe/skills"
ln -s ../../../my-own-spec "$SBl6/plugins/vibe/skills/spec"
l6out="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBl6" --dry-run --platform codex 2>&1)"
assert_contains "host-support/2" "dry-run plans a user-retargeted package link as skipped" \
  "$l6out" "skip plugins/vibe/skills/spec"
assert_not_contains "host-support/2" "dry-run does not claim to create the user-retargeted package link" \
  "$l6out" "link plugins/vibe/skills/spec"
rm -rf "$SBl6"
# --only spec is a PURE spec install: host entry points are flow-half carriers, so
# none is deployed — which is also what stops a dangling skill link to a skill
# that is not installed. Control: the same selection with both halves does deploy.
SBos="$(mktmp)"
osout="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBos" --only spec --platform codex 2>&1)"
[[ ! -e "$SBos/plugins" ]] \
  && pass "host-support/2" "--only spec installs no host entry point (no dangling skill link)" \
  || fail "host-support/2" "--only spec installed a host entry point"
assert_contains "host-support/2" "--only spec reports the skipped host surface" "$osout" "host surface 'codex' skipped"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBos" --platform codex >/dev/null 2>&1
[[ -e "$SBos/plugins/vibe/plugin.json" ]] \
  && pass "host-support/2" "control: the same selection with both halves deploys the host package" \
  || fail "host-support/2" "control: the host package was not deployed with both halves"
rm -rf "$SBos"
# --only flow is the MIRROR of that case: the spec half is absent, so a package
# link to .agents/skills/spec resolves to NOTHING. A dangling link is worse than an
# absent one — it advertises a skill the target does not have, which is exactly the
# property the --only spec leg above pins from the other side.
for h in codex droid; do
  case "$h" in codex) hpkg=plugins/vibe ;; droid) hpkg=factory-plugin ;; esac
  SBfo1="$(mktmp)"
  fo1out="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBfo1" --only flow --platform "$h" 2>&1)"
  [[ ! -e "$SBfo1/.agents/skills/spec" && -f "$SBfo1/.agents/skills/vibe/SKILL.md" ]] \
    && pass "host-support/2" "only-flow fixture ($h): the spec half is absent and vibe is present (population floor)" \
    || fail "host-support/2" "only-flow fixture ($h): the spec half is absent and vibe is present (population floor)"
  [[ ! -e "$SBfo1/$hpkg/skills/spec" && ! -L "$SBfo1/$hpkg/skills/spec" ]] \
    && pass "host-support/2" "--only flow --platform $h deploys no skills/spec link (no dangling link)" \
    || fail "host-support/2" "--only flow --platform $h deployed a skills/spec link"
  # Structural negative over the WHOLE deployed package, with a floor so an empty
  # find cannot read as a clean pass: every link that IS there must resolve.
  fo1dangle=""; fo1n=0
  while IFS= read -r l; do
    fo1n=$((fo1n + 1))
    [[ -e "$l" ]] || fo1dangle="$fo1dangle ${l#"$SBfo1/"}"
  done < <(find "$SBfo1/$hpkg" -type l 2>/dev/null)
  [[ "$fo1n" -ge 1 ]] || fo1dangle="$fo1dangle (floor: $fo1n links examined under $hpkg, want >=1)"
  assert_eq "host-support/2" "no dangling link anywhere under the --only flow $h package (examined $fo1n)" "$fo1dangle" ""
  { [[ -L "$SBfo1/$hpkg/skills/vibe" && -f "$SBfo1/$hpkg/skills/vibe/SKILL.md" ]]; } \
    && pass "host-support/2" "--only flow --platform $h still deploys the vibe link, and it resolves" \
    || fail "host-support/2" "--only flow --platform $h did not deploy a resolving vibe link"
  assert_contains "host-support/2" "--only flow --platform $h warns which skill link it skipped" "$fo1out" "$hpkg/skills/spec"
  assert_contains "host-support/2" "--only flow --platform $h says why (the spec half is not installed)" "$fo1out" "spec skill is not installed"
  # …and a later install WITH the missing half ADDS the skipped link: the slot was
  # left free (not occupied by a stub), so deploy_package creates it and records it.
  PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBfo1" --platform "$h" >/dev/null 2>&1
  { [[ -L "$SBfo1/$hpkg/skills/spec" && -f "$SBfo1/$hpkg/skills/spec/SKILL.md" ]]; } \
    && pass "host-support/2" "a later full install adds the previously skipped $h skills/spec link (resolves)" \
    || fail "host-support/2" "a later full install did not add the previously skipped $h skills/spec link"
  grep -qF "$hpkg/skills/spec" "$SBfo1/.vibe/install-manifest.json" 2>/dev/null \
    && pass "host-support/2" "…and records it as vibe-owned, so uninstall can still remove it" \
    || fail "host-support/2" "…and records it as vibe-owned, so uninstall can still remove it"
  rm -rf "$SBfo1"
done
# Control: the same selections with BOTH halves deploy both links, and both resolve.
for h in codex droid; do
  case "$h" in codex) hpkg=plugins/vibe ;; droid) hpkg=factory-plugin ;; esac
  SBfo2="$(mktmp)"
  PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBfo2" --platform "$h" >/dev/null 2>&1
  { [[ -f "$SBfo2/$hpkg/skills/spec/SKILL.md" && -f "$SBfo2/$hpkg/skills/vibe/SKILL.md" ]]; } \
    && pass "host-support/2" "control: both halves deploy both $h skill links, both resolving" \
    || fail "host-support/2" "control: both halves did not deploy both resolving $h skill links"
  rm -rf "$SBfo2"
done
# The dry-run plan mirrors it: plan the link only where it would resolve, and name
# the skipped one — a plan that promises a dangling link is the same lie as a plan
# that promises a deploy the real run refuses.
SBfo3="$(mktmp)"
fo3out="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBfo3" --only flow --platform codex --dry-run 2>&1)"
assert_contains "host-support/2" "the --only flow dry-run plans the vibe link" "$fo3out" "link plugins/vibe/skills/vibe"
assert_not_contains "host-support/2" "the --only flow dry-run does not plan the dangling spec link" "$fo3out" "link plugins/vibe/skills/spec"
assert_contains "host-support/2" "the --only flow dry-run names the skipped spec link" "$fo3out" "skip plugins/vibe/skills/spec"
rm -rf "$SBfo3"
# The skip is keyed on whether the link would RESOLVE, not on the selection flag
# alone: a target that already carries the spec half (a previous --only spec
# install) gets a real, resolving link even under --only flow. This is the control
# that fails if the guard is narrowed to `WANT_SPEC -eq 1` and starts skipping a
# link that would have been perfectly good.
SBfo4="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBfo4" --only spec --platform none >/dev/null 2>&1
[[ -f "$SBfo4/.agents/skills/spec/SKILL.md" ]] \
  && pass "host-support/2" "pre-existing-half fixture: the spec skill really is there first (population floor)" \
  || fail "host-support/2" "pre-existing-half fixture: the spec skill really is there first (population floor)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBfo4" --only flow --platform codex >/dev/null 2>&1
{ [[ -f "$SBfo4/plugins/vibe/skills/spec/SKILL.md" && -f "$SBfo4/plugins/vibe/skills/vibe/SKILL.md" ]]; } \
  && pass "host-support/2" "a target already carrying the spec half gets a resolving codex spec link under --only flow" \
  || fail "host-support/2" "a target already carrying the spec half gets a resolving codex spec link under --only flow"
rm -rf "$SBfo4"
# --adapters is orthogonal to host selection: those symlinks point at AGENTS.md,
# a host-neutral artifact, so they must work under `none` too.
SBad="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBad" --platform none --adapters claude >/dev/null 2>&1
{ [[ -L "$SBad/CLAUDE.md" && "$(readlink "$SBad/CLAUDE.md")" == "AGENTS.md" && ! -e "$SBad/.claude" ]]; } \
  && pass "host-support/2" "--adapters stays orthogonal to --platform (host-neutral AGENTS.md link)" \
  || fail "host-support/2" "--adapters is not orthogonal to --platform"
rm -rf "$SBad"
# Re-install: byte-stable managed files, no duplicated registrations.
before="$(tree_fp "$SBd")"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBd" >/dev/null 2>&1
after="$(tree_fp "$SBd")"
assert_eq "host-support/2" "re-install is byte-stable across every managed host file" "$before" "$after"
# The package skill links must RESOLVE in a real target — the whole point of the
# rewrite: the source's ../../../spec target does not exist in a target, so a
# verbatim copy would dangle.
links=1
for pair in "plugins/vibe/skills/spec:../../../.agents/skills/spec" \
            "plugins/vibe/skills/vibe:../../../.agents/skills/vibe" \
            "factory-plugin/skills/spec:../../.agents/skills/spec" \
            "factory-plugin/skills/vibe:../../.agents/skills/vibe"; do
  lp="${pair%%:*}"; want="${pair#*:}"
  if [[ -L "$SBd/$lp" && "$(readlink "$SBd/$lp")" == "$want" && -f "$SBd/$lp/SKILL.md" ]]; then
    : # rewritten AND resolves
  else
    links=0; echo "        $lp is not a link to $want resolving to SKILL.md"
  fi
done
assert_eq "host-support/2" "every deployed host skill link is rewritten AND resolves in the target" "$links" "1"
dupn=0
for d in plugins/vibe/skills factory-plugin/skills; do
  dupn=$((dupn + $(find "$SBd/$d" -type l | wc -l | tr -d ' ')))
done
assert_eq "host-support/2" "re-install does not duplicate the host skill registrations" "$dupn" "4"
# Uninstall: user files in EVERY new shared dir, then a full removal. The
# co-located files are what make "remove only what we created" discriminating on
# BOTH removal paths (the manifest pass here, and the shipped-source fallback in
# the legacy leg below).
printf 'mine\n' > "$SBd/.opencode/commands/mine.md"
mkdir -p "$SBd/plugins/other"; printf 'mine\n' > "$SBd/plugins/other/plugin.json"
printf 'mine\n' > "$SBd/plugins/vibe/mine.txt"
printf 'mine\n' > "$SBd/factory-plugin/mine.txt"
printf 'mine\n' > "$SBd/.factory-plugin/mine.json"
printf 'mine\n' > "$SBd/.agents/plugins/mine.json"
printf 'mine\n' > "$SBd/.claude/commands/mine.md"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBd" --uninstall --yes >/dev/null 2>&1
gone=1
for p in .opencode/commands/flow.md plugins/vibe/plugin.json .agents/plugins/marketplace.json \
         factory-plugin/commands/flow.md factory-plugin/.factory-plugin/plugin.json \
         .factory-plugin/marketplace.json .claude/commands/flow.md .claude/hooks/stop-gate.sh \
         .claude/skills/spec .claude/skills/vibe .vibe/install-manifest.json; do
  if [[ -e "$SBd/$p" || -L "$SBd/$p" ]]; then gone=0; echo "        survived uninstall: $p"; fi
done
assert_eq "host-support/2" "uninstall removes the recorded vibe artifacts of every host" "$gone" "1"
kept=1
for p in .opencode/commands/mine.md plugins/other/plugin.json plugins/vibe/mine.txt \
         factory-plugin/mine.txt .factory-plugin/mine.json .agents/plugins/mine.json \
         .claude/commands/mine.md; do
  [[ -f "$SBd/$p" ]] || { kept=0; echo "        lost user file: $p"; }
done
assert_eq "host-support/2" "uninstall preserves user files in every new shared dir byte-for-byte" "$kept" "1"
[[ ! -e "$SBd/plugins/vibe/skills" ]] \
  && pass "host-support/2" "uninstall prunes a host package subdir it emptied" \
  || fail "host-support/2" "uninstall left an emptied host package subdir behind"
rm -rf "$SBd"
# --uninstall --dry-run must show the EXACT project paths (design.md says the
# dry-run shows the exact project paths), not one summary line per pass: the
# manifest pass and the shipped-source fallback each name every path they would
# remove — and still write nothing.
SBd5="$(mktmp)"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBd5" >/dev/null 2>&1
d5before="$(tree_fp "$SBd5")"
d5out="$(PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBd5" --uninstall --dry-run 2>&1)"
assert_eq "host-support/2" "--uninstall --dry-run writes nothing" "$d5before" "$(tree_fp "$SBd5")"
for p in .opencode/commands/flow.md plugins/vibe/plugin.json plugins/vibe/skills/spec \
         .agents/plugins/marketplace.json factory-plugin/commands/flow.md .factory-plugin/marketplace.json; do
  assert_contains "host-support/2" "--uninstall --dry-run names the concrete manifest path $p" "$d5out" "$p"
done
# …and the same concreteness on the shipped-source fallback (no usable manifest).
rm -f "$SBd5/.vibe/install-manifest.json"
d5before="$(tree_fp "$SBd5")"
d5out2="$(PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBd5" --uninstall --dry-run 2>&1)"
assert_contains "host-support/2" "the dry-run fallback reports the shipped-source inverse" "$d5out2" "falling back to the shipped-source inverse"
assert_eq "host-support/2" "--uninstall --dry-run (legacy fallback) writes nothing" "$d5before" "$(tree_fp "$SBd5")"
for p in .opencode/commands/flow.md plugins/vibe/plugin.json factory-plugin/commands/flow.md; do
  assert_contains "host-support/2" "--uninstall --dry-run fallback names the concrete path $p" "$d5out2" "$p"
done
rm -rf "$SBd5"
# --uninstall --only spec removes the spec half and LEAVES the manifest (the flow
# half's host records must stay, or a later full uninstall could not remove those
# artifacts). The records of the paths THIS pass deleted must go with it: a record
# naming a path that no longer exists is a false ownership claim.
SBp7="$(mktmp)"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBp7" >/dev/null 2>&1
MAN="$SBp7/.vibe/install-manifest.json"
grep -qF '.claude/skills/spec' "$MAN" \
  && pass "host-support/2" "only-spec-prune fixture: the spec registration record is there first (population floor)" \
  || fail "host-support/2" "only-spec-prune fixture: the spec registration record is there first (population floor)"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBp7" --uninstall --only spec --yes >/dev/null 2>&1
[[ ! -e "$SBp7/.agents/skills/spec" ]] \
  && pass "host-support/2" "only-spec-prune fixture: the spec half really went (population floor)" \
  || fail "host-support/2" "only-spec-prune fixture: the spec half really went (population floor)"
assert_eq "host-support/2" "an --only spec uninstall prunes the records of the paths it removed" \
  "$(grep -c '\.claude/skills/spec' "$MAN" 2>/dev/null || true)" "0"
assert_eq "host-support/2" "…and keeps the surviving flow-half records (still removable later)" \
  "$([[ "$(man_n opencode)" -ge 1 && "$(man_n codex)" -ge 1 ]] && echo yes || echo no)" "yes"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBp7" --uninstall --yes >/dev/null 2>&1
left7=""
for p in .opencode/commands/flow.md plugins/vibe/plugin.json .factory-plugin/marketplace.json \
         .claude/commands/flow.md .claude/skills/vibe .claude/skills/spec; do
  [[ -e "$SBp7/$p" || -L "$SBp7/$p" ]] && left7="$left7 $p"
done
assert_eq "host-support/2" "a later full uninstall after an --only spec pass leaves nothing behind" "$left7" ""
rm -rf "$SBp7"
# The manifest must be CONSULTED, not merely written. Discriminating fixture: a
# copy of this repo installs, then the COPY loses the Codex marketplace, so the
# shipped-source inverse can no longer name it — only a manifest-driven removal
# can take it out of the target.
FS3="$(mktmp)"
tar -cf - --exclude=.git -C "$REPO_ROOT" . 2>/dev/null | tar -xf - -C "$FS3"
SBf="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$FS3/install.sh" "$SBf" --platform codex >/dev/null 2>&1
[[ -f "$SBf/.agents/plugins/marketplace.json" ]] \
  && pass "host-support/2" "manifest-leg fixture: the artifact exists before uninstall (population floor)" \
  || fail "host-support/2" "manifest-leg fixture: the artifact exists before uninstall (population floor)"
rm -f "$FS3/.agents/plugins/marketplace.json"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$FS3/install.sh" "$SBf" --uninstall --yes >/dev/null 2>&1
[[ ! -e "$SBf/.agents/plugins/marketplace.json" ]] \
  && pass "host-support/2" "uninstall removes a manifest-recorded artifact the source no longer ships (manifest consulted)" \
  || fail "host-support/2" "uninstall removes a manifest-recorded artifact the source no longer ships (manifest consulted)"
rm -rf "$SBf" "$FS3"
# MERGE, not replace: installing for ONE host must not erase the ownership records
# of the hosts already installed here — their files are still on disk and would be
# stranded at uninstall.
SBmerge="$(mktmp)"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBmerge" >/dev/null 2>&1
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBmerge" --platform codex >/dev/null 2>&1
MAN="$SBmerge/.vibe/install-manifest.json"
assert_eq "host-support/2" "a later one-host install preserves the other hosts' manifest records" \
  "$([[ "$(man_n claude)" -ge 1 && "$(man_n opencode)" -ge 1 && "$(man_n droid)" -ge 1 ]] && echo yes || echo no)" "yes"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBmerge" --uninstall --yes >/dev/null 2>&1
str=1
for p in .claude/commands/flow.md .opencode/commands/flow.md factory-plugin/commands/flow.md \
         .factory-plugin/marketplace.json; do
  [[ -e "$SBmerge/$p" || -L "$SBmerge/$p" ]] && { str=0; echo "        stranded after uninstall: $p"; }
done
assert_eq "host-support/2" "uninstall removes the preserved hosts' artifacts too (no stranding)" "$str" "1"
rm -rf "$SBmerge"
# …and a --only spec re-install must not CLOBBER a selected host's records either.
# The spec half deploys NO host surface (step 3b skips the selected host), so that
# host's files stay on disk and its existing records stay the truth: dropping them
# left codex's package, its two skill links, and its marketplace recorded nowhere,
# while the surviving opencode records kept the manifest "usable" — so the later
# uninstall took the manifest pass and never fell back, stranding all of them.
SBf1="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBf1" --platform opencode,codex >/dev/null 2>&1
MAN="$SBf1/.vibe/install-manifest.json"
[[ "$(man_n codex)" -ge 1 && "$(man_n opencode)" -ge 1 ]] \
  && pass "host-support/2" "spec-reskip fixture: both selected hosts are recorded first (population floor)" \
  || fail "host-support/2" "spec-reskip fixture: both selected hosts are recorded first (population floor)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBf1" --only spec --platform codex >/dev/null 2>&1
[[ -e "$SBf1/plugins/vibe/plugin.json" && -e "$SBf1/.agents/plugins/marketplace.json" ]] \
  && pass "host-support/2" "spec-reskip fixture: the codex surfaces are still on disk after the spec re-install (floor)" \
  || fail "host-support/2" "spec-reskip fixture: the codex surfaces are still on disk after the spec re-install (floor)"
assert_eq "host-support/2" "a --only spec re-install keeps the SKIPPED host's manifest records (codex)" \
  "$([[ "$(man_n codex)" -ge 1 ]] && echo yes || echo no)" "yes"
assert_eq "host-support/2" "…and keeps the unselected host's records (opencode)" \
  "$([[ "$(man_n opencode)" -ge 1 ]] && echo yes || echo no)" "yes"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBf1" --uninstall --yes >/dev/null 2>&1
stranded1=""
for p in plugins/vibe/plugin.json plugins/vibe/skills/spec plugins/vibe/skills/vibe \
         .agents/plugins/marketplace.json .opencode/commands/flow.md; do
  [[ -e "$SBf1/$p" || -L "$SBf1/$p" ]] && stranded1="$stranded1 $p"
done
assert_eq "host-support/2" "a later uninstall strands nothing after a --only spec re-install" "$stranded1" ""
rm -rf "$SBf1"
# …and the same protection under DETECTION-default (no --platform at all): a plain
# `--only spec` on a machine where host CLIs are detected is the other clobber path
# the same skip branch has to cover.
SBf1d="$(mktmp)"
PATH="$FARM_CO" "$FARM_CO/bash" "$INSTALL" "$SBf1d" >/dev/null 2>&1
MAN="$SBf1d/.vibe/install-manifest.json"
[[ "$(man_n codex)" -ge 1 && "$(man_n opencode)" -ge 1 ]] \
  && pass "host-support/2" "detected-host fixture: both detected hosts are recorded first (population floor)" \
  || fail "host-support/2" "detected-host fixture: both detected hosts are recorded first (population floor)"
PATH="$FARM_CO" "$FARM_CO/bash" "$INSTALL" "$SBf1d" --only spec >/dev/null 2>&1
assert_eq "host-support/2" "a detection-default --only spec re-install keeps the detected hosts' records" \
  "$([[ "$(man_n codex)" -ge 1 && "$(man_n opencode)" -ge 1 ]] && echo yes || echo no)" "yes"
rm -rf "$SBf1d"
# The jq-less manifest_actions fallback parser needs coverage where it MATTERS:
# every merge/removal leg above runs with jq present, so the pure-bash reader was
# never exercised on a real manifest (jq is recommended, not required).
NOJQ8="$(mkshim jq)"
# (a) MERGE: install two hosts with jq, then re-install ONE host without jq — the
# other host's records must survive the fallback parser's read of the manifest.
SBj8="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBj8" --platform opencode,codex >/dev/null 2>&1
MAN="$SBj8/.vibe/install-manifest.json"
[[ "$(man_n codex)" -ge 1 && "$(man_n opencode)" -ge 1 ]] \
  && pass "host-support/2" "jq-less merge fixture: both hosts are recorded first (population floor)" \
  || fail "host-support/2" "jq-less merge fixture: both hosts are recorded first (population floor)"
PATH="$NOJQ8" "$NOJQ8/bash" "$INSTALL" "$SBj8" --platform codex >/dev/null 2>&1
assert_eq "host-support/2" "the jq-less merge parser preserves another host's records" \
  "$([[ "$(man_n opencode)" -ge 1 ]] && echo yes || echo no)" "yes"
assert_eq "host-support/2" "the jq-less merge parser still refreshes the selected host's records" \
  "$([[ "$(man_n codex)" -ge 1 ]] && echo yes || echo no)" "yes"
rm -rf "$SBj8"
# (b) REMOVAL: a manifest-recorded artifact the SOURCE no longer ships can only be
# removed if the jq-less removal pass READ the manifest — the shipped-source
# inverse cannot name a file its source lacks. A co-located user file stays.
FS8="$(mktmp)"
tar -cf - --exclude=.git -C "$REPO_ROOT" . 2>/dev/null | tar -xf - -C "$FS8"
SBj8b="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$FS8/install.sh" "$SBj8b" --platform opencode >/dev/null 2>&1
printf 'mine\n' > "$SBj8b/.opencode/commands/mine.md"
[[ -f "$SBj8b/.opencode/commands/flow.md" ]] \
  && pass "host-support/2" "jq-less removal fixture: the recorded artifact exists before uninstall (population floor)" \
  || fail "host-support/2" "jq-less removal fixture: the recorded artifact exists before uninstall (population floor)"
rm -f "$FS8/.opencode/commands/flow.md"
PATH="$NOJQ8" "$NOJQ8/bash" "$FS8/install.sh" "$SBj8b" --uninstall --yes >/dev/null 2>&1
[[ ! -e "$SBj8b/.opencode/commands/flow.md" ]] \
  && pass "host-support/2" "a jq-less uninstall removes a manifest-recorded artifact the source no longer ships" \
  || fail "host-support/2" "a jq-less uninstall removes a manifest-recorded artifact the source no longer ships"
[[ -f "$SBj8b/.opencode/commands/mine.md" ]] \
  && pass "host-support/2" "…and the jq-less removal pass preserves a co-located user file" \
  || fail "host-support/2" "…and the jq-less removal pass preserves a co-located user file"
rm -rf "$SBj8b" "$FS8" "$NOJQ8"
# LEGACY fallback: a target with no usable manifest (installed before the manifest
# existed) is still fully inverted, by the shipped-source inverse.
SBl="$(mktmp)"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBl" >/dev/null 2>&1
rm -f "$SBl/.vibe/install-manifest.json"
# …and co-located user files here too, so the FALLBACK path is held to the same
# per-file inverse as the manifest path (a blanket rm -rf of the package dir must
# fail this leg, not merely the manifest one).
printf 'mine\n' > "$SBl/.opencode/commands/mine.md"
printf 'mine\n' > "$SBl/factory-plugin/mine.txt"
mkdir -p "$SBl/plugins/other"; printf 'mine\n' > "$SBl/plugins/other/plugin.json"
lout="$(PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBl" --uninstall --yes 2>&1)"
assert_contains "host-support/2" "a manifest-less target reports the shipped-source fallback" "$lout" "falling back to the shipped-source inverse"
leg=1
for p in .opencode/commands/flow.md plugins/vibe/plugin.json .agents/plugins/marketplace.json \
         factory-plugin/commands/flow.md factory-plugin/.factory-plugin/plugin.json \
         .factory-plugin/marketplace.json; do
  [[ -e "$SBl/$p" || -L "$SBl/$p" ]] && { leg=0; echo "        legacy target kept: $p"; }
done
assert_eq "host-support/2" "a manifest-less (legacy) target is fully inverted by the shipped-source inverse" "$leg" "1"
lkept=1
for p in .opencode/commands/mine.md factory-plugin/mine.txt plugins/other/plugin.json; do
  [[ -f "$SBl/$p" ]] || { lkept=0; echo "        legacy uninstall lost user file: $p"; }
done
assert_eq "host-support/2" "the shipped-source fallback is per-file too (user files survive)" "$lkept" "1"
rm -rf "$SBl"
# COLLISION: a user-owned entry at a packaged path is reported with a recovery
# path, left byte-identical, NOT recorded as vibe-owned, and therefore survives
# the uninstall (ownership is what the manifest proves).
SBcol="$(mktmp)"
mkdir -p "$SBcol/.agents/plugins"
printf 'USER marketplace, not vibe\n' > "$SBcol/.agents/plugins/marketplace.json"
colout="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBcol" --platform codex 2>&1)"
grep -qF 'USER marketplace, not vibe' "$SBcol/.agents/plugins/marketplace.json" \
  && pass "host-support/2" "a colliding user file is preserved, not silently replaced" \
  || fail "host-support/2" "a colliding user file is preserved, not silently replaced"
assert_contains "host-support/2" "the collision is reported with a recovery path" "$colout" "move it aside and re-run"
[[ -e "$SBcol/plugins/vibe/plugin.json" ]] \
  && pass "host-support/2" "a collision does not abort the rest of the host package" \
  || fail "host-support/2" "a collision does not abort the rest of the host package"
MAN="$SBcol/.vibe/install-manifest.json"
assert_eq "host-support/2" "a colliding user file is NOT recorded as vibe-owned" \
  "$(grep -c 'marketplace' "$MAN" 2>/dev/null || true)" "0"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBcol" --uninstall --yes >/dev/null 2>&1
grep -qF 'USER marketplace, not vibe' "$SBcol/.agents/plugins/marketplace.json" \
  && pass "host-support/2" "the collided user file survives uninstall (ownership is manifest-proven)" \
  || fail "host-support/2" "the collided user file survives uninstall (ownership is manifest-proven)"
rm -rf "$SBcol"
# A RETARGETED host skill link is the user's: the manifest records the target we
# created, not merely the path, so uninstall must leave their link alone.
SBrt="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBrt" --platform codex >/dev/null 2>&1
rm -f "$SBrt/plugins/vibe/skills/spec"
ln -s ../../../my-own-spec "$SBrt/plugins/vibe/skills/spec"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBrt" --uninstall --yes >/dev/null 2>&1
{ [[ -L "$SBrt/plugins/vibe/skills/spec" && "$(readlink "$SBrt/plugins/vibe/skills/spec")" == "../../../my-own-spec" ]]; } \
  && pass "host-support/2" "uninstall leaves a user-retargeted host skill link alone (target, not path)" \
  || fail "host-support/2" "uninstall removed a user-retargeted host skill link"
rm -rf "$SBrt"
# A codex-only target: the removal pass must prune the directories it emptied even
# though the other hosts' package dirs never existed (the prune names five roots,
# not all of which exist on every target).
SBone="$(mktmp)"
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBone" --platform codex >/dev/null 2>&1
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBone" --uninstall --yes >/dev/null 2>&1
{ [[ ! -e "$SBone/plugins" && ! -e "$SBone/.agents/plugins" ]]; } \
  && pass "host-support/2" "uninstall prunes every emptied host dir, even roots absent on that target" \
  || fail "host-support/2" "uninstall left an emptied host dir behind on a codex-only target"
rm -rf "$SBone"
# --uninstall is NOT narrowed by --platform: removing vibe removes it from every
# host it was installed into, which is what a user asking for removal means.
SBup="$(mktmp)"
PATH="$FARM_ALL" "$FARM_ALL/bash" "$INSTALL" "$SBup" >/dev/null 2>&1
PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBup" --uninstall --yes --platform codex >/dev/null 2>&1
up=1
for p in .claude/commands/flow.md .opencode/commands/flow.md factory-plugin/commands/flow.md; do
  [[ -e "$SBup/$p" || -L "$SBup/$p" ]] && { up=0; echo "        --platform narrowed the uninstall: $p"; }
done
assert_eq "host-support/2" "--uninstall is not narrowed by --platform (every host is inverted)" "$up" "1"
rm -rf "$SBup"
# Capability report (R5). codex-only is the sharpest case: nothing may be said
# about hooks, and the entry mechanism must come from the shipped surface matrix.
SBs="$(mktmp)"
sout="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBs" --platform codex 2>&1)"
assert_contains "host-support/2" "the capability report names the configured host" "$sout" "codex"
inv="$(jq -r '.hosts[] | select(.id == "codex") | .entry.invocation' "$SURFACES" 2>/dev/null)"
[[ -n "$inv" ]] && pass "host-support/2" "population floor: the matrix declares the codex invocation" \
  || fail "host-support/2" "population floor: the matrix declares the codex invocation"
assert_contains "host-support/2" "the report's codex entry mechanism matches the shipped surface matrix" "$sout" "$inv"
# …and its skills claim too: the report names plugins/vibe/skills for codex, so
# the token it prints must come from the matrix (the matrix was the outlier that
# omitted the package surface, which is exactly the drift this pins).
cskdir="$(jq -r '.hosts[] | select(.id == "codex") | .skills[] | select(endswith("plugins/vibe/skills/{name}")) | rtrimstr("/{name}")' "$SURFACES" 2>/dev/null)"
[[ -n "$cskdir" ]] \
  && pass "host-support/2" "population floor: the matrix declares the codex package skill surface" \
  || fail "host-support/2" "population floor: the matrix declares the codex package skill surface"
assert_contains "host-support/2" "the report's codex skills claim matches the matrix's package surface" "$sout" "$cskdir"
assert_not_contains "host-support/2" "a codex-only report claims no hook support" "$sout" "hook"
assert_not_contains "host-support/2" "a codex-only report claims no native agent/droid registration" "$sout" "subagent"
rm -rf "$SBs"
# The jq-less report must agree with the matrix too: the fallback map is not a
# second source of truth that may drift from the data.
NOJQ3="$(mkshim jq)"
SBx="$(mktmp)"
xout="$(PATH="$NOJQ3" "$NOJQ3/bash" "$INSTALL" "$SBx" --platform codex 2>&1)"
assert_contains "host-support/2" "the jq-less report matches the matrix's declared invocation" "$xout" "$inv"
rm -rf "$SBx" "$NOJQ3"
# Claude hooks are reported only when their wiring actually installed.
SBc2="$(mktmp)"
cout="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBc2" --platform claude 2>&1)"
assert_contains "host-support/2" "a wired Claude install reports the hooks as wired" "$cout" "hooks wired via .claude/settings.json"
rm -rf "$SBc2"
NOJQ4="$(mkshim jq)"
SBc3="$(mktmp)"
cout3="$(PATH="$NOJQ4" "$NOJQ4/bash" "$INSTALL" "$SBc3" --platform claude 2>&1)"
assert_contains "host-support/2" "an unwired Claude install reports the hooks as NOT wired" "$cout3" "hooks NOT wired"
assert_not_contains "host-support/2" "an unwired Claude install never claims the hooks are live" "$cout3" "hooks are wired automatically"
rm -rf "$SBc3" "$NOJQ4"
# Every host line credits only that host.
SBmix="$(mktmp)"
mout="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBmix" --platform claude,opencode 2>&1)"
oline="$(printf '%s\n' "$mout" | grep -m1 '^install:   opencode' || true)"
assert_contains "host-support/2" "the opencode line names its /flow entry point" "$oline" "/flow (.opencode/commands/flow.md)"
assert_not_contains "host-support/2" "the opencode line claims no hooks (R5)" "$oline" "hook"
assert_not_contains "host-support/2" "the opencode line claims no native agent registration" "$oline" "subagent"
rm -rf "$SBmix"
# R5 means the claims follow DEPLOYMENT REALITY, not the selection alone. Under
# --only spec NO host entry point is deployed (step 3b skips every host), so the
# claude line used to name /flow, the .claude/skills registration, and a hooks
# state — for steps that never ran, with no warning printed to point at.
SBsp="$(mktmp)"
spout="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBsp" --only spec --platform claude 2>&1)"
[[ ! -e "$SBsp/.claude/commands" ]] \
  && pass "host-support/2" "only-spec-report fixture: no .claude/commands was deployed (population floor)" \
  || fail "host-support/2" "only-spec-report fixture: no .claude/commands was deployed (population floor)"
assert_not_contains "host-support/2" "a --only spec report claims no /flow entry point" "$spout" ".claude/commands/flow.md"
assert_not_contains "host-support/2" "a --only spec report claims no hook state (the wiring step never ran)" "$spout" "hooks NOT wired"
assert_not_contains "host-support/2" "a --only spec report claims no .claude/skills registration" "$spout" ".claude/skills/{spec,vibe}"
assert_contains "host-support/2" "a --only spec report says the host was not deployed" "$spout" "not deployed (--only spec"
rm -rf "$SBsp"
SBsp2="$(mktmp)"
spout2="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBsp2" --only spec --platform codex 2>&1)"
assert_not_contains "host-support/2" "a --only spec codex report claims no package skill surface" "$spout2" "plugins/vibe/skills"
rm -rf "$SBsp2"
# A colliding entry point is the user's, so the report says NOT installed — the
# old report named `opencode /flow (.opencode/commands/flow.md)` regardless of the
# WARN that deploy_file printed and the file it left untouched.
SBcolr="$(mktmp)"; mkdir -p "$SBcolr/.opencode/commands"
printf 'USER flow command, not vibe\n' > "$SBcolr/.opencode/commands/flow.md"
colrout="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBcolr" --platform opencode 2>&1)"
assert_contains "host-support/2" "the report says the collided entry point was NOT installed" "$colrout" "NOT installed (collision"
assert_not_contains "host-support/2" "the report does not claim a colliding /flow entry point" "$colrout" "/flow (.opencode/commands/flow.md)"
rm -rf "$SBcolr"
# R5 covers the other half too: --only flow installs no spec skill, and the
# completion banner right above the report says so — a skills claim of
# `{spec,vibe}` contradicted the banner.
SBfo="$(mktmp)"
foout="$(PATH="$FARM_NONE" "$FARM_NONE/bash" "$INSTALL" "$SBfo" --only flow --platform claude 2>&1)"
[[ ! -e "$SBfo/.agents/skills/spec" ]] \
  && pass "host-support/2" "only-flow-report fixture: no spec half was installed (population floor)" \
  || fail "host-support/2" "only-flow-report fixture: no spec half was installed (population floor)"
assert_not_contains "host-support/2" "a --only flow report claims no spec skill it did not install" "$foout" ".agents/skills/{spec,vibe}"
assert_not_contains "host-support/2" "a --only flow claude report claims no spec registration it did not make" "$foout" ".claude/skills/{spec,vibe}"
assert_contains "host-support/2" "a --only flow report still names the vibe skill it did install" "$foout" ".agents/skills/{vibe}"
rm -rf "$SBfo"
rm -rf "$FARM_NONE" "$FARM_ALL" "$FARM_CO"

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
