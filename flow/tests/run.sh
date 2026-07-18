#!/usr/bin/env bash
# flow/tests/run.sh — behaviour tests for the vibe-flow core: the state machine,
# set-state.sh / validate-state.sh, D12 orders (orders.sh + per-skill blocks),
# and graceful degradation (check-skills.sh). Pure bash; no bats.
#
# Each test cites its plan unit ID (vibe-flow/n). Exit 0 = all pass.
#
# The suite is hermetic: it builds one throwaway sandbox (mktemp -d) that mirrors
# the repo's flow/ and runs every cursor-touching script inside it. The live repo
# cursor (flow/state.json) is never read or written, so concurrent runs never race
# and a pre-existing cursor is left byte-identical.

# The `cond && pass || fail` reporting idiom is intentional and safe here:
# pass()/fail() always return 0, so fail never runs spuriously after pass.
# shellcheck disable=SC2015
set -uo pipefail

# Locate the real repo (SRC_ROOT) by upward marker search — physical path so real
# and symlinked invocations converge. SRC_ROOT is used read-only: to seed the
# sandbox and to run install.sh / the dogfood health check against the real tree.
_find_repo_root() {
  local d; d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  while [[ "$d" != "/" ]]; do
    [[ -d "$d/.spec" || -e "$d/.git" ]] && { printf '%s\n' "$d"; return 0; }
    d="$(dirname "$d")"
  done
  return 1
}
SRC_ROOT="$(_find_repo_root)" || { echo "cannot locate repo root (.spec/.git)" >&2; exit 1; }

# One hermetic sandbox for the whole suite. It mirrors the real flow/ so every
# self-locating script resolves its state.json, machine, and orders INTO the
# sandbox — the live cursor is never touched. The layout preserves the
# real-vs-symlink path-parity subject the parity tests exercise: flow/ is a real
# copy, .agents/skills/vibe is the recreated ../../flow symlink (NOT a cp -RL
# deref), and .spec is a root marker so marker-search halts here instead of
# walking up into the real repo. mktemp -d is unique per run, so two concurrent
# suites never share state. The live flow/state.json is excluded from the copy
# (never read); tests seed the sandbox cursor from state.example.json as needed.
SANDBOX="$(mktemp -d)"
cleanup() { rm -rf "$SANDBOX"; }
trap cleanup EXIT
mkdir -p "$SANDBOX/flow" "$SANDBOX/.agents/skills" "$SANDBOX/.spec"
find "$SRC_ROOT/flow" -mindepth 1 -maxdepth 1 ! -name state.json \
  -exec cp -R {} "$SANDBOX/flow/" \;
ln -s ../../flow "$SANDBOX/.agents/skills/vibe"

# All repo-relative test paths now resolve inside the sandbox.
REPO_ROOT="$SANDBOX"
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

# orders.sh no-jq parity (review-fix): the jq-less inject path is the missing third
# leg (set-state + detect-context already degrade). Its output MUST be byte-
# identical to the jq path for the cursor-driven and explicit cases below — pre-fix
# the no-jq run printed the generic 'state=unknown' fallback for every state. Build
# a jq-free PATH carrying only the coreutils the no-jq path uses (no jq, no awk).
pnojq="$(mktemp -d)"
for t in dirname sed head cat; do ln -sf "$(command -v "$t")" "$pnojq/$t"; done
orders_parity() {
  local label="$1"; shift
  local a b
  a="$(bash "$SCRIPTS/orders.sh" "$@" 2>/dev/null)"
  b="$(PATH="$pnojq" "$BASH_BIN" "$SCRIPTS/orders.sh" "$@" 2>/dev/null)"
  assert_eq "vibe-flow/1" "orders.sh jq/no-jq byte-identical — $label" "$b" "$a"
}
rm -f "$STATE"
orders_parity "idle (no cursor)"
cp "$FLOW/state.example.json" "$STATE"; bash "$SCRIPTS/set-state.sh" idle >/dev/null
orders_parity "idle (cursor)"
bash "$SCRIPTS/set-state.sh" feature.impl demo >/dev/null
orders_parity "feature.impl (feature=demo from cursor)"
# ...and the interpolation actually happened on the no-jq path (demo, no placeholder).
nojq_impl="$(PATH="$pnojq" "$BASH_BIN" "$SCRIPTS/orders.sh" 2>/dev/null)"
assert_contains "vibe-flow/1" "no-jq orders interpolate cursor feature (demo)" "$nojq_impl" "demo/n"
assert_not_contains "vibe-flow/1" "no-jq orders leave no <feature> placeholder" "$nojq_impl" "<feature>/n"
assert_not_contains "vibe-flow/1" "no-jq orders do not degrade to state=unknown" "$nojq_impl" "state=unknown"
orders_parity "quick.verify (explicit arg)" quick.verify
rm -f "$STATE"; rm -rf "$pnojq"

echo ""
echo "=== vibe-flow/3 — graceful skill degradation ==="
out="$(bash "$SCRIPTS/check-skills.sh" feature.design 2>&1; echo "rc=$?")"
assert_contains "vibe-flow/3" "check-skills warns on assumed-installed superpowers" "$out" "superpowers:brainstorming"
assert_contains "vibe-flow/3" "check-skills never hard-fails (exit 0)" "$out" "rc=0"
out="$(bash "$SCRIPTS/check-skills.sh" setup.detect)"
assert_contains "vibe-flow/3" "check-skills confirms bundled spec when delegated" "$(bash "$SCRIPTS/check-skills.sh" strategy.spec 2>&1)" "spec"

echo ""
echo "=== set-state.sh — writer, not gate ==="
cp "$FLOW/state.example.json" "$STATE"
out="$(bash "$SCRIPTS/set-state.sh" bogus.state 2>&1; echo "rc=$?")"
assert_contains "vibe-flow/core" "set-state.sh rejects unknown state" "$out" "not a known state"
# amend is no longer a known state (folded into precedence) — rejected as unknown.
out="$(bash "$SCRIPTS/set-state.sh" amend 2>&1; echo "rc=$?")"
assert_contains "vibe-flow/core" "set-state.sh rejects amend as an unknown state" "$out" "not a known state"
assert_not_contains "vibe-flow/core" "set-state.sh does not accept amend (rc!=0)" "$out" "rc=0"
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

# flow-mvp/3 + simplify — gates (edges), abort edges, dead-state removal, router hygiene.
# gates keys parse as <state>><state> with both states known and the target
# present in the source state's `next` array.
gate_count="$(jq -r '(.gates // {}) | keys | length' "$MACHINE")"
assert_eq "flow-mvp/3" "gates field carries the two gated edges" "$gate_count" "2"
bad_gates="$(jq -r '
  .states as $s
  | (.gates // {}) | keys[]
  | . as $edge
  | ($edge | split(">")) as $p
  | select(
      ($p | length) != 2
      or (($s | has($p[0])) | not)
      or (($s | has($p[1])) | not)
      or (([$s[$p[0]].next[]?] | index($p[1])) == null)
    )
' "$MACHINE" | paste -sd, -)"
assert_eq "flow-mvp/3" "every gates key is <known-state>><known-state> with target in source.next" "$bad_gates" ""

# Abort edges: `set-state.sh idle` is always legal (SKILL.md precedence), so idle must
# be in `next` for EVERY non-idle state — not a hand-picked sample. Iterating all states
# makes this fail if the abort edge is dropped from any single one.
non_idle_states="$(jq -r '.states | keys[] | select(. != "idle")' "$MACHINE")"
abort_missing=""
while IFS= read -r st; do
  [[ -z "$st" ]] && continue
  has_idle="$(jq -r --arg s "$st" '[.states[$s].next[]?] | index("idle") != null' "$MACHINE")"
  [[ "$has_idle" == "true" ]] || abort_missing="$abort_missing $st"
done <<< "$non_idle_states"
assert_eq "flow-mvp/3" "idle is an abort edge from every non-idle flow state" "$abort_missing" ""
# Coverage guard: the iteration is non-trivial and DOES include the three states that
# previously lacked the abort edge (so a regression that drops them from the sweep fails).
covered_count="$(printf '%s\n' "$non_idle_states" | grep -c .)"
[[ "$covered_count" -ge 10 ]] && pass "flow-mvp/3" "abort-edge sweep covers all $covered_count non-idle flow states" || fail "flow-mvp/3" "abort-edge sweep covered too few states ($covered_count)"
assert_contains "flow-mvp/3" "abort-edge sweep includes feature.plan (previously missing idle)" "$non_idle_states" "feature.plan"
assert_contains "flow-mvp/3" "abort-edge sweep includes quick.fix (previously missing idle)" "$non_idle_states" "quick.fix"
assert_contains "flow-mvp/3" "abort-edge sweep includes quick.triage (previously missing idle)" "$non_idle_states" "quick.triage"

# Dead compound states are gone: strategy.spec and quick.verify end the flow directly.
# The removed state names are assembled from parts so this regression guard does not
# itself trip the repo-wide "no dead-state references outside the archive" check.
dead_suffix="compound"
dead_sc="strategy.$dead_suffix"
dead_qc="quick.$dead_suffix"
gone_states="$(jq -r --arg sc "$dead_sc" --arg qc "$dead_qc" '[.states | keys[] | select(. == $sc or . == $qc or . == "amend")] | join(",")' "$MACHINE")"
assert_eq "simplify/dead-states" "the two per-phase compound states and amend are removed from the machine" "$gone_states" ""
# flow-legibility/1 — loop edges + research artifact
ss_next="$(jq -c '.states."strategy.spec".next' "$MACHINE")"
assert_eq "flow-legibility/1" "strategy.spec loops back to brainstorm: next is [strategy.brainstorm, idle]" "$ss_next" '["strategy.brainstorm","idle"]'
ss_no_compound="$(jq -r --arg sc "$dead_sc" '[.states."strategy.spec".next[]] | index($sc) == null' "$MACHINE")"
assert_eq "simplify/dead-states" "strategy.spec.next carries no dead compound state" "$ss_no_compound" "true"
fp_design="$(jq -r '[.states."feature.plan".next[]] | index("feature.design") != null' "$MACHINE")"
assert_eq "flow-legibility/1" "feature.plan loops back to feature.design" "$fp_design" "true"
fp_impl="$(jq -r '[.states."feature.plan".next[]] | index("feature.impl") != null' "$MACHINE")"
assert_eq "flow-legibility/1" "feature.plan still routes to feature.impl (gate intact)" "$fp_impl" "true"
fd_research="$(jq -r '[.states."feature.design".writes[]] | any(test("research\\.md"))' "$MACHINE")"
assert_eq "flow-legibility/1" "research.md is a first-class feature.design write" "$fd_research" "true"
qv_ok="$(jq -r --arg qc "$dead_qc" '[.states."quick.verify".next[]] | (index("quick.fix") != null) and (index("idle") != null) and (index($qc) == null)' "$MACHINE")"
assert_eq "simplify/dead-states" "quick.verify.next is quick.fix + idle, no dead compound state" "$qv_ok" "true"
# No per-state caveman field survives; a single top-level style note replaces them.
cav_left="$(jq -r '[.states | to_entries[] | select(.value | has("caveman"))] | length' "$MACHINE")"
assert_eq "simplify/style" "no state carries a caveman field" "$cav_left" "0"
has_style="$(jq -r 'has("style")' "$MACHINE")"
assert_eq "simplify/style" "machine carries a top-level style note" "$has_style" "true"
no_cav_levels="$(jq -r '(has("caveman_levels") or has("safety_carveouts") or has("modifiers"))' "$MACHINE")"
assert_eq "simplify/style" "caveman_levels, safety_carveouts, and modifiers are removed" "$no_cav_levels" "false"

# Router hygiene: idle drops its router delegate; setup.apply keeps only spec.
idle_deleg="$(jq -c '.states.idle.delegates' "$MACHINE")"
assert_eq "flow-mvp/3" "idle delegates array is empty" "$idle_deleg" "[]"
setup_deleg="$(jq -c '.states."setup.apply".delegates' "$MACHINE")"
assert_eq "flow-mvp/3" "setup.apply delegates is exactly [spec]" "$setup_deleg" '["spec"]'

echo ""
echo "=== flow-mvp/5,6 — machine delegates ⊆ phase-file prose ==="
# Subagents receive no per-turn orders, so every delegate's contract must live in
# the phase file that documents its state. For each state with a non-empty
# delegates array, assert each delegate name appears verbatim in the mapped phase
# file. Suffix rules (*.verify, *.compound) win over the flow prefix so verify.md
# and compound.md own those states.
phase_file_for() {
  case "$1" in
    *.verify)   echo "verify.md" ;;
    *.compound) echo "compound.md" ;;
    setup.*)    echo "setup.md" ;;
    strategy.*) echo "strategy.md" ;;
    feature.*)  echo "feature.md" ;;
    quick.*)    echo "quick.md" ;;
    *)          echo "" ;;
  esac
}
deleg_ok=1
while IFS= read -r st; do
  [[ -z "$st" ]] && continue
  pf="$(phase_file_for "$st")"
  if [[ -z "$pf" || ! -f "$FLOW/$pf" ]]; then
    deleg_ok=0; echo "        no phase file mapped for $st"; continue
  fi
  body="$(cat "$FLOW/$pf")"
  while IFS= read -r dg; do
    [[ -z "$dg" ]] && continue
    [[ "$body" == *"$dg"* ]] || { deleg_ok=0; echo "        $st: delegate '$dg' missing from $pf"; }
  done < <(jq -r --arg s "$st" '.states[$s].delegates[]?' "$MACHINE")
done < <(jq -r '.states | to_entries[] | select((.value.delegates | length) > 0) | .key' "$MACHINE")
assert_eq "flow-mvp/5" "every machine delegate appears verbatim in its phase file" "$deleg_ok" "1"

# quick.md carries the inline conditional lesson step (the optional quick-fix lesson
# now lives inline in quick.verify, not a separate per-phase compound state): it appends
# to .spec/lessons.md and refreshes the digest before going idle.
quick_body="$(cat "$FLOW/quick.md")"
assert_contains "simplify/dead-states" "quick.md carries the inline lesson step (lessons.md)" "$quick_body" ".spec/lessons.md"
assert_contains "simplify/dead-states" "quick.md refreshes the digest inline" "$quick_body" "regen-active-rules.sh"
assert_not_contains "simplify/dead-states" "quick.md no longer names the dead quick compound state" "$quick_body" "$dead_qc"

# Router hygiene: the compound row serves feature.compound only; the quick row lists
# quick.verify and no longer claims a dead per-phase compound state anywhere.
SKILL_MD="$FLOW/SKILL.md"
quick_router_row="$(grep -F '](quick.md)' "$SKILL_MD" | head -1)"
compound_router_row="$(grep -F '](compound.md)' "$SKILL_MD" | head -1)"
assert_contains "simplify/dead-states" "SKILL.md quick row lists quick.verify" "$quick_router_row" "quick.verify"
assert_not_contains "simplify/dead-states" "SKILL.md quick row no longer claims the dead quick compound state" "$quick_router_row" "$dead_qc"
assert_contains "simplify/dead-states" "SKILL.md compound row lists feature.compound" "$compound_router_row" "feature.compound"
assert_not_contains "simplify/dead-states" "SKILL.md compound row no longer claims the dead strategy compound state" "$compound_router_row" "$dead_sc"
assert_not_contains "simplify/dead-states" "SKILL.md compound row no longer claims the dead quick compound state" "$compound_router_row" "$dead_qc"

# compound.md no longer implies finishing-a-development-branch performs the archive
# move: the finishing delegate block is sequenced AFTER the Archive step.
comp="$FLOW/compound.md"
arch_ln="$(grep -n '\*\*Archive' "$comp" | head -1 | cut -d: -f1)"
fin_ln="$(grep -n 'finishing-a-development-branch' "$comp" | head -1 | cut -d: -f1)"
if [[ -n "$arch_ln" && -n "$fin_ln" && "$fin_ln" -gt "$arch_ln" ]]; then
  pass "flow-mvp/6" "compound.md sequences finishing after the archive step"
else
  fail "flow-mvp/6" "compound.md sequences finishing after the archive step"
  echo "        archive line: ${arch_ln:-none}, finishing line: ${fin_ln:-none}"
fi

echo ""
echo "=== flow-mvp/7 — auto-advance: gate markers <-> machine gates, orders byte budget ==="
# Auto-advance is prose; the only stop-and-ask points are the two `gates` edges in
# the machine. Their SOURCE states must carry a `gate:` marker in their orders block,
# and no other state may. Extraction is the suite's own orders.sh; with the cursor
# removed the <feature> placeholder stays literal, so the byte measurement is stable.
rm -f "$STATE"
gate_sources="$(jq -r '(.gates // {}) | keys[] | split(">")[0]' "$MACHINE" | sort -u)"

# Each gates-edge source state carries `gate:` in its orders block.
gate_marker_ok=1
while IFS= read -r gs; do
  [[ -z "$gs" ]] && continue
  blk="$(bash "$SCRIPTS/orders.sh" "$gs")"
  [[ "$blk" == *"gate:"* ]] || { gate_marker_ok=0; echo "        gate source $gs orders block lacks 'gate:'"; }
done <<< "$gate_sources"
assert_eq "flow-mvp/7" "every gates-edge source state carries 'gate:' in its orders" "$gate_marker_ok" "1"

# No OTHER state's orders block carries `gate:`.
extra_gate=""
while IFS= read -r st; do
  [[ -z "$st" ]] && continue
  printf '%s\n' "$gate_sources" | grep -qxF "$st" && continue
  blk="$(bash "$SCRIPTS/orders.sh" "$st")"
  [[ "$blk" == *"gate:"* ]] && extra_gate="$extra_gate $st"
done <<< "$(jq -r '.states | keys[]' "$MACHINE")"
assert_eq "flow-mvp/7" "no non-gate state carries 'gate:' in its orders" "$extra_gate" ""

# Every orders block resolves within the 400-byte budget.
over_budget=""
while IFS= read -r st; do
  [[ -z "$st" ]] && continue
  blk="$(bash "$SCRIPTS/orders.sh" "$st")"
  n="$(printf '%s' "$blk" | wc -c | tr -d ' ')"
  [[ "$n" -le 400 ]] || over_budget="$over_budget $st($n)"
done <<< "$(jq -r '.states | keys[]' "$MACHINE")"
assert_eq "flow-mvp/7" "every orders block is within the 400-byte budget" "$over_budget" ""

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
# flow-mvp/10 — caveman is demoted out of deps.json entirely (vibe vocabulary,
# not an external dependency); superpowers + feature-dev remain.
dep_names="$(jq -r '[.deps[].name] | join(",")' "$DEPS")"
assert_not_contains "flow-mvp/10" "deps.json has no caveman entry" "$dep_names" "caveman"
assert_contains "flow-mvp/10" "deps.json still declares superpowers" "$dep_names" "superpowers"
assert_contains "flow-mvp/10" "deps.json still declares feature-dev" "$dep_names" "feature-dev"
# Healthy dogfood repo: the sandbox lacks .claude/spec, so this check runs the
# (byte-identical) sandbox doctor against the real SRC_ROOT explicitly. doctor is
# read-only — it never writes the repo or its cursor.
out="$(bash "$DOCTOR" "$SRC_ROOT" 2>&1; echo "rc=$?")"
assert_contains "install-tooling/4" "doctor exits 0 on a healthy repo" "$out" "rc=0"
assert_contains "install-tooling/4" "doctor reports core.vibe ok" "$out" "ok   core.vibe"
assert_contains "install-tooling/4" "doctor reports machine ok" "$out" "ok   machine"
assert_contains "install-tooling/4" "doctor reports each adapter hook script present" "$out" "ok   adapter.script.stop-gate.sh"
# The source repo dogfoods its own hooks: .claude/settings.json wires all three,
# so doctor reports activation ok. (The unwired WARN scenario is covered below via
# a fresh install with settings.json removed.)
assert_contains "install-tooling/4" "doctor reports adapter.activation ok in the dogfood source repo" "$out" "ok   adapter.activation"
assert_contains "install-tooling/4" "doctor lists dep superpowers" "$out" "dep.superpowers"
assert_contains "install-tooling/4" "doctor lists dep feature-dev" "$out" "dep.feature-dev"
assert_not_contains "flow-mvp/10" "doctor no longer lists dep.caveman" "$out" "dep.caveman"
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
# No-jq target with a VALID cursor: validate-state.sh needs jq, so it would exit 1
# and doctor used to mislabel the cursor "present but invalid — reseed". It must
# instead report an unverified OK, never advise reseeding, and still exit 0.
# Discriminating: the old code emitted "warn cursor" + "invalid" here.
nojq_doc="$(mktemp -d)"
for t in dirname readlink grep find sed head cat env; do
  _p="$(command -v "$t" 2>/dev/null)" && ln -sf "$_p" "$nojq_doc/$t"
done
out="$(PATH="$nojq_doc" "$BASH_BIN" "$DOCTOR" "$d" 2>&1; echo "rc=$?")"
assert_contains "review-fix" "no-jq doctor reports a valid cursor as ok (unverified)" "$out" "ok   cursor"
assert_contains "review-fix" "no-jq doctor marks the cursor unverified (jq missing)" "$out" "cursor present (unverified"
assert_not_contains "review-fix" "no-jq doctor does not warn the valid cursor invalid" "$out" "warn cursor"
assert_not_contains "review-fix" "no-jq doctor does not advise reseeding a valid cursor" "$out" "reseed"
assert_contains "review-fix" "no-jq doctor still exits 0" "$out" "rc=0"
rm -rf "$nojq_doc"
rm -rf "$d"
# Absent dep: reported as a warn with its degrade text, still exit 0.
d="$(mktemp -d)"; mkdir -p "$d/.spec" "$d/.agents/skills/vibe/reference"
printf '{"deps":[{"name":"vibe-nonexistent-dep-xyz","kind":"skill-collection","source":"x","required_by":["*"],"degrade":"inline fallback"}]}\n' > "$d/.agents/skills/vibe/reference/deps.json"
out="$(bash "$DOCTOR" "$d" 2>&1; echo "rc=$?")"
assert_contains "install-tooling/4" "doctor warns an absent dep with degrade text" "$out" "degrade: inline fallback"
assert_contains "install-tooling/4" "doctor exits 0 with a missing dep" "$out" "rc=0"
rm -rf "$d"

echo ""
echo "=== install-tooling/4 — doctor adapter.activation (settings.json wiring) ==="
# A full install wires the three hooks into settings.json — doctor reports ok.
SBA="$(mktemp -d)"
bash "$SRC_ROOT/install.sh" "$SBA" >/dev/null 2>&1
out="$(bash "$DOCTOR" "$SBA" 2>&1; echo "rc=$?")"
assert_contains "install-tooling/4" "doctor reports adapter.activation ok when settings.json wires the hooks" "$out" "ok   adapter.activation"
assert_contains "install-tooling/4" "doctor exits 0 on a wired install" "$out" "rc=0"
# Remove only the wiring (scripts stay): doctor must WARN, name the gap, exit 0.
rm -f "$SBA/.claude/settings.json"
out="$(bash "$DOCTOR" "$SBA" 2>&1; echo "rc=$?")"
assert_contains "install-tooling/4" "doctor still reports the hook scripts present" "$out" "ok   adapter.script.stop-gate.sh"
assert_contains "install-tooling/4" "doctor warns adapter.activation when scripts exist but settings.json does not wire them" "$out" "warn adapter.activation"
assert_contains "install-tooling/4" "doctor exits 0 when hooks are unwired" "$out" "rc=0"
rm -rf "$SBA"

echo ""
echo "=== detect-context.sh — lessons.md write policy ==="
DETECT="$SCRIPTS/detect-context.sh"
out="$(bash "$DETECT" decide .spec/lessons.md setup.apply)"
assert_eq "vibe-flow/core" "decide lessons.md returns allow under setup.apply" "$out" "allow"
out="$(bash "$DETECT" decide .spec/lessons.md feature.compound)"
assert_eq "vibe-flow/core" "decide lessons.md returns allow under feature.compound" "$out" "allow"
# The conditional lesson step now lives in strategy.spec and quick.verify — both allow.
out="$(bash "$DETECT" decide .spec/lessons.md strategy.spec)"
assert_eq "simplify/dead-states" "decide lessons.md returns allow under strategy.spec" "$out" "allow"
out="$(bash "$DETECT" decide .spec/lessons.md quick.verify)"
assert_eq "simplify/dead-states" "decide lessons.md returns allow under quick.verify" "$out" "allow"
out="$(bash "$DETECT" decide .spec/lessons.md idle)"
assert_contains "vibe-flow/core" "decide lessons.md blocks under idle" "$out" "block:"
# Discriminating: the removed states must not be special-cased any more, and a
# non-lesson state (quick.fix) still blocks — the allow-list did not over-widen.
out="$(bash "$DETECT" decide .spec/lessons.md quick.fix)"
assert_contains "simplify/dead-states" "decide lessons.md blocks under quick.fix" "$out" "block:"

echo ""
echo "=== detect-context.sh — verify writes no src, spec frozen in impl ==="
# Verify states write no src/tests: decide returns a warn that routes findings back to
# the fix state, NOT allow. Discriminating — asserts warn AND the correct route target,
# so reverting to the old `allow` (or mis-routing quick.verify to feature.impl) fails.
out="$(bash "$DETECT" decide src/app.js feature.verify)"
assert_contains "flow-mvp/3" "decide src warns under feature.verify" "$out" "warn:"
assert_not_contains "flow-mvp/3" "decide src does not allow under feature.verify" "$out" "allow"
assert_contains "flow-mvp/3" "feature.verify src warn routes back to feature.impl" "$out" "set-state.sh feature.impl"
out="$(bash "$DETECT" decide tests/app_test.js feature.verify)"
assert_contains "flow-mvp/3" "decide tests warns under feature.verify" "$out" "warn:"
out="$(bash "$DETECT" decide src/app.js quick.verify)"
assert_contains "flow-mvp/3" "decide src warns under quick.verify" "$out" "warn:"
assert_not_contains "flow-mvp/3" "decide src does not allow under quick.verify" "$out" "allow"
assert_contains "flow-mvp/3" "quick.verify src warn routes back to quick.fix" "$out" "set-state.sh quick.fix"
# Regression guard: the impl/fix states still allow src writes (the warn didn't over-reach).
out="$(bash "$DETECT" decide src/app.js feature.impl)"
assert_eq "flow-mvp/3" "decide src still allows under feature.impl" "$out" "allow"
out="$(bash "$DETECT" decide src/app.js quick.fix)"
assert_eq "flow-mvp/3" "decide src still allows under quick.fix" "$out" "allow"

# Feature specs are frozen once impl/fix begins: decide warns instead of silently
# allowing (feature.md forbids .spec edits in impl). Design/plan (and setup) still author.
out="$(bash "$DETECT" decide .spec/features/widget/tech.md feature.impl)"
assert_contains "flow-mvp/3" "decide .spec/features warns under feature.impl" "$out" "warn:"
assert_not_contains "flow-mvp/3" "decide .spec/features does not allow under feature.impl" "$out" "allow"
out="$(bash "$DETECT" decide .spec/features/widget/product.md quick.fix)"
assert_contains "flow-mvp/3" "decide .spec/features warns under quick.fix" "$out" "warn:"
out="$(bash "$DETECT" decide .spec/features/widget/tech.md feature.design)"
assert_eq "flow-mvp/3" "decide .spec/features still allows under feature.design" "$out" "allow"
out="$(bash "$DETECT" decide .spec/features/widget/plan.md feature.plan)"
assert_eq "flow-mvp/3" "decide .spec/features still allows under feature.plan" "$out" "allow"

echo ""
echo "=== detect-context.sh — no-jq cursor read stays state-aware ==="
# Without jq, current_state() falls back to sed over the machine-written cursor.
# Discriminating: pre-fix code degraded every state to idle, so decide with NO
# explicit state hard-blocked root-spec writes even in strategy.spec on jq-less
# targets — the opposite of "jq recommended, not required".
bash "$SCRIPTS/set-state.sh" strategy.spec >/dev/null
nojq="$(mktemp -d)"
for t in dirname sed head; do ln -sf "$(command -v "$t")" "$nojq/$t"; done
out="$(PATH="$nojq" "$BASH_BIN" "$DETECT" decide .spec/product.md)"
assert_eq "review-fix" "no-jq decide honors the live cursor (allow in strategy.spec)" "$out" "allow"
out="$(PATH="$nojq" "$BASH_BIN" "$DETECT" decide .spec/lessons.md)"
assert_eq "review-fix" "no-jq decide honors the live cursor (lessons.md allow in strategy.spec)" "$out" "allow"
# Discriminating on the BLOCK path: a state where lessons.md is NOT allowed must
# block via the no-jq (sed) cursor read AND name the real state, not degrade to idle.
bash "$SCRIPTS/set-state.sh" feature.impl demo >/dev/null
out="$(PATH="$nojq" "$BASH_BIN" "$DETECT" decide .spec/lessons.md)"
assert_contains "review-fix" "no-jq decide still blocks lessons.md in feature.impl" "$out" "block:"
assert_contains "review-fix" "no-jq block message names the real state, not idle" "$out" "current: feature.impl"
rm -rf "$nojq"
bash "$SCRIPTS/set-state.sh" idle >/dev/null

echo ""
echo "=== /flow command — feature arg, idle-always-legal, gate token ==="
# The /flow command is LLM-executed prose (read from the real repo, not the sandbox).
# It must (a) forward an optional feature to set-state.sh, (b) treat idle as always
# legal, (c) enforce the two gated edges via an explicit `confirm` token. These doc
# assertions fail if any of those contracts is dropped from the command file.
FLOWCMD="$SRC_ROOT/.claude/commands/flow.md"
if [[ -f "$FLOWCMD" ]]; then
  cmd="$(cat "$FLOWCMD")"
  assert_contains "flow-mvp/3" "/flow forwards an optional feature to set-state.sh" "$cmd" "set-state.sh <target> [feature]"
  assert_contains "flow-mvp/3" "/flow treats idle as always legal (skips membership + gate)" "$cmd" "Abort is always legal"
  assert_contains "flow-mvp/3" "/flow enforces gated edges against the machine gates object" "$cmd" "gates"
  assert_contains "flow-mvp/3" "/flow documents the confirm approval token" "$cmd" "confirm"
  assert_contains "flow-mvp/3" "/flow argument-hint advertises feature + confirm" "$cmd" "[feature] [confirm]"
else
  fail "flow-mvp/3" "/flow command file present at .claude/commands/flow.md"
fi
# The machine still declares exactly the two gated edges the command enforces.
gate_edge_count="$(jq -r '(.gates // {}) | keys | length' "$MACHINE")"
assert_eq "flow-mvp/3" "machine declares exactly the two gated edges the command enforces" "$gate_edge_count" "2"
# SKILL.md precedence documents the same confirm token (fix 5c).
assert_contains "flow-mvp/3" "SKILL.md precedence documents the /flow confirm token" "$(cat "$FLOW/SKILL.md")" "confirm"

echo ""
echo "=== orders.sh on a fresh non-git install (stranger-eval regression) ==="
# A fresh full install creates neither .git nor .spec. orders.sh must still
# resolve the linked skill's block by self-locating the skills dir from its own
# path — not degrade to 'state=unknown' for want of a repo-root marker.
SBI="$(mktemp -d)"
bash "$SRC_ROOT/install.sh" "$SBI" >/dev/null 2>&1
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
echo "=== flow-mvp/9 — evidence receipt + verify tooth (stop-gate) ==="
# The stop-gate is an adapter under .claude/, invoked directly with a crafted
# CLAUDE_PROJECT_DIR sandbox. The hook reads only: detect-context.sh + machine +
# cursor (for the state/feature) and the receipt file, plus `git status` in the
# project root. Each scenario builds its own throwaway sandbox; the live cursor
# and the shared hermetic sandbox are never touched.
GATE="$SRC_ROOT/.claude/hooks/stop-gate.sh"

# Build a fresh sandbox carrying just what the hook reads: a real detect-context.sh
# + state-machine.json + a crafted cursor. $4=git seeds a git work tree (one
# committed tracked file; cursor + evidence/ gitignored, mirroring the installer).
mk_gate_sbx() {
  local flow="$1" phase="$2" feature="$3" mode="$4"
  local s; s="$(mktemp -d)"
  mkdir -p "$s/.agents/skills/vibe/scripts"
  cp "$SRC_ROOT/flow/scripts/detect-context.sh" "$s/.agents/skills/vibe/scripts/"
  cp "$SRC_ROOT/flow/state-machine.json" "$s/.agents/skills/vibe/"
  local feat_json="null"
  [[ "$feature" != "null" ]] && feat_json="\"$feature\""
  printf '{"flow":"%s","phase":"%s","feature":%s,"updated":"2026-07-08T00:00:00Z"}\n' \
    "$flow" "$phase" "$feat_json" > "$s/.agents/skills/vibe/state.json"
  if [[ "$mode" == "git" ]]; then
    printf '.agents/skills/vibe/state.json\n.agents/skills/vibe/evidence/\n' > "$s/.gitignore"
    printf 'seed\n' > "$s/tracked.txt"
    git -C "$s" init -q
    git -C "$s" config user.email t@t.test
    git -C "$s" config user.name test
    git -C "$s" add -A
    git -C "$s" commit -q -m init
  fi
  printf '%s\n' "$s"
}
run_gate() { printf '%s' "$2" | CLAUDE_PROJECT_DIR="$1" bash "$GATE" 2>&1; echo "rc=$?"; }

# 1) feature.verify, git repo, no receipt -> block (exit 2), names the exact path.
s="$(mk_gate_sbx feature verify widget git)"
out="$(run_gate "$s" '{}')"
assert_contains "flow-mvp/9" "feature.verify with no receipt blocks (exit 2)" "$out" "rc=2"
assert_contains "flow-mvp/9" "missing-receipt block names the exact path" "$out" "evidence/feature-widget.md"
assert_contains "flow-mvp/9" "missing-receipt block names the abort hatch" "$out" "set-state.sh idle"
rm -rf "$s"

# 2) write the receipt, commit so the tree is clean -> pass (exit 0).
s="$(mk_gate_sbx feature verify widget git)"
mkdir -p "$s/.agents/skills/vibe/evidence"
printf 'ran: bash tests; observed: 12 passed\n' > "$s/.agents/skills/vibe/evidence/feature-widget.md"
out="$(run_gate "$s" '{}')"
assert_contains "flow-mvp/9" "feature.verify with a fresh receipt + clean tree passes (exit 0)" "$out" "rc=0"
rm -rf "$s"

# 3) modify a tracked file after the receipt -> block as stale (exit 2). The
# receipt mtime is pinned into the past so the post-receipt edit is deterministically
# newer (no sleep, no 1s mtime-granularity flake).
s="$(mk_gate_sbx feature verify widget git)"
mkdir -p "$s/.agents/skills/vibe/evidence"
printf 'ran: bash tests\n' > "$s/.agents/skills/vibe/evidence/feature-widget.md"
printf 'edited after the receipt\n' >> "$s/tracked.txt"
touch -t 200001010000 "$s/.agents/skills/vibe/evidence/feature-widget.md"
out="$(run_gate "$s" '{}')"
assert_contains "flow-mvp/9" "feature.verify with a stale receipt blocks (exit 2)" "$out" "rc=2"
assert_contains "flow-mvp/9" "stale block names the abort hatch" "$out" "set-state.sh idle"
rm -rf "$s"

# 4) stop_hook_active passes through even with no receipt (no block loops).
s="$(mk_gate_sbx feature verify widget git)"
out="$(run_gate "$s" '{"stop_hook_active": true}')"
assert_contains "flow-mvp/9" "stop_hook_active short-circuits to exit 0" "$out" "rc=0"
rm -rf "$s"

# 5) idle cursor never blocks, regardless of receipts.
s="$(mk_gate_sbx idle idle null git)"
out="$(run_gate "$s" '{}')"
assert_contains "flow-mvp/9" "idle cursor never blocks (exit 0)" "$out" "rc=0"
rm -rf "$s"

# 6) quick.verify: no receipt -> block naming evidence/quick.md; with a receipt and
# a clean tree -> pass.
s="$(mk_gate_sbx quick verify null git)"
out="$(run_gate "$s" '{}')"
assert_contains "flow-mvp/9" "quick.verify with no receipt blocks (exit 2)" "$out" "rc=2"
assert_contains "flow-mvp/9" "quick block names evidence/quick.md" "$out" "evidence/quick.md"
mkdir -p "$s/.agents/skills/vibe/evidence"
printf 'ran: repro; observed: fixed\n' > "$s/.agents/skills/vibe/evidence/quick.md"
out="$(run_gate "$s" '{}')"
assert_contains "flow-mvp/9" "quick.verify with a receipt + clean tree passes (exit 0)" "$out" "rc=0"
rm -rf "$s"

# 7) non-git sandbox, receipt present -> existence-only pass (exit 0).
s="$(mk_gate_sbx feature verify widget nogit)"
mkdir -p "$s/.agents/skills/vibe/evidence"
printf 'ran: bash tests\n' > "$s/.agents/skills/vibe/evidence/feature-widget.md"
out="$(run_gate "$s" '{}')"
assert_contains "flow-mvp/9" "non-git sandbox is existence-only, passes with a receipt (exit 0)" "$out" "rc=0"
rm -rf "$s"

# 8) feature.verify with NO feature in the cursor -> ambiguous receipt path, so the
# gate degrades to warn-only (exit 0) rather than blocking.
s="$(mk_gate_sbx feature verify null git)"
out="$(run_gate "$s" '{}')"
assert_contains "flow-mvp/9" "feature.verify without a feature degrades to warn-only (exit 0)" "$out" "rc=0"
rm -rf "$s"

echo ""
echo "=== review-fix — stop-gate is jq-optional (no-jq PATH shim) ==="
# The blocking tooth (missing/stale receipt) is documented as a HARD block, so it
# must fire with or WITHOUT jq — pre-fix the hook exited 0 the instant jq was absent,
# silently disarming the only Stop-side block. Build a jq-free PATH carrying only the
# coreutils the jq-less hook path uses, then re-run the same scenarios; each outcome
# must match the jq path above. Discriminating: restore `command -v jq || exit 0` and
# every block case below flips to rc=0.
gnojq="$(mktemp -d)"
for _t in dirname sed head cat grep git; do
  _p="$(command -v "$_t" 2>/dev/null)" && ln -sf "$_p" "$gnojq/$_t"
done
run_gate_nojq() { printf '%s' "$2" | PATH="$gnojq" CLAUDE_PROJECT_DIR="$1" "$BASH_BIN" "$GATE" 2>&1; echo "rc=$?"; }

# block-missing-receipt (git repo)
s="$(mk_gate_sbx feature verify widget git)"
out="$(run_gate_nojq "$s" '{}')"
assert_contains "review-fix" "no-jq: feature.verify with no receipt still blocks (exit 2)" "$out" "rc=2"
assert_contains "review-fix" "no-jq: missing-receipt block still names the exact path" "$out" "evidence/feature-widget.md"
rm -rf "$s"

# pass-fresh (receipt + clean tree)
s="$(mk_gate_sbx feature verify widget git)"
mkdir -p "$s/.agents/skills/vibe/evidence"
printf 'ran: bash tests; observed: pass\n' > "$s/.agents/skills/vibe/evidence/feature-widget.md"
out="$(run_gate_nojq "$s" '{}')"
assert_contains "review-fix" "no-jq: fresh receipt + clean tree passes (exit 0)" "$out" "rc=0"
rm -rf "$s"

# block-stale (git repo) — receipt mtime pinned into the past, post-receipt edit newer
s="$(mk_gate_sbx feature verify widget git)"
mkdir -p "$s/.agents/skills/vibe/evidence"
printf 'ran: bash tests\n' > "$s/.agents/skills/vibe/evidence/feature-widget.md"
printf 'edited after the receipt\n' >> "$s/tracked.txt"
touch -t 200001010000 "$s/.agents/skills/vibe/evidence/feature-widget.md"
out="$(run_gate_nojq "$s" '{}')"
assert_contains "review-fix" "no-jq: stale receipt still blocks (exit 2)" "$out" "rc=2"
rm -rf "$s"

# existence-only (no git) -> pass on presence
s="$(mk_gate_sbx feature verify widget nogit)"
mkdir -p "$s/.agents/skills/vibe/evidence"
printf 'ran: bash tests\n' > "$s/.agents/skills/vibe/evidence/feature-widget.md"
out="$(run_gate_nojq "$s" '{}')"
assert_contains "review-fix" "no-jq: non-git existence-only passes with a receipt (exit 0)" "$out" "rc=0"
rm -rf "$s"

# stop_hook_active pass-through (read from stdin via sed, not jq)
s="$(mk_gate_sbx feature verify widget git)"
out="$(run_gate_nojq "$s" '{"stop_hook_active": true}')"
assert_contains "review-fix" "no-jq: stop_hook_active short-circuits to exit 0" "$out" "rc=0"
rm -rf "$s"

# non-verify state (idle) exits 0
s="$(mk_gate_sbx idle idle null git)"
out="$(run_gate_nojq "$s" '{}')"
assert_contains "review-fix" "no-jq: idle cursor never blocks (exit 0)" "$out" "rc=0"
rm -rf "$s"

# quick.verify no receipt -> block naming evidence/quick.md (the second verify state)
s="$(mk_gate_sbx quick verify null git)"
out="$(run_gate_nojq "$s" '{}')"
assert_contains "review-fix" "no-jq: quick.verify with no receipt blocks (exit 2)" "$out" "rc=2"
assert_contains "review-fix" "no-jq: quick block still names evidence/quick.md" "$out" "evidence/quick.md"
rm -rf "$s"
rm -rf "$gnojq"

echo ""
echo "=== review-fix — PreToolUse Bash write sniffer (warn-only) ==="
# The three hard blocks intercept file tools only; a raw `echo >> .spec/lessons.md`
# would slip past undocumented. The guard now warn-sniffs Bash commands. Discriminating:
# with the sniffer removed the Bash payload carries no file_path, so the hook exits 0
# with NO warn — the lessons-write assertion below flips.
GUARD="$SRC_ROOT/.claude/hooks/pre-tool-use-guard.sh"
gsbx="$(mktemp -d)"
mkdir -p "$gsbx/.agents/skills/vibe/scripts"
cp "$SRC_ROOT/flow/scripts/detect-context.sh" "$gsbx/.agents/skills/vibe/scripts/"
cp "$SRC_ROOT/flow/state-machine.json" "$gsbx/.agents/skills/vibe/"
run_guard() { printf '%s' "$2" | CLAUDE_PROJECT_DIR="$1" bash "$GUARD" 2>&1; echo "rc=$?"; }

out="$(run_guard "$gsbx" '{"tool_name":"Bash","tool_input":{"command":"echo x >> .spec/lessons.md"}}')"
assert_contains "review-fix" "bash 'echo >> .spec/lessons.md' warns" "$out" "warn"
assert_contains "review-fix" "bash lessons-write warn names lessons.md" "$out" ".spec/lessons.md"
assert_contains "review-fix" "bash write sniffer never blocks (exit 0)" "$out" "rc=0"

out="$(run_guard "$gsbx" '{"tool_name":"Bash","tool_input":{"command":"grep foo .spec/lessons.md"}}')"
assert_not_contains "review-fix" "bash 'grep .spec/lessons.md' (a read) does NOT warn" "$out" "warn"
assert_contains "review-fix" "bash read exits 0" "$out" "rc=0"

out="$(run_guard "$gsbx" '{"tool_name":"Bash","tool_input":{"command":"bash flow/scripts/set-state.sh idle"}}')"
assert_not_contains "review-fix" "bash set-state.sh idle does NOT warn for state.json" "$out" "warn"
assert_contains "review-fix" "bash set-state.sh exits 0" "$out" "rc=0"

# state.json class + set-state carve-out: a raw redirect INTO state.json (no
# set-state.sh) warns; the same target alongside set-state.sh does not.
out="$(run_guard "$gsbx" '{"tool_name":"Bash","tool_input":{"command":"echo x | tee .agents/skills/vibe/state.json"}}')"
assert_contains "review-fix" "bash 'tee state.json' (no set-state) warns" "$out" "warn"
out="$(run_guard "$gsbx" '{"tool_name":"Bash","tool_input":{"command":"set-state.sh feature.impl > flow/state.json"}}')"
assert_not_contains "review-fix" "bash redirect to state.json via set-state.sh does NOT warn" "$out" "warn"

# a root-spec in-place edit warns; sed -i is a write op.
out="$(run_guard "$gsbx" '{"tool_name":"Bash","tool_input":{"command":"sed -i s/a/b/ .spec/product.md"}}')"
assert_contains "review-fix" "bash 'sed -i .spec/product.md' warns (root spec)" "$out" "warn"

# Edit-tool behavior is unchanged: lessons.md under idle still HARD blocks (exit 2).
out="$(run_guard "$gsbx" '{"tool_name":"Edit","tool_input":{"file_path":".spec/lessons.md"}}')"
assert_contains "review-fix" "Edit-tool lessons.md under idle still hard-blocks (exit 2)" "$out" "rc=2"
assert_contains "review-fix" "Edit-tool behavior unchanged (BLOCKED message)" "$out" "BLOCKED"

# graceful degrade: empty stdin exits 0 with no warn.
out="$(printf '' | CLAUDE_PROJECT_DIR="$gsbx" bash "$GUARD" 2>&1; echo "rc=$?")"
assert_contains "review-fix" "empty stdin exits 0 (graceful)" "$out" "rc=0"
assert_not_contains "review-fix" "empty stdin does not warn" "$out" "warn"
rm -rf "$gsbx"

echo ""
echo "=== review-fix — merge-settings.sh Bash matcher + idempotency ==="
MS="$SRC_ROOT/flow/scripts/merge-settings.sh"
guard_group='[.hooks.PreToolUse[] | select([.hooks[]?.command // empty] | any(test("pre-tool-use-guard")))]'
# The shipped adapter wires Bash into the PreToolUse matcher (file tools kept).
shipped_m="$(jq -r '.hooks.PreToolUse[0].matcher' "$SRC_ROOT/.claude/settings.json")"
assert_contains "review-fix" "shipped .claude/settings.json PreToolUse matcher includes Bash" "$shipped_m" "Bash"
assert_contains "review-fix" "shipped matcher keeps the file tools" "$shipped_m" "Edit|Write|NotebookEdit"

mt="$(mktemp -d)"
bash "$MS" merge "$mt" >/dev/null 2>&1
pm="$(jq -r '.hooks.PreToolUse[0].matcher' "$mt/.claude/settings.json")"
assert_contains "review-fix" "merge writes a PreToolUse matcher including Bash" "$pm" "Bash"
# idempotency: re-merge must not duplicate the vibe PreToolUse group.
bash "$MS" merge "$mt" >/dev/null 2>&1
n="$(jq -r "$guard_group | length" "$mt/.claude/settings.json")"
assert_eq "review-fix" "re-merge does not duplicate the vibe PreToolUse group" "$n" "1"
# an OLD-matcher vibe entry is REPLACED (strip-by-command-path), not appended.
jq '.hooks.PreToolUse = [{"matcher":"Edit|Write|NotebookEdit","hooks":[{"type":"command","command":"bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/pre-tool-use-guard.sh\"","timeout":10}]}]' \
   "$mt/.claude/settings.json" > "$mt/s.tmp" && mv "$mt/s.tmp" "$mt/.claude/settings.json"
bash "$MS" merge "$mt" >/dev/null 2>&1
n2="$(jq -r "$guard_group | length" "$mt/.claude/settings.json")"
pm2="$(jq -r "$guard_group"' | .[0].matcher' "$mt/.claude/settings.json")"
assert_eq "review-fix" "re-merge over an old-matcher entry leaves exactly one vibe group" "$n2" "1"
assert_contains "review-fix" "re-merge upgrades the old matcher to include Bash" "$pm2" "Bash"
rm -rf "$mt"

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
