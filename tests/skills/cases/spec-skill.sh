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

# --- skill-revision/5: layers, root-neutral paths, no-binary truth, setup move --
_R="$REPO_ROOT/spec/reference"
out="$(fixed_root_hits "$_S")"
assert_eq "skill-revision/5" "SKILL.md names no fixed spec root" "" "$out"
_w="$(wc -w < "$_S" | tr -d ' ')"
if [[ "$_w" -le 1400 ]]; then pass "skill-revision/5" "SKILL.md is $_w words (ceiling 1400)"
else fail "skill-revision/5" "SKILL.md is $_w words (ceiling 1400)"; fi
for _x in '.config.yaml' 'build it'; do
  if grep -qF "$_x" "$_S"; then fail "skill-revision/5" "SKILL.md does not say '$_x'"
  else pass "skill-revision/5" "SKILL.md does not say '$_x'"; fi
done
_inj="$(grep -E '^!`vibe-spec list-specs' "$_S" || true)"
assert_contains "skill-revision/5" "the injection line falls back when the binary is missing" "$_inj" '2>/dev/null ||'
assert_contains "skill-revision/5" "SKILL.md links reference/layered.md" "$(cat "$_S")" "(reference/layered.md)"
assert_contains "skill-revision/5" "SKILL.md links reference/setup.md" "$(cat "$_S")" "(reference/setup.md)"
_su="$(cat "$_R/setup.md" 2>/dev/null)"
for _x in featureLayer suggest superpowers; do
  assert_contains "skill-revision/5" "reference/setup.md names the $_x key" "$_su" "\`$_x\`"
done
if grep -qF 'vibe-flow' "$_R/setup.md" 2>/dev/null; then fail "skill-revision/5" "reference/setup.md drops the vibe-flow key"
else pass "skill-revision/5" "reference/setup.md drops the vibe-flow key"; fi
_ly="$_R/layered.md"
if [[ -f "$_ly" && "$(wc -l < "$_ly" | tr -d ' ')" -le 100 ]]; then pass "skill-revision/5" "reference/layered.md exists, at most 100 lines"
else fail "skill-revision/5" "reference/layered.md exists, at most 100 lines"; fi

# Application check: a vibe-spec init root plus the notes layered.md's example writes.
_vsb="${VIBE_SPEC_BIN:-$REPO_ROOT/target/debug/vibe-spec}"
[[ -x "$_vsb" ]] || (cd "$REPO_ROOT" && cargo build -q --locked -p vibe-spec >/dev/null 2>&1) || true
if [[ ! -x "$_vsb" ]]; then
  fail "skill-revision/5" "vibe-spec binary available for the application check ($_vsb)"
else
  _p="$_sa/layered"; mkdir -p "$_p"
  (cd "$_p" && git init -q . && "$_vsb" init >/dev/null 2>&1)
  mkdir -p "$_p/.vibe"
  printf '{"featureLayer": "vibe", "superpowers": {"writing-plans": false}, "suggest": false}\n' > "$_p/.vibe/spec.json"
  (cd "$_p" && "$_vsb" root >/dev/null 2>&1); rc=$?
  assert_rc "skill-revision/5" "control: the three skill keys in .vibe/spec.json leave vibe-spec root at exit 0" 0 "$rc"
  # Write every `<root>/<path>`: block of the example; "Add to" appends.
  awk -v R="$_p/docs/spec" '
    /^(Add to )?`<root>\/[^`]+`/ { p = $0; sub(/^[^`]*`<root>\//, "", p); sub(/`.*$/, "", p); app = ($0 ~ /^Add to/); next }
    /^```markdown$/ && p != "" { inb = 1; f = R "/" p; d = f; sub(/\/[^\/]*$/, "", d); system("mkdir -p \"" d "\""); if (!app) printf "" > f; next }
    /^```$/ && inb { inb = 0; close(f); p = ""; next }
    inb { print >> f }
  ' "$_ly"
  _notes="$(cd "$_p/docs/spec" && find . -path './*/*.md' -type f | grep -c .)"
  assert_population "skill-revision/5" "notes written from the layered.md example" "$_notes" 2
  out="$(cd "$_p" && "$_vsb" validate --strict --json 2>&1)"; rc=$?
  assert_rc "skill-revision/5" "the example validates strictly" 0 "$rc"
  if grep -qE '"check":"SF3[013]"' <<<"$out"; then fail "skill-revision/5" "the example raises no SF30, SF31 or SF33"
  else pass "skill-revision/5" "the example raises no SF30, SF31 or SF33"; fi
  printf -- '---\ntype: tech-topic\nupdated: 2026-10-24\n---\n# Storage\n' > "$_p/docs/spec/tech-storage.md"
  out="$(cd "$_p" && "$_vsb" validate --strict --json 2>&1)"
  assert_contains "skill-revision/5" "control: a root topic file raises SF30" "$out" '"check":"SF30"'
fi

# --- skill-revision/6: one wrap-up sequence; agents that never wait -----------
awk '/^## Wrap-up/{f=1; next} /^## /{f=0} f' "$_S" > "$_sa/wrapup.md"
_n="$(grep -cE '^[0-9]+\. ' "$_sa/wrapup.md")"
assert_eq "skill-revision/6" "§ Wrap-up has 7 numbered steps" 7 "$_n"
_order=""
for _x in 'promote <feature> --dry-run' 'promote <feature> --done' 'check-drift --pre-merge' 'finishing-a-development-branch'; do
  _l="$(grep -nF -- "$_x" "$_sa/wrapup.md" | head -n 1 | cut -d: -f1)"
  _order="$_order ${_l:-0}"
done
_sorted="$(tr ' ' '\n' <<<"$_order" | grep . | sort -n | tr '\n' ' ')"
if [[ "$_order" != *" 0"* && " $_sorted" == "$_order " ]]; then
  pass "skill-revision/6" "§ Wrap-up names dry-run, --done, check-drift --pre-merge, finishing in order (lines$_order)"
else fail "skill-revision/6" "§ Wrap-up names dry-run, --done, check-drift --pre-merge, finishing in order (lines$_order)"; fi
_n="$(awk '/^## Wrap-up/{f=1; next} /^## /{f=0} !f' "$_S" | grep -cF 'superpowers:finishing-a-development-branch')"
assert_eq "skill-revision/6" "SKILL.md names superpowers:finishing-a-development-branch only in § Wrap-up" 0 "$_n"
for _a in spec-promoter spec-health; do
  out="$(lint_description "$REPO_ROOT/spec/agents/$_a.md")"; rc=$?
  assert_rc "skill-revision/6" "$_a description passes the lint ${out:+— $out}" 0 "$rc"
done

rm -rf "$_sa"
