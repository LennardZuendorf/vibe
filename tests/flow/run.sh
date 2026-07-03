#!/usr/bin/env bash
# tests/flow/run.sh — behaviour tests for the vibe-flow core: the state machine,
# set-state.sh / validate-state.sh, D12 orders (orders.sh + per-skill blocks),
# and graceful degradation (check-skills.sh). Pure bash; no bats.
#
# Each test cites its plan unit ID (vibe-flow/n). Exit 0 = all pass.
#
# These tests never depend on a live cursor: they pass explicit state args or
# seed a throwaway state.json and restore the original.

# The `cond && pass || fail` reporting idiom is intentional and safe here:
# pass()/fail() always return 0, so fail never runs spuriously after pass.
# shellcheck disable=SC2015
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FLOW="$REPO_ROOT/.agents/skills/vibe"
SCRIPTS="$FLOW/scripts"
MACHINE="$FLOW/state-machine.json"
SKILLS="$REPO_ROOT/.agents/skills"
STATE="$FLOW/state.json"

PASS=0
FAIL=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
assert_contains()     { if [[ "$3" == *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        expected to contain: $4"; echo "        got: $3"; fi; }
assert_not_contains() { if [[ "$3" != *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        expected NOT to contain: $4"; fi; }
assert_eq()           { if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        expected: $4"; echo "        got:      $3"; fi; }

# Save/restore any real cursor so tests are side-effect free.
BACKUP=""
if [[ -f "$STATE" ]]; then BACKUP="$(mktemp)"; cp "$STATE" "$BACKUP"; fi
restore_state() {
  if [[ -n "$BACKUP" ]]; then cp "$BACKUP" "$STATE"; rm -f "$BACKUP"; else rm -f "$STATE"; fi
}
trap restore_state EXIT

echo "=== vibe-flow/1 — D12 orders source ==="

# orders.sh idle fallback (no cursor).
rm -f "$STATE"
out="$(bash "$SCRIPTS/orders.sh")"
assert_contains "vibe-flow/1" "orders.sh idle prints machine inline fallback" "$out" "state=idle"

# Every skill-owning state (skill != null, not idle) has inject:null in the
# machine AND resolves to a non-empty orders block in its linked skill.
states="$(jq -r '.states | to_entries[] | select(.key != "idle") | select(.value.skill != null) | .key' "$MACHINE")"
all_null=1; all_block=1
while IFS= read -r s; do
  [[ -z "$s" ]] && continue
  inj="$(jq -r --arg s "$s" '.states[$s].inject' "$MACHINE")"
  [[ "$inj" == "null" ]] || { all_null=0; echo "        $s inject not null: $inj"; }
  skill="$(jq -r --arg s "$s" '.states[$s].skill' "$MACHINE")"
  block="$(bash "$SCRIPTS/orders.sh" "$s")"
  if [[ -z "$block" ]] || [[ "$block" == *"state=unknown"* ]]; then
    all_block=0; echo "        $s resolved no orders block (skill=$skill)"
  fi
  grep -qF "<!-- vibe:orders:$s -->" "$SKILLS/$skill/SKILL.md" || { all_block=0; echo "        $skill missing block for $s"; }
done <<< "$states"
assert_eq "vibe-flow/1" "all skill-owning states carry inject:null" "$all_null" "1"
assert_eq "vibe-flow/1" "all skill-owning states resolve a non-empty orders block from the linked skill" "$all_block" "1"

# idle keeps an inline inject in the machine (skill-less fallback).
idle_inj="$(jq -r '.states.idle.inject' "$MACHINE")"
assert_not_contains "vibe-flow/1" "idle retains inline inject (not null)" "$idle_inj" "null"

# <feature> interpolation is stable and substitutes from the cursor.
out="$(bash "$SCRIPTS/orders.sh" feature.impl)"
assert_contains "vibe-flow/1" "orders without cursor feature keep <feature> placeholder" "$out" "<feature>/n"
cp "$FLOW/state.example.json" "$STATE"
bash "$SCRIPTS/set-state.sh" feature.impl widget >/dev/null
out="$(bash "$SCRIPTS/orders.sh")"
assert_contains "vibe-flow/1" "orders interpolate cursor feature" "$out" "widget/n"
assert_not_contains "vibe-flow/1" "no leftover placeholder after interpolation" "$out" "<feature>/n"

# orders.sh degrades when jq is absent (still exits 0, prints fallback). Build a
# throwaway PATH carrying only the coreutils orders.sh needs (dirname) but NOT jq,
# so the no-jq path is exercised on any platform (jq is /usr/bin/jq on Linux).
BASH_BIN="$(command -v bash)"
nojq="$(mktemp -d)"
ln -sf "$(command -v dirname)" "$nojq/dirname"
out="$(PATH="$nojq" "$BASH_BIN" "$SCRIPTS/orders.sh" 2>/dev/null; echo "rc=$?")"
rm -rf "$nojq"
assert_contains "vibe-flow/1" "orders.sh exits 0 even without jq" "$out" "rc=0"
assert_not_contains "vibe-flow/1" "orders.sh without jq does not crash blank" "$out" "rc=127"
assert_contains "vibe-flow/1" "orders.sh without jq still prints a fallback" "$out" "state="

echo ""
echo "=== vibe-flow/3 — graceful skill degradation ==="
out="$(bash "$SCRIPTS/check-skills.sh" feature.design 2>&1; echo "rc=$?")"
assert_contains "vibe-flow/3" "check-skills warns on assumed-installed superpowers" "$out" "superpowers:brainstorming"
assert_contains "vibe-flow/3" "check-skills never hard-fails (exit 0)" "$out" "rc=0"
out="$(bash "$SCRIPTS/check-skills.sh" caveman ultra)"
assert_contains "vibe-flow/3" "caveman fallback prints level definition" "$out" "caveman[ultra]"
out="$(bash "$SCRIPTS/check-skills.sh" setup.detect)"
assert_contains "vibe-flow/3" "check-skills confirms bundled spec when delegated" "$(bash "$SCRIPTS/check-skills.sh" strategy.spec 2>&1)" "spec"

echo ""
echo "=== set-state.sh — writer, not gate ==="
cp "$FLOW/state.example.json" "$STATE"
out="$(bash "$SCRIPTS/set-state.sh" amend 2>&1; echo "rc=$?")"
assert_contains "vibe-flow/core" "set-state.sh rejects amend (modifier)" "$out" "modifier"
assert_not_contains "vibe-flow/core" "set-state.sh does not accept amend (rc!=0)" "$out" "rc=0"
out="$(bash "$SCRIPTS/set-state.sh" bogus.state 2>&1; echo "rc=$?")"
assert_contains "vibe-flow/core" "set-state.sh rejects unknown state" "$out" "not a known state"
bash "$SCRIPTS/set-state.sh" feature.design alpha >/dev/null
bash "$SCRIPTS/set-state.sh" feature.plan >/dev/null
feat="$(jq -r '.feature' "$STATE")"
assert_eq "vibe-flow/core" "set-state.sh preserves feature across transitions" "$feat" "alpha"
bash "$SCRIPTS/set-state.sh" idle >/dev/null
feat="$(jq -r '.feature' "$STATE")"
assert_eq "vibe-flow/core" "idle clears the feature pointer" "$feat" "null"

echo ""
echo "=== validate-state.sh — cursor sanity ==="
cp "$FLOW/state.example.json" "$STATE"
out="$(bash "$SCRIPTS/validate-state.sh" 2>&1; echo "rc=$?")"
assert_contains "vibe-flow/core" "validate-state OK on a fresh cursor" "$out" "rc=0"
printf '{not json' > "$STATE"
out="$(bash "$SCRIPTS/validate-state.sh" 2>&1; echo "rc=$?")"
assert_contains "vibe-flow/core" "validate-state fails on corrupt JSON" "$out" "rc=1"

echo ""
echo "=== state-machine.json — internal consistency ==="
# every `next` target is a known state
bad_next="$(jq -r '
  .states as $s
  | [ $s | to_entries[] | .value.next[]? ] | unique
  | map(select(. as $n | ($s | has($n)) | not)) | join(",")
' "$MACHINE")"
assert_eq "vibe-flow/core" "every next target is a known state" "$bad_next" ""
# every non-null skill links to an existing skill dir
missing_skill=0
while IFS= read -r sk; do
  [[ -z "$sk" ]] && continue
  [[ -f "$SKILLS/$sk/SKILL.md" ]] || { missing_skill=1; echo "        missing skill dir: $sk"; }
done <<< "$(jq -r '.states[].skill | select(. != null)' "$MACHINE" | sort -u)"
assert_eq "vibe-flow/core" "every linked skill has a SKILL.md" "$missing_skill" "0"

echo ""
echo "=== regen-active-rules.sh — digest from lessons ==="
d="$(mktemp -d)"
mkdir -p "$d/.spec" "$d/.agents/skills/vibe/scripts"
cp "$SCRIPTS/regen-active-rules.sh" "$d/.agents/skills/vibe/scripts/"
cat > "$d/.spec/lessons.md" <<'EOF'
# Lessons

### Inline comment lesson
**Pattern:** mentions a marker token.
**Rule:** keep the `<!-- vibe:orders:x -->` token intact in the body.
**Tags:** t
**Date:** 2026-06-18

<!-- Format for each lesson:
### Should be ignored
**Rule:** this template line must not become a digest entry
-->
EOF
printf '# T\n<!-- vibe:active-rules:start -->\nold\n<!-- vibe:active-rules:end -->\n' > "$d/AGENTS.md"
bash "$d/.agents/skills/vibe/scripts/regen-active-rules.sh" >/dev/null 2>&1
block="$(awk '/active-rules:start/,/active-rules:end/' "$d/AGENTS.md")"
assert_contains "vibe-flow/core" "regen captures rule body even with an inline <!-- token" "$block" "keep the"
assert_not_contains "vibe-flow/core" "regen excludes the format-template comment lesson" "$block" "must not become a digest entry"
# regression: regen leaves no stray temp files beside the target — neither the
# .block.* blockfile nor the .XXXXXX mktemp tmp (both are named AGENTS.md.*).
strays="$(find "$d" -name 'AGENTS.md.*' | wc -l | tr -d ' ')"
assert_eq "vibe-flow/core" "regen leaves no stray temp files beside the target" "$strays" "0"
rm -rf "$d"

echo ""
echo "=== path parity — symlinked vs real path ==="
# Scripts self-locate via ${BASH_SOURCE}; invoking through the canonical flow/
# path and through the .agents/skills/vibe symlink alias must be byte-identical
# (same state file, same machine, same output). Seed a deterministic cursor so
# both spellings read identical state.
REAL_SCRIPTS="$REPO_ROOT/flow/scripts"
ALIAS_SCRIPTS="$REPO_ROOT/.agents/skills/vibe/scripts"
cp "$FLOW/state.example.json" "$STATE"
bash "$SCRIPTS/set-state.sh" feature.impl widget >/dev/null
for st in idle feature.impl strategy.spec feature.compound quick.fix; do
  a="$(bash "$REAL_SCRIPTS/orders.sh" "$st" 2>&1)"
  b="$(bash "$ALIAS_SCRIPTS/orders.sh" "$st" 2>&1)"
  assert_eq "path-parity" "orders.sh identical via real vs symlink path ($st)" "$a" "$b"
done
a="$(bash "$REAL_SCRIPTS/detect-context.sh" snapshot 2>&1)"
b="$(bash "$ALIAS_SCRIPTS/detect-context.sh" snapshot 2>&1)"
assert_eq "path-parity" "detect-context snapshot identical via both paths" "$a" "$b"
a="$(bash "$REAL_SCRIPTS/detect-context.sh" decide .spec/lessons.md 2>&1)"
b="$(bash "$ALIAS_SCRIPTS/detect-context.sh" decide .spec/lessons.md 2>&1)"
assert_eq "path-parity" "detect-context decide identical via both paths" "$a" "$b"
a="$(bash "$REAL_SCRIPTS/validate-state.sh" 2>&1)"
b="$(bash "$ALIAS_SCRIPTS/validate-state.sh" 2>&1)"
assert_eq "path-parity" "validate-state identical via both paths" "$a" "$b"
# doctor.sh self-locates the repo root by marker search — both spellings must agree,
# with an explicit root and via find_root (no-arg).
a="$(bash "$REAL_SCRIPTS/doctor.sh" "$REPO_ROOT" 2>&1)"
b="$(bash "$ALIAS_SCRIPTS/doctor.sh" "$REPO_ROOT" 2>&1)"
assert_eq "path-parity" "doctor identical via both paths (explicit root)" "$a" "$b"
a="$(bash "$REAL_SCRIPTS/doctor.sh" 2>&1)"
b="$(bash "$ALIAS_SCRIPTS/doctor.sh" 2>&1)"
assert_eq "path-parity" "doctor identical via both paths (self-located root)" "$a" "$b"

# regen-active-rules resolves the repo root by marker search, so a sandbox reached
# via a real flow/ path and via a .agents/skills/vibe symlink yields the same
# rewrite (a fixed `..` hop overshot the root on the flow/ path).
pr="$(mktemp -d)"
mkdir -p "$pr/flow/scripts" "$pr/.agents/skills" "$pr/.spec"
cp "$SCRIPTS/regen-active-rules.sh" "$pr/flow/scripts/"
( cd "$pr/.agents/skills" && ln -s ../../flow vibe )
cat > "$pr/.spec/lessons.md" <<'EOF'
# Lessons

### Parity lesson
**Rule:** paths resolve the same via symlink or real dir.
**Date:** 2026-07-03
EOF
seed_agents() { printf '# T\n<!-- vibe:active-rules:start -->\nold\n<!-- vibe:active-rules:end -->\n' > "$pr/AGENTS.md"; }
seed_agents; bash "$pr/flow/scripts/regen-active-rules.sh" >/dev/null 2>&1; real_out="$(cat "$pr/AGENTS.md")"
seed_agents; bash "$pr/.agents/skills/vibe/scripts/regen-active-rules.sh" >/dev/null 2>&1; alias_out="$(cat "$pr/AGENTS.md")"
assert_eq "path-parity" "regen-active-rules identical via both paths" "$real_out" "$alias_out"
assert_contains "path-parity" "regen via real flow/ path resolves the repo root" "$real_out" "paths resolve the same"
rm -rf "$pr"

echo ""
echo "=== install-tooling/4 — doctor.sh + deps.json ==="
DOCTOR="$SCRIPTS/doctor.sh"
DEPS="$FLOW/reference/deps.json"
# deps.json: valid JSON; every entry carries the five required fields.
jq -e . "$DEPS" >/dev/null 2>&1 && pass "install-tooling/4" "deps.json is valid JSON" || fail "install-tooling/4" "deps.json JSON"
missing_field="$(jq -r '[.deps[] | select((has("name") and has("kind") and has("source") and has("required_by") and has("degrade")) | not)] | length' "$DEPS")"
assert_eq "install-tooling/4" "every deps.json entry has name/kind/source/required_by/degrade" "$missing_field" "0"
# Healthy source repo: exit 0 and the core checks report ok.
out="$(bash "$DOCTOR" 2>&1; echo "rc=$?")"
assert_contains "install-tooling/4" "doctor exits 0 on a healthy repo" "$out" "rc=0"
assert_contains "install-tooling/4" "doctor reports core.vibe ok" "$out" "ok   core.vibe"
assert_contains "install-tooling/4" "doctor reports machine ok" "$out" "ok   machine"
assert_contains "install-tooling/4" "doctor reports adapter wiring ok" "$out" "ok   adapter.hooks"
assert_contains "install-tooling/4" "doctor lists dep superpowers" "$out" "dep.superpowers"
assert_contains "install-tooling/4" "doctor lists dep feature-dev" "$out" "dep.feature-dev"
assert_contains "install-tooling/4" "doctor lists dep caveman" "$out" "dep.caveman"
# Broken install (dead vibe symlink, no machine/adapter): warns, still exits 0.
d="$(mktemp -d)"; mkdir -p "$d/.spec" "$d/.agents/skills"
ln -s /nonexistent-vibe "$d/.agents/skills/vibe"
out="$(bash "$DOCTOR" "$d" 2>&1; echo "rc=$?")"
assert_contains "install-tooling/4" "doctor flags a broken vibe symlink" "$out" "warn core.vibe"
assert_contains "install-tooling/4" "broken-symlink message names it" "$out" "BROKEN"
assert_contains "install-tooling/4" "doctor still exits 0 on a broken install" "$out" "rc=0"
rm -rf "$d"
# Invalid cursor: doctor delegates to validate-state.sh, warns, still exits 0.
d="$(mktemp -d)"; mkdir -p "$d/.spec" "$d/.agents/skills/vibe/scripts"
cp "$SCRIPTS/validate-state.sh" "$d/.agents/skills/vibe/scripts/"; chmod +x "$d/.agents/skills/vibe/scripts/validate-state.sh"
cp "$MACHINE" "$d/.agents/skills/vibe/"
printf '{not valid json' > "$d/.agents/skills/vibe/state.json"
out="$(bash "$DOCTOR" "$d" 2>&1; echo "rc=$?")"
assert_contains "install-tooling/4" "doctor warns on an invalid cursor" "$out" "warn cursor"
assert_contains "install-tooling/4" "doctor still exits 0 on an invalid cursor" "$out" "rc=0"
rm -rf "$d"
# Valid cursor: doctor delegates to validate-state.sh and reports ok (happy path + delegation).
d="$(mktemp -d)"; mkdir -p "$d/.spec" "$d/.agents/skills/vibe/scripts"
cp "$SCRIPTS/validate-state.sh" "$d/.agents/skills/vibe/scripts/"; chmod +x "$d/.agents/skills/vibe/scripts/validate-state.sh"
cp "$MACHINE" "$d/.agents/skills/vibe/"
cp "$FLOW/state.example.json" "$d/.agents/skills/vibe/state.json"
out="$(bash "$DOCTOR" "$d" 2>&1; echo "rc=$?")"
assert_contains "install-tooling/4" "doctor reports a valid cursor as ok" "$out" "ok   cursor"
rm -rf "$d"
# Absent dep: reported as a warn with its degrade text, still exit 0.
d="$(mktemp -d)"; mkdir -p "$d/.spec" "$d/.agents/skills/vibe/reference"
printf '{"deps":[{"name":"vibe-nonexistent-dep-xyz","kind":"skill-collection","source":"x","required_by":["*"],"degrade":"inline fallback"}]}\n' > "$d/.agents/skills/vibe/reference/deps.json"
out="$(bash "$DOCTOR" "$d" 2>&1; echo "rc=$?")"
assert_contains "install-tooling/4" "doctor warns an absent dep with degrade text" "$out" "degrade: inline fallback"
assert_contains "install-tooling/4" "doctor exits 0 with a missing dep" "$out" "rc=0"
rm -rf "$d"

echo ""
echo "=== orders.sh on a fresh non-git install (stranger-eval regression) ==="
# A fresh full install creates neither .git nor .spec. orders.sh must still
# resolve the linked skill's block by self-locating the skills dir from its own
# path — not degrade to 'state=unknown' for want of a repo-root marker.
SBI="$(mktemp -d)"
bash "$REPO_ROOT/install.sh" "$SBI" >/dev/null 2>&1
[[ ! -e "$SBI/.git" && ! -e "$SBI/.spec" ]] \
  && pass "orders-fresh" "fresh install has no .git/.spec marker (precondition)" \
  || fail "orders-fresh" "precondition: fresh install unexpectedly has a marker"
bash "$SBI/.agents/skills/vibe/scripts/set-state.sh" quick.triage >/dev/null 2>&1
out="$(bash "$SBI/.agents/skills/vibe/scripts/orders.sh" quick.triage 2>&1)"
assert_not_contains "orders-fresh" "orders.sh does not degrade to state=unknown on a fresh non-git install" "$out" "state=unknown"
[[ -n "$out" ]] && pass "orders-fresh" "orders.sh returns a non-empty block on a fresh install" || fail "orders-fresh" "orders.sh empty on fresh install"
# and the block is byte-identical whether or not a marker is later added
( cd "$SBI" && git init -q >/dev/null 2>&1 )
out2="$(bash "$SBI/.agents/skills/vibe/scripts/orders.sh" quick.triage 2>&1)"
assert_eq "orders-fresh" "orders.sh block identical with and without a repo-root marker" "$out" "$out2"
rm -rf "$SBI"

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
