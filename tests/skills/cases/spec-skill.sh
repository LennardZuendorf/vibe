# shellcheck shell=bash disable=SC2016
# tests/skills/cases/spec-skill.sh — Track A: the spec skill, its agents and its
# pressure harness (skill-revision/2-7). Sourced by run.sh after lib.sh.

_sa="$(mktemp -d)"
_P="$REPO_ROOT/spec/tests/pressure"

# --- skill-revision/2: pressure harness on v0.4 roots, with near-misses -------
_n="$(awk -F'\t' 'NR > 1 && $2 == "none"' "$_P/scenarios.tsv" | grep -c .)"
assert_population "skill-revision/2" "scenarios.tsv near-miss rows (accept = none)" "$_n" 6
_n="$(awk -F'\t' 'NR > 1 && $2 != "none"' "$_P/scenarios.tsv" | grep -c .)"
assert_population "skill-revision/2" "scenarios.tsv should-trigger rows" "$_n" 6

_h="$(cat "$_P/run.sh")"
assert_contains "skill-revision/2" "harness takes the binary from VIBE_SPEC_BIN" "$_h" 'VIBE_SPEC_BIN'
assert_contains "skill-revision/2" "harness builds the binary when VIBE_SPEC_BIN is unset" "$_h" 'cargo build -q -p vibe-spec'
assert_contains "skill-revision/2" "harness seeds the sandbox with vibe-spec init" "$_h" '"$VSB" init'
assert_contains "skill-revision/2" "harness checks the sandbox with vibe-spec root" "$_h" '"$VSB" root'
if grep -qF -- '-d "$d/proj/.spec"' "$_P/run.sh"; then # literal
  fail "skill-revision/2" "harness no longer tests for a fixed legacy directory"
else pass "skill-revision/2" "harness no longer tests for a fixed legacy directory"; fi
assert_contains "skill-revision/2" "harness has a --root legacy leg" "$_h" 'legacy)'
assert_contains "skill-revision/2" "legacy leg seeds with the frozen setup.sh" "$_h" 'spec/scripts/setup.sh'
out="$(bash "$_P/run.sh" --root bogus 2>&1 </dev/null)"; rc=$?
assert_rc "skill-revision/2" "harness refuses an unknown --root before any run" 2 "$rc"
assert_contains "skill-revision/2" "the refusal names the --root values" "$out" "default|legacy"

# _score ACCEPT SP FIRST EXPECTED_RC — the harness scorer, fed canned first-skill values.
_score() {
  local out rc
  out="$(bash "$_P/run.sh" --score "$1" "$2" "$3" 2>&1 </dev/null)"; rc=$?
  assert_rc "skill-revision/2" "score accept=$1 sp=$2 first=$3 -> rc $4 ($out)" "$4" "$rc"
}
_trig='spec|superpowers:brainstorming'
_score "$_trig" present spec 0
_score "$_trig" present none 1
_score "$_trig" present superpowers:brainstorming 0
_score "$_trig" absent superpowers:brainstorming 1
_score none present spec 1
_score none present vibe 1
_score none present none 0
_score none present superpowers:brainstorming 0
_score none absent none 0
_score none absent spec 1

rm -rf "$_sa"
