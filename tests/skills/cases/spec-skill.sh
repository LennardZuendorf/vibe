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

# --- skill-revision/3: description, frontmatter, selection evals -------------
_S="$REPO_ROOT/spec/SKILL.md"
out="$(lint_description "$_S" assertive)"; rc=$?
assert_rc "skill-revision/3" "spec description passes the lint (assertive) ${out:+— $out}" 0 "$rc"
out="$(lint_frontmatter "$_S" "name description user-invocable argument-hint allowed-tools compatibility" spec)"; rc=$?
assert_rc "skill-revision/3" "spec frontmatter keys are the allowlist ${out:+— $(printf '%s' "$out" | tr '\n' ' ')}" 0 "$rc"
_J="$REPO_ROOT/spec/tests/evals/selection.json"
_n="$(grep -c '"expect": "spec"' "$_J" 2>/dev/null)"
assert_population "skill-revision/3" "selection evals should-select queries" "${_n:-0}" 10
_n="$(grep -c '"expect": "none"' "$_J" 2>/dev/null)"
assert_population "skill-revision/3" "selection evals near-miss queries" "${_n:-0}" 10
_bad=0
while IFS=$'\t' read -r _id _acc _q; do
  [[ "$_acc" == none ]] || continue
  _q="${_q//\"/\\\"}"
  grep -F "\"query\": \"$_q\"" "$_J" 2>/dev/null | grep -qF '"expect": "none"' || { _bad=1; echo "        missing near-miss: $_q"; }
done < "$_P/scenarios.tsv"
assert_rc "skill-revision/3" "every harness near-miss is a selection-eval near-miss" 0 "$_bad"

# --- skill-revision/4: Rule, Routing and Red Flags ---------------------------
_sk="$(cat "$_S")"
if grep -qF 'bounded path' "$_S"; then fail "skill-revision/4" "SKILL.md names no 'bounded path' (superpowers has none)"
else pass "skill-revision/4" "SKILL.md names no 'bounded path' (superpowers has none)"; fi
if grep -qE '^\| `audit` \|' "$_S"; then fail "skill-revision/4" "SKILL.md has no audit argument row"
else pass "skill-revision/4" "SKILL.md has no audit argument row"; fi
assert_contains "skill-revision/4" "SKILL.md has a SUBAGENT-STOP block" "$_sk" "<SUBAGENT-STOP>"
assert_contains "skill-revision/4" "SKILL.md has a Not-for line" "$_sk" "**Not for:**"
assert_contains "skill-revision/4" "SKILL.md states the core principle" "$_sk" "**Core principle:**"
assert_contains "skill-revision/4" "Red Flags end with the consequence line" "$_sk" "**Any of these thoughts means: stop, invoke this skill, route.**"
awk '/^## The Rule/{f=1} /^## Routing/{f=2} f==1' "$_S" > "$_sa/rule.md"
awk '/^## The Rule/{f=1} /^## Red Flags/{f=0} f' "$_S" > "$_sa/rule-routing.md"
assert_contains "skill-revision/4" "The Rule runs vibe-spec lessons-for" "$(cat "$_sa/rule.md")" "vibe-spec lessons-for"
_n="$(grep -c . "$_sa/rule-routing.md")"
assert_population "skill-revision/4" "Rule + Routing lines examined" "$_n" 30
out="$(fixed_root_hits "$_sa/rule-routing.md")"
assert_eq "skill-revision/4" "Rule and Routing name no fixed spec root" "" "$out"

rm -rf "$_sa"
