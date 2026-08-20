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

# Hermeticity: scrub the ambient CLAUDE_PROJECT_DIR before any fixture runs.
# doctrine.sh and detect-context.sh honour it BY DESIGN (the plugin-mode
# contract), so inheriting it points them at the developer's real repo and its
# live cursor instead of the sandbox below. Every Claude Code session exports
# it, which is why local runs and CI diverged: two doctrine fixtures failed for
# anyone running the suite from an agent session while CI, which sets nothing,
# stayed green. Fixtures that WANT the variable set it per invocation
# (`CLAUDE_PROJECT_DIR="$projd" bash …`), so nothing here needs it inherited.
unset CLAUDE_PROJECT_DIR

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

# flow-legibility/2 — self-carrying imperative orders: every block names its own
# transition command (not just a `next:` label). Non-gated states carry
# `set-state.sh <next>`; gated-source states carry the `/flow <next> confirm` gate.
imperative_missing=""
while IFS= read -r st; do
  [[ -z "$st" ]] && continue
  blk="$(bash "$SCRIPTS/orders.sh" "$st")"
  if printf '%s\n' "$gate_sources" | grep -qxF "$st"; then
    { [[ "$blk" == *"/flow"* ]] && [[ "$blk" == *"confirm"* ]]; } || imperative_missing="$imperative_missing $st(gate)"
  else
    [[ "$blk" == *"set-state.sh"* ]] || imperative_missing="$imperative_missing $st"
  fi
done <<< "$(jq -r '.states | keys[]' "$MACHINE")"
assert_eq "flow-legibility/2" "every orders block states its transition command (set-state.sh, or /flow confirm at a gate)" "$imperative_missing" ""
# Every set-state.sh target named in a block must be a LEGAL next for that state —
# catches a wrong-target regression (naming a non-successor), not just presence.
bad_target=""
while IFS= read -r st; do
  [[ -z "$st" ]] && continue
  blk="$(bash "$SCRIPTS/orders.sh" "$st")"
  nexts="$(jq -r --arg s "$st" '.states[$s].next[]?' "$MACHINE" 2>/dev/null)"
  while IFS= read -r tgt; do
    [[ -z "$tgt" ]] && continue
    printf '%s\n' "$nexts" | grep -qxF "$tgt" || bad_target="$bad_target $st->$tgt"
  done < <(printf '%s\n' "$blk" | grep -oE 'set-state\.sh [a-z][a-z.]*' | sed 's/set-state\.sh //')
done <<< "$(jq -r '.states | keys[]' "$MACHINE")"
assert_eq "flow-legibility/2" "every set-state.sh target in an orders block is a legal next" "$bad_target" ""

echo ""
echo "=== flow-legibility/3 — model-tier pins in delegation contracts ==="
feat_body="$(cat "$FLOW/feature.md")"
verify_body="$(cat "$FLOW/verify.md")"
skill_body="$(cat "$FLOW/SKILL.md")"
assert_contains "flow-legibility/3" "code-explorer pinned to sonnet (feature.md)" "$feat_body" "code-explorer → sonnet"
assert_contains "flow-legibility/3" "code-architect pinned to opus (feature.md)" "$feat_body" "code-architect → opus"
assert_contains "flow-legibility/3" "code-reviewer pinned to opus (verify.md)" "$verify_body" "code-reviewer → opus"
assert_contains "flow-legibility/3" "SKILL.md carries the subagent model-tier policy" "$skill_body" "mechanical/exploration → sonnet"

echo ""
echo "=== flow-legibility/4 — doctrine block + resolver ==="
# Hermeticity precondition for everything below. doctrine.sh (and detect-context.sh)
# honour CLAUDE_PROJECT_DIR by design — that is the plugin-mode contract asserted a
# few lines down. Every Claude Code session exports it, so a suite that inherits it
# reads the DEVELOPER'S live cursor instead of the sandbox's and two fixtures fail
# locally while CI, which sets nothing, stays green. (Pre-existing since 8c97eff:
# `CLAUDE_PROJECT_DIR=/home/user/vibe bash flow/tests/run.sh` -> 226 passed, 2 failed.)
# The suite scrubs it at the top and every fixture that WANTS it sets it per-call.
assert_eq "flow-legibility/4" "suite is hermetic: no ambient CLAUDE_PROJECT_DIR reaches the fixtures" \
  "${CLAUDE_PROJECT_DIR-<unset>}" "<unset>"
rm -f "$STATE"
doc="$(bash "$SCRIPTS/doctrine.sh")"
assert_contains "flow-legibility/4" "doctrine names the two human gates" "$doc" "plan → impl, and verify → ship"
assert_contains "flow-legibility/4" "doctrine carries the durable/ephemeral framing" "$doc" "sessions are ephemeral"
assert_contains "flow-legibility/4" "doctrine states the session-start reads" "$doc" ".spec/lessons.md"
assert_contains "flow-legibility/4" "doctrine states the state.json write invariant" "$doc" ".agents/skills/vibe/state.json"
# inject-triggers/5 (R4): NO live state rides SessionStart output — Claude Code
# replays that output verbatim on --resume, so a cursor line printed here is
# stale by construction, and the per-turn level channel names the state anyway.
# The four assertions above are the population floor for this negative: there IS
# a payload, and it does not name a state.
assert_not_contains "inject-triggers/5" "doctrine prints no cursor summary (idle)" "$doc" "Cursor:"
# ... and still none when the live cursor is somewhere interesting. Discriminating:
# before R4 this printed 'Cursor: feature.impl (feature=widget).'
cp "$FLOW/state.example.json" "$STATE"; bash "$SCRIPTS/set-state.sh" feature.impl widget >/dev/null
doc2="$(bash "$SCRIPTS/doctrine.sh")"
assert_contains "inject-triggers/5" "doctrine still emits the block with a live cursor set" "$doc2" "sessions are ephemeral"
assert_not_contains "inject-triggers/5" "doctrine names no state, whatever the cursor says" "$doc2" "Cursor:"
assert_not_contains "inject-triggers/5" "doctrine leaks no feature name either" "$doc2" "widget"
assert_eq "inject-triggers/5" "doctrine output is a function of SKILL.md alone (cursor-independent)" "$doc2" "$doc"
rm -f "$STATE"
# jq/no-jq byte parity (mirrors orders.sh parity)
pnojq4="$(mktemp -d)"
for t in dirname sed head cat; do ln -sf "$(command -v "$t")" "$pnojq4/$t"; done
a4="$(bash "$SCRIPTS/doctrine.sh" 2>/dev/null)"
b4="$(PATH="$pnojq4" "$BASH_BIN" "$SCRIPTS/doctrine.sh" 2>/dev/null)"
assert_eq "flow-legibility/4" "doctrine.sh jq/no-jq byte-identical (idle)" "$b4" "$a4"
rm -rf "$pnojq4"
# Plugin mode: the hook sets CLAUDE_PROJECT_DIR when the code lives outside the
# repo (per-user plugin). doctrine.sh used to read the PROJECT cursor through it;
# since R4 it reads no cursor at all, so the env var is inert here. Seeded with a
# distinct project cursor anyway — discriminating in the other direction now: a
# doctrine.sh that still consulted it would print quick.triage.
projd="$(mktemp -d)"
mkdir -p "$projd/.agents/skills/vibe"
printf '{"flow":"quick","phase":"triage","feature":"","updated":"x"}\n' > "$projd/.agents/skills/vibe/state.json"
docp="$(CLAUDE_PROJECT_DIR="$projd" bash "$SCRIPTS/doctrine.sh" 2>/dev/null)"
assert_contains "inject-triggers/5" "doctrine still emits the block under CLAUDE_PROJECT_DIR" "$docp" "sessions are ephemeral"
assert_not_contains "inject-triggers/5" "CLAUDE_PROJECT_DIR no longer selects a cursor to print" "$docp" "Cursor:"
assert_not_contains "inject-triggers/5" "the project cursor's state does not leak either" "$docp" "quick.triage"
rm -rf "$projd"
# single-source parity, tied to the CODE: both prose texts must state the same
# per-rule writable-state sets that detect-context.sh `decide` actually enforces.
# Comparing PER-RULE sets against `decide` (not the union of both rules, not
# prose-vs-prose) catches a rule reassignment (moving a state between the lessons
# and root rules) and a prose/code drift — the holes a union or two-substring check
# leaves open.
#
# inject-triggers/2 briefly retired these and was WRONG to (review, Important 2):
# a guard may only be retired once the surface it guards has stopped being
# hand-authored. inject-triggers/6 retires HALF of them, one for one:
#
#   * the AGENTS.md TEMPLATE no longer enumerates any state. `flow.invariants`
#     is composed into the shipped `agents-md` channel, so the states reach a
#     target's AGENTS.md rendered from `content/policy.json` — the same data
#     `decide` reads. The two template assertions are replaced below by the
#     same per-rule comparison run against the RENDER, plus a floor proving the
#     template really stopped enumerating (a deletion that is only a deletion
#     would otherwise read as a pass).
#   * the `vibe:doctrine` block in the vibe SKILL.md is STILL hand-authored —
#     doctrine.sh emits that block verbatim and nothing generates it. Its two
#     assertions therefore STAY. Retiring them would leave the SessionStart
#     payload free to drift from the enforcer with nothing watching.
#
# The `decide` ground-truth pins added below are ADDITIVE, not a replacement:
# they check the enforcer itself (once against the engine with node present, once
# against the permanent bash branch in the no-node leg), where these four check
# the shipped prose against it.
tmpl="$(cat "$FLOW/reference/templates/AGENTS.md")"
assert_contains "flow-legibility/4" "AGENTS.md template shares the gate line" "$tmpl" "plan → impl, and verify → ship"
assert_contains "flow-legibility/4" "AGENTS.md template shares the ephemeral framing" "$tmpl" "sessions are ephemeral"
states_of() { grep -oE '(feature|strategy|setup|quick)\.[a-z]+' | sort -u | paste -sd, -; }
# ground truth: the states for which `decide` returns allow, straight from the code.
allowed_for() {
  local p="$1" s r=""
  while IFS= read -r s; do
    [[ -z "$s" ]] && continue
    [[ "$(bash "$SCRIPTS/detect-context.sh" decide "$p" "$s")" == allow ]] && r="$r$s"$'\n'
  done < <(jq -r '.states|keys[]' "$MACHINE")
  printf '%s' "$r" | states_of
}
lessons_truth="$(allowed_for .spec/lessons.md)"
root_truth="$(allowed_for .spec/product.md)"
src_truth="$(allowed_for src/app.js)"
features_truth="$(allowed_for .spec/features/x/plan.md)"
assert_eq "flow-legibility/4" "decide lessons-rule set is the expected non-trivial set" "$lessons_truth" "feature.compound,quick.verify,setup.apply,strategy.spec"
assert_eq "inject-triggers/2" "decide root-spec rule set is the expected non-trivial set" "$root_truth" "feature.compound,setup.apply,strategy.spec"
assert_eq "inject-triggers/2" "decide src/tests rule allows exactly the impl/fix states" "$src_truth" "feature.impl,quick.fix,setup.apply"
# The features rule is the one whose allow band is the COMPLEMENT of a warn band —
# every state except the two building ones — so it discriminates a rule whose
# arms were inverted, which a same-shape "small allow set" pin cannot.
assert_eq "inject-triggers/2" "decide features rule freezes exactly the two building states" "$features_truth" "feature.compound,feature.design,feature.plan,feature.verify,quick.triage,quick.verify,setup.apply,setup.detect,strategy.brainstorm,strategy.spec"
# The cursor rule has no allow arm at all, so its ground truth is the EMPTY set —
# which an `allowed_for` that examined nothing would produce just as happily. Pin
# the POPULATION alongside the verdict: every one of the machine's states must
# come back a block.
cursor_blocked=0; cursor_seen=0
while IFS= read -r s; do
  [[ -z "$s" ]] && continue
  cursor_seen=$((cursor_seen + 1))
  [[ "$(bash "$SCRIPTS/detect-context.sh" decide .agents/skills/vibe/state.json "$s")" == block:* ]] \
    && cursor_blocked=$((cursor_blocked + 1))
done < <(jq -r '.states|keys[]' "$MACHINE")
assert_eq "inject-triggers/2" "decide blocks a direct state.json edit in every machine state" "$cursor_blocked/$cursor_seen" "13/13"
doc="$(bash "$SCRIPTS/doctrine.sh")"   # no Cursor: line to filter out since R4
doc_lessons="$(printf '%s\n' "$doc" | tr ';' '\n' | grep 'lessons.md' | states_of)"
doc_root="$(printf '%s\n' "$doc" | tr ';' '\n' | grep 'product,tech' | states_of)"
assert_eq "flow-legibility/4" "doctrine lessons rule matches decide" "$doc_lessons" "$lessons_truth"
assert_eq "flow-legibility/4" "doctrine root-spec rule matches decide" "$doc_root" "$root_truth"
# The template's replacement, in two halves.
#
# Half 1 — the template states no rule of its own any more. A bare
# assert_not_contains would pass just as happily against a DELETED template, so
# the negative carries a floor: the template is still there, still the
# instructions block, and still names the enforcer command.
# Structural, not phrase-matched: every `<flow>.<phase>` token in the template's
# HAND-AUTHORED half — the whole file minus the generated `vibe:rules` region
# (which the template now ships pre-rendered; see Important 2 below) — not just
# the ones inside a section a rename could move out from under. Exactly one
# survives, `setup.apply`, in the managed-marker comment that says when the
# block is replaced. Any re-added rule enumeration by hand, in any section and
# any wording, fails this; the generated block is checked for equality with the
# render instead, which is a stronger property than "says nothing".
tmpl_hand="$(awk '/^<!-- vibe:rules -->$/{skip=1} !skip{print} /^<!-- \/vibe:rules -->$/{skip=0}' "$FLOW/reference/templates/AGENTS.md")"
tmpl_states="$(printf '%s\n' "$tmpl_hand" | states_of)"
tmpl_state_lines="$(printf '%s\n' "$tmpl_hand" | grep -cE '(feature|strategy|setup|quick)\.[a-z]+')"
assert_contains "inject-triggers/6" "floor: the template is present and is the instructions block" "$tmpl" "<!-- vibe:instructions:start -->"
assert_contains "inject-triggers/6" "floor: the template still points at the enforcer" "$tmpl" "detect-context.sh decide <path>"
assert_contains "inject-triggers/6" "floor: the hand-authored half is most of the template" "$tmpl_hand" "## Write policy"
assert_eq "inject-triggers/6" "the template's hand-authored half names no flow state outside the marker comment" "$tmpl_states" "setup.apply"
assert_eq "inject-triggers/6" "... and it does so on exactly one line (the marker comment)" "$tmpl_state_lines" "1"
assert_contains "inject-triggers/6" "... which is the managed-marker comment, not a write rule" \
  "$(printf '%s\n' "$tmpl_hand" | grep -E '(feature|strategy|setup|quick)\.[a-z]+')" "replaced on the next setup.apply"
# Half 2 — the states a target now READS come from the render, and the render
# agrees with `decide` per rule. Same comparison the two retired assertions
# made, moved onto the generated text.
if command -v node >/dev/null 2>&1; then
  rend="$( cd "$REPO_ROOT" && node "$FLOW/engine/cli.mjs" render agents-md 2>/dev/null )"
  rend_lessons="$(printf '%s\n' "$rend" | grep -F ".spec/lessons.md" | states_of)"
  rend_root="$(printf '%s\n' "$rend" | grep -F ".spec/product.md" | states_of)"
  assert_contains "inject-triggers/6" "floor: the agents-md render carries the invariants block" "$rend" "Write invariants"
  assert_eq "inject-triggers/6" "floor: each rule is rendered on exactly one line" \
    "$(printf '%s\n' "$rend" | grep -cF ".spec/lessons.md")/$(printf '%s\n' "$rend" | grep -cF ".spec/product.md")" "1/1"
  assert_eq "inject-triggers/6" "rendered lessons rule matches decide" "$rend_lessons" "$lessons_truth"
  assert_eq "inject-triggers/6" "rendered root-spec rule matches decide" "$rend_root" "$root_truth"
  # Fix round 1, Important 2 — the template SHIPS a pre-rendered copy of this
  # block, so a target with no node (a hookless host, where AGENTS.md is the
  # only carrier there is) still receives the real enumerated rules instead of
  # two sections pointing at a block that was never written. That copy is only
  # safe while it cannot drift: it must equal, byte for byte, what
  # `render agents-md` composes from content/policy.json — which is also what
  # makes an install WITH node re-render it to "no change".
  tmpl_rules="$(awk '/^<!-- vibe:rules -->$/{f=1;next} /^<!-- \/vibe:rules -->$/{f=0} f' "$FLOW/reference/templates/AGENTS.md")"
  tmpl_rules_note="$(printf '%s\n' "$tmpl_rules" | head -n1)"
  tmpl_rules_body="$(printf '%s\n' "$tmpl_rules" | sed '1,2d')"
  assert_contains "inject-triggers/6" "the template ships a vibe:rules block with the managed-region note" \
    "$tmpl_rules_note" "_Managed by vibe"
  assert_eq "inject-triggers/6" "the template's shipped vibe:rules body IS the agents-md render (no drift possible)" \
    "$tmpl_rules_body" "$rend"
  # Floor for that equality: neither side is empty, and the body really carries
  # the generated rules (an empty-vs-empty comparison would pass just as well).
  assert_contains "inject-triggers/6" "floor: the shipped block carries the generated invariants" \
    "$tmpl_rules_body" ".spec/lessons.md"
else
  echo "  SKIP [inject-triggers/6] rendered write invariants (node not on PATH)"
fi

echo ""
echo "=== flow-legibility/6 — drift inference (detect-context.sh infer) ==="
DETECT="$SCRIPTS/detect-context.sh"
d1="$(bash "$DETECT" infer $' M src/app.sh' idle)"
assert_contains "flow-legibility/6" "idle + src edit infers feature.impl drift" "$d1" "drift:feature.impl:"
assert_contains "flow-legibility/6" "idle drift names the set-state fix (not /flow, which rejects the hop)" "$d1" "set-state.sh feature.impl or quick.fix"
assert_not_contains "flow-legibility/6" "idle drift does not emit an illegal /flow hop" "$d1" "/flow feature.impl"
# infer matches `tests/` (plural, like decide), not singular `test/`
dsing="$(bash "$DETECT" infer $' M test/x.sh' idle)"
assert_eq "flow-legibility/6" "singular test/ is not code (infer agrees with decide)" "$dsing" ""
d2="$(bash "$DETECT" infer $'?? src/new.sh' feature.design)"
assert_contains "flow-legibility/6" "feature.design + src edit infers drift" "$d2" "drift:feature.impl:"
d2b="$(bash "$DETECT" infer $' M src/x.sh' quick.triage)"
assert_contains "flow-legibility/6" "quick.triage + src edit routes to quick.fix (not feature.impl)" "$d2b" "drift:quick.fix:"
assert_not_contains "flow-legibility/6" "quick.triage drift does not mis-route to feature.impl" "$d2b" "feature.impl"
d3="$(bash "$DETECT" infer $' M src/app.sh' feature.impl)"
assert_eq "flow-legibility/6" "feature.impl + src edit is consistent (no drift)" "$d3" ""
d4="$(bash "$DETECT" infer $' M README.md' idle)"
assert_eq "flow-legibility/6" "idle + non-src edit is not drift" "$d4" ""
d5="$(bash "$DETECT" infer "" idle)"
assert_eq "flow-legibility/6" "idle + clean tree is not drift" "$d5" ""
db="$(bash "$DETECT" decide .spec/lessons.md idle)"
assert_contains "flow-legibility/6" "decide still hard-blocks lessons.md at idle (unchanged)" "$db" "block:"
# no-jq: infer with state passed needs neither jq nor git
pnojq6="$(mktemp -d)"; for t in dirname sed head cat grep; do ln -sf "$(command -v "$t")" "$pnojq6/$t"; done
d6="$(PATH="$pnojq6" "$BASH_BIN" "$DETECT" infer $' M tests/t.sh' idle 2>/dev/null)"
assert_contains "flow-legibility/6" "infer works without jq (tests/ edit, state passed)" "$d6" "drift:feature.impl:"
rm -rf "$pnojq6"

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
# doctrine.sh self-locates like orders.sh and is reached through the symlink from
# the SessionStart hook — both spellings must resolve the same SKILL.md block.
a="$(bash "$REAL_SCRIPTS/doctrine.sh" 2>&1)"
b="$(bash "$ALIAS_SCRIPTS/doctrine.sh" 2>&1)"
assert_eq "path-parity" "doctrine identical via both paths" "$a" "$b"

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
  # js-core/7: the hook is node-first now, so it also reads the JS engine at
  # its standard nested path (the same one install.sh's `cp -RL` lands it at).
  cp -R "$SRC_ROOT/flow/engine" "$s/.agents/skills/vibe/engine"
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
# `node` included (js-core/7): the hook is node-first now, so a farm that
# omits it would test the unrelated "Node absent" degrade (R4, always exit 0)
# instead of the jq-absent path this fixture means to exercise.
for _t in dirname sed head cat grep git node; do
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

# inject-triggers/5 (R6): predicate 3 (the stuck-phase nudge) is DELETED. It
# said what the per-turn level channel now says on every turn, and it queued a
# relay line on every Stop to say it. What used to be pinned here was the
# jq-dependence of that nudge; what is pinned here now is its absence — with a
# control, so "no nudge" cannot be satisfied by a gate that stopped running.
s="$(mk_gate_sbx feature impl widget git)"
out="$(run_gate_nojq "$s" '{}')"
assert_not_contains "inject-triggers/5" "no-jq: no stuck-phase nudge in a non-idle state" "$out" "still in"
assert_contains "inject-triggers/5" "no-jq: a non-idle state exits 0" "$out" "rc=0"
rm -rf "$s"
# CONTROL: the same no-jq gate DOES still speak when it has something to say —
# feature.verify with no receipt is the one blocking tooth.
s="$(mk_gate_sbx feature verify widget git)"
out="$(run_gate_nojq "$s" '{}')"
assert_contains "inject-triggers/5" "no-jq control: the evidence tooth still blocks (exit 2)" "$out" "rc=2"
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
# js-core/7: the hook is node-first now, so it also reads the JS engine at
# its standard nested path (the same one install.sh's `cp -RL` lands it at).
cp -R "$SRC_ROOT/flow/engine" "$gsbx/.agents/skills/vibe/engine"
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

echo "=== install-agnostic-paths — validator refs in flow skill files ==="
# The flow skill's phase files must reference the spec validator portably (the
# `/spec validate` route), never a hard-coded `.agents/skills/spec/scripts/validate.sh`
# — that only resolves in the vendored install and breaks under a global/plugin layout.
# Two deliberate exemptions: (1) the flow skill's OWN vibe-script commands stay
# project-root-relative because they are injected as prompt text, not run from a skill
# dir; (2) the AGENTS.md template's Commands block is the local-install project contract
# (all three commands use vendored paths; the plugin install never creates AGENTS.md),
# so it is left whole. Scan the real source phase files.
flow_validator_bad=0
for f in "$SRC_ROOT"/flow/*.md; do
  [[ -f "$f" ]] || continue
  if grep -qF '.agents/skills/spec/scripts/validate.sh' "$f"; then
    flow_validator_bad=$((flow_validator_bad + 1)); echo "        offender: ${f#"$SRC_ROOT"/}"
  fi
done
assert_eq "install-agnostic-paths" "flow phase files reference the validator portably" "$flow_validator_bad" "0"

echo ""
echo "=== js-core/8 — tests/run.sh dispatches sub-suites on ITS OWN interpreter ==="
# The macOS CI leg exists to run everything under stock /bin/bash 3.2. That is
# only true if the aggregator hands its OWN interpreter down: dispatching through
# a PATH-resolved `bash` lets a Homebrew bash 5 on the runner silently execute
# the three bash suites (and every nested `bash script` inside them), so a
# bash-3.2 regression stays invisible on the one leg built to catch it.
#
# The probe is a miniature repo whose four suite scripts are stubs, run with a
# DECOY `bash` earlier on PATH than the real one. The decoy is a working shell —
# it just announces itself before exec'ing the real binary — so the tree keeps
# running either way; the only signal is whether its marker appears.
AGG_INTERP="${BASH:-$(command -v bash)}"
PROBE="$(mktemp -d)"
mkdir -p "$PROBE/.spec" "$PROBE/tests" "$PROBE/spec/tests" "$PROBE/flow/tests/adapters" \
         "$PROBE/flow/engine/tests" "$PROBE/decoy" "$PROBE/realbin"
# Invoke through a symlink alias of the same binary: the PATH-resolved `bash`
# and the aggregator's own $BASH then differ as STRINGS while staying the same
# shell, so the assertion discriminates on any machine — no second bash needed.
ALIAS="$PROBE/realbin/bash"
ln -s "$AGG_INTERP" "$ALIAS"
cp "$SRC_ROOT/tests/run.sh" "$PROBE/tests/run.sh"
for s in spec/tests/run.sh flow/tests/run.sh flow/tests/adapters/run.sh; do
  # The stub's own body is data, not code to expand here — $BASH must reach the
  # file literally so the STUB reports which shell executed it.
  # shellcheck disable=SC2016
  {
    echo '#!/usr/bin/env bash'
    echo 'echo "DISPATCHED-ON=$BASH"'
    # nested invocation: the suites run install.sh / the hooks this way ~200x
    echo 'bash -c ":"'
  } > "$PROBE/$s"
  chmod +x "$PROBE/$s"
done
printf 'process.stdout.write("stub-engine\\n");\n' > "$PROBE/flow/engine/tests/run.mjs"
{
  echo '#!/bin/sh'
  echo 'echo "DECOY-BASH-USED" >&2'
  echo "exec $AGG_INTERP \"\$@\""
} > "$PROBE/decoy/bash"
chmod +x "$PROBE/decoy/bash"
probe_out="$(PATH="$PROBE/decoy:$PATH" "$ALIAS" "$PROBE/tests/run.sh" 2>&1 || true)"
assert_not_contains "js-core/8" "no suite (or nested call) resolves bash through PATH" "$probe_out" "DECOY-BASH-USED"
probe_dispatched="$(printf '%s\n' "$probe_out" | grep -c "DISPATCHED-ON=$ALIAS" || true)"
assert_eq "js-core/8" "all three bash suites run on the aggregator's own interpreter" "$probe_dispatched" "3"
rm -rf "$PROBE"

echo ""
echo "=== js-core/8 — bash-3.2 portability lint (stock macOS /bin/bash) ==="
# Stock macOS ships /bin/bash 3.2.57 (Apple froze it at the last GPLv2 release) and
# every tracked script here runs under `set -u`. bash < 4.4 raises
# `arr[@]: unbound variable` and ABORTS when an EMPTY array is expanded through an
# unguarded [@] / [*] slice. That is exactly how the shipped `install.sh --uninstall`
# died on every macOS machine: it aborted mid-way, silently skipping hook removal,
# the CLAUDE.md/WARP.md symlink loop and the AGENTS.md unmerge. The 3.2-safe
# spellings are ${arr[@]+"..."} in list context and ${arr[*]:-} in string context;
# ${#arr[@]} is already safe. Five more bash-4-only constructs are banned alongside,
# so the next one cannot land silently either.
#
# Patterns are written with one-character classes ([A] for A) so this file's own
# source cannot match itself while it is scanned, and the two guarded array
# spellings are stripped from each line before the bare-slice pattern runs (the
# fix idiom nests a literal slice inside its own guard).
B32_BARE_SLICE='\$\{[A-Za-z_][A-Za-z0-9_]*\[[@*]\]\}'
B32_BANNED='(declare|local|typeset)[[:space:]]+-[a-zA-Z]*[An]|(map[f]ile|read[a]rray)[[:space:]]|glob[s]tar|&>[>]|\$\{[A-Za-z_][A-Za-z0-9_]*(\[[^]]*\])?(,,|,|\^\^|\^)\}|\$\{[A-Za-z_][A-Za-z0-9_]*@[A-Za-z]\}'
b32_bad=0
b32_broken=0
B32_SCANNED=""
B32_ROOT="$SRC_ROOT"
# One spelling of the report label, reused by the assertions below. They match
# `<label> <path>:<line>`, never a bare path: `sed: can't read <path>` also
# contains the path, so a path-only substring test would pass on exactly the
# broken-enumeration geometry it exists to catch.
B32_LABEL="unguarded array slice under set -u:"
b32_scan() {
  local file="$1" label="$2" pattern="$3" hits
  hits="$(sed -e 's/^[[:space:]]*#.*$//' \
              -e 's/\${[A-Za-z_][A-Za-z0-9_]*\[[@*]\][-+][^}]*}//g' \
              -e 's/\${[A-Za-z_][A-Za-z0-9_]*\[[@*]\]:[-+][^}]*}//g' "$file" \
          | grep -nE "$pattern" || true)"
  [[ -n "$hits" ]] || return 0
  while IFS= read -r h; do
    [[ -n "$h" ]] || continue
    b32_bad=$((b32_bad + 1))
    echo "        $label ${file#"$B32_ROOT"/}:${h%%:*}"
  done <<< "$hits"
}
# Enumerate the files a lint owns, null-delimited (paths may contain spaces).
# GLOB defaults to '*.sh'; the .mjs path-derivation lint at the bottom of this
# file passes '*.mjs' rather than keeping its own raw `find` walk (js-core/8 final
# review, M6) — one enumerator, so a gitignored tree cannot be honest for one lint
# and invisible to the other.
#
# `git ls-files` is what makes the assertion's word "tracked" literally true: a
# filesystem walk also picks up gitignored trees, whose offenders nobody can fix.
# Index order is already sorted, so the report stays deterministic. Non-git install
# targets (this tool ships into repos with no .git) degrade to a find(1) walk that
# still asks `git check-ignore` per file whenever git can answer.
#
# PRECISELY what "warn-first" means here, because the comment used to overstate it
# (js-core/8 final review, M7): it is a property of the ENUMERATION, not of the
# verdict. When git cannot answer whether a path is ignored, that path is SCANNED
# rather than silently skipped — the lint never narrows itself into vacuity. An
# offender found that way still counts toward `b32_bad`, and the caller's
# assertion still fails. So on a source tarball with no `.git` at all and a
# leftover ignored worktree present, this lint reports offenders nobody can fix.
# That is a known, accepted cost: the alternative — a hardcoded skip list, or a
# fallback that degrades to silence — is exactly the vacuity fix round 3 closed,
# and .spec/lessons.md forbids the hand-maintained list. Recorded rather than
# papered over.
#
# Two spellings here are load-bearing, both from the re-review's Finding 3:
#   * NO `--full-name`. That flag prints paths relative to the REPOSITORY root,
#     which is not $root whenever this source is vendored into a subdirectory of
#     a larger repo (`tools/vibe/`, where the marker search stops at
#     `tools/vibe/.spec` but git's root is the monorepo). Joined back onto $root
#     they became `$root/tools/vibe/…` — twelve paths that do not exist, sed
#     could not open one of them, and the lint reported a clean tree. `git -C`
#     already makes $root the cwd, so plain output is relative to $root.
#   * `--cached --others --exclude-standard`, not bare `ls-files`. Bare means
#     "in the index", so a fresh clone-and-init, or a new script not yet
#     `git add`ed, enumerated nothing (or missed the one file that mattered)
#     while still reporting success. --others adds untracked-but-not-ignored
#     files; --exclude-standard keeps the gitignore honesty that made this
#     rewrite necessary in the first place.
b32_list_scripts() {
  local root="$1" glob="${2:-*.sh}" rel f
  if command -v git >/dev/null 2>&1 &&
     git -C "$root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    while IFS= read -r -d '' rel; do
      printf '%s\0' "$root/$rel"
    done < <(git -C "$root" ls-files -z --cached --others --exclude-standard -- "$glob" 2>/dev/null)
    return 0
  fi
  while IFS= read -r -d '' f; do
    git -C "$root" check-ignore -q -- "$f" 2>/dev/null && continue
    printf '%s\0' "$f"
  done < <(find "$root" -name "$glob" -not -path '*/.git/*' -print0)
}
# COVERAGE cross-check (js-core/8 fix round 3; re-review round 2, Important 4).
#
# `b32_broken` above is a CARDINALITY floor: it sees "scanned zero" and
# "scanned nothing readable". It cannot see PARTIAL enumeration, which is the
# geometry a real install has — one unrelated `.sh` anywhere is enough to hold
# it at 0 while every script that mattered went unscanned. Three ways that
# happens, none of them exotic: this source vendored under a directory the host
# repo gitignores (`git ls-files --others --exclude-standard` skips it), this
# source inside a git SUBMODULE (`ls-files` reports a gitlink, never contents),
# and any geometry nobody has enumerated yet.
#
# So the two enumeration mechanisms already in this file — the git listing and
# the find(1) walk — are CROSS-CHECKED instead of used as alternatives: every
# `.sh` the walk finds must either have been scanned or be a path git itself
# calls ignored. That subsumes the enumerated geometries and the ones nobody
# thought of, because it never asks WHY a file was omitted, only whether the
# omission is explained.
#
# `check-ignore --stdin` is one invocation for the whole list rather than one
# per file: this walk crosses gitignored worktrees and node_modules on a
# developer machine, and a per-file fork there is seconds, not milliseconds.
b32_coverage_check() {
  local root="$1" f rel walk="" ignored=""
  while IFS= read -r -d '' f; do
    rel="${f#"$root"/}"
    printf '%s\n' "$B32_SCANNED" | grep -qxF "$rel" && continue
    walk="$walk$rel"$'\n'
  done < <(find "$root" -name '*.sh' -not -path '*/.git/*' -print0 2>/dev/null)
  [[ -n "$walk" ]] || return 0
  if command -v git >/dev/null 2>&1 &&
     git -C "$root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    ignored="$(printf '%s' "$walk" | git -C "$root" check-ignore --stdin 2>/dev/null || true)"
  fi
  while IFS= read -r rel; do
    [[ -n "$rel" ]] || continue
    printf '%s\n' "$ignored" | grep -qxF "$rel" && continue
    b32_broken=$((b32_broken + 1))
    echo "        enumeration omits a shell script the walk found, and git does not call it ignored: $rel"
  done <<< "$walk"
}
# A guard that scans nothing must fail loudly, never report success. `b32_bad`
# counts OFFENDERS and cannot distinguish "clean tree" from "no tree"; the
# enumeration faults get their own counter so the difference is visible:
#   * an empty file list,
#   * a listed path that cannot be opened (the wrong-root join above), and
#   * a file the walk found that the enumeration dropped without explanation.
# B32_SCANNED carries the root-relative paths actually read, for the structural
# floors the caller asserts against.
b32_lint() {
  local root="$1" f n=0 missing=0
  B32_ROOT="$root"
  B32_SCANNED=""
  while IFS= read -r -d '' f; do
    n=$((n + 1))
    if [[ ! -r "$f" ]]; then
      missing=$((missing + 1))
      echo "        enumerated path is not readable (enumeration is rooted wrong): ${f#"$root"/}"
      continue
    fi
    B32_SCANNED="$B32_SCANNED${f#"$root"/}"$'\n'
    b32_scan "$f" "$B32_LABEL" "$B32_BARE_SLICE"
    b32_scan "$f" "bash-4-only construct:" "$B32_BANNED"
  done < <(b32_list_scripts "$root")
  if [[ "$n" -eq 0 ]]; then
    b32_broken=$((b32_broken + 1))
    echo "        enumerated no shell scripts under $root — the lint scanned NOTHING"
  fi
  if [[ "$missing" -gt 0 ]]; then
    b32_broken=$((b32_broken + 1))
    echo "        $missing of $n enumerated paths are unreadable — the lint scanned almost NOTHING"
  fi
  b32_coverage_check "$root"
}
b32_lint "$SRC_ROOT"
assert_eq "js-core/8" "no bash-3.2-hostile constructs in tracked or new shell scripts" "$b32_bad" "0"
# A guard that scanned nothing must not read as a clean tree. `b32_broken` counts
# enumeration faults (empty list, or a listed path that cannot be opened) — kept
# on its own axis so "no offenders" and "no files" can never be confused.
assert_eq "js-core/8" "bash-3.2 lint enumeration is sound (non-empty, every path readable)" "$b32_broken" "0"
# SELF-COVERAGE floor (js-core/8 fix round 3; re-review round 2, Important 4).
#
# The strongest floor available costs one line and cannot rot: the lint must
# have read THE VERY FILE MAKING THIS ASSERTION. It needs no list, no count and
# no sibling file, and it goes red in exactly the geometries the cardinality
# floor misses — this source vendored under a gitignored directory, or inside a
# submodule, where `b32_bad` and `b32_broken` both stay 0 because the host tree
# supplied enough unrelated scripts to look like a successful scan.
#
# `pwd -P` resolves the physical path, so an invocation through the shipped
# `.agents/skills/vibe -> flow` symlink converges on the same relative path
# SRC_ROOT is expressed in (.spec/lessons.md: self-locate by resolving, do not
# count hops).
b32_self_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
b32_self_abs="$b32_self_dir/$(basename "${BASH_SOURCE[0]}")"
b32_self_rel="${b32_self_abs#"$SRC_ROOT"/}"
assert_eq "js-core/8" "bash-3.2 lint scanned the very file asserting it (self-coverage floor)" \
  "$(printf '%s\n' "$B32_SCANNED" | grep -qxF "$b32_self_rel" && echo scanned || echo "NOT SCANNED: $b32_self_rel")" \
  "scanned"

# Structural floor rather than a pinned count: whatever bash suites tests/run.sh
# dispatches MUST be inside the set this lint scanned. Derived from the
# aggregator at runtime, so adding a suite widens the floor automatically and a
# hand-maintained number cannot rot (.spec/lessons.md: hand-written counts rot
# silently). flow/tests/run.sh — this file — is one of them, so the floor is
# also self-referential: the lint must always find the tree it is scanning from.
b32_floor_missing=""
b32_floor_n=0
b32_floor_list="$(sed -n '/^SUITE_SCRIPTS=(/,/^)/p' "$SRC_ROOT/tests/run.sh" 2>/dev/null | grep -oE '"[^"]+\.sh"' | tr -d '"')"
while IFS= read -r b32_rel; do
  [[ -n "$b32_rel" ]] || continue
  b32_floor_n=$((b32_floor_n + 1))
  printf '%s\n' "$B32_SCANNED" | grep -qxF "$b32_rel" || b32_floor_missing="$b32_floor_missing $b32_rel"
done <<< "$b32_floor_list"
assert_eq "js-core/8" "bash-3.2 lint scanned every bash suite the aggregator dispatches" "$b32_floor_missing" ""
# ...and the scrape it is derived FROM must not be empty, which is how this
# floor passed VACUOUSLY (js-core/8 fix round 3; re-review round 2, Important 4):
# an absent $SRC_ROOT/tests/run.sh — a fresh install target has none — or a
# SUITE_SCRIPTS array ever written without double quotes yields no entries at
# all, the loop body never executes, and the assertion above succeeds having
# compared nothing. This is a floor on the MECHANISM (did the scrape parse
# anything?), deliberately not on this file's own name: coupling it to
# "$b32_self_rel is one of the entries" would also fail for any legitimate
# rename or copy of this suite, which is a different claim than the one being
# made here.
[[ "$b32_floor_n" -gt 0 ]] || echo "        SUITE_SCRIPTS scrape of $SRC_ROOT/tests/run.sh produced NO entries"
assert_eq "js-core/8" "bash-3.2 lint suite floor scraped a non-empty aggregator list" \
  "$([[ "$b32_floor_n" -gt 0 ]] && echo ok || echo "scraped nothing")" "ok"

# Enumeration teeth, pinned in BOTH directions. A filesystem walk used to report
# offenders inside gitignored trees — a leftover subagent worktree under
# .claude/worktrees/ (this repo spawns them), or node_modules/. Those files are
# pinned at another commit and nobody can fix them, so the lint went red for every
# developer using worktrees while CI (a fresh checkout, no worktrees) stayed green:
# a guard that cries wolf is a guard people learn to ignore. The cure must not be a
# scan narrowed until it finds nothing, so the tracked direction is pinned too.
# The probe's offender lives in a directory with a space to hold the null-delimited
# enumeration honest.
B32PROBE="$(mktemp -d)"
mkdir -p "$B32PROBE/ignored" "$B32PROBE/tracked dir"
printf 'ignored/\n' > "$B32PROBE/.gitignore"
# The hostile spelling is split across two literals so this file's own source
# cannot match the scan it is feeding, same reason as the [A]-class patterns above.
# The slice must reach the probe file unexpanded — it is data, not an expansion.
# shellcheck disable=SC2016
b32_hostile='printf "%s\n" "${'"arr[@]"'}"'
for d in ignored "tracked dir"; do
  {
    echo '#!/usr/bin/env bash'
    echo 'set -euo pipefail'
    echo 'arr=()'
    echo "$b32_hostile"
  } > "$B32PROBE/$d/bad.sh"
done
git -C "$B32PROBE" init -q >/dev/null 2>&1 || true
git -C "$B32PROBE" add -A >/dev/null 2>&1 || true
b32_probe_out="$(b32_bad=0; b32_lint "$B32PROBE" 2>&1)"
assert_not_contains "js-core/8" "bash-3.2 lint ignores gitignored trees (worktrees, node_modules)" \
  "$b32_probe_out" "ignored/bad.sh"
assert_contains "js-core/8" "bash-3.2 lint still flags a tracked script (path with spaces)" \
  "$b32_probe_out" "$B32_LABEL tracked dir/bad.sh:4"
# Install targets can have no .git at all (the stranger-eval lesson). The fallback
# walk must still find offenders there — an enumeration that degrades to silence
# would make the lint vacuous on every target that is not this repo.
rm -rf "$B32PROBE/.git"
b32_nogit_out="$(b32_bad=0; b32_lint "$B32PROBE" 2>&1)"
assert_contains "js-core/8" "bash-3.2 lint degrades to a find walk with no .git" \
  "$b32_nogit_out" "$B32_LABEL tracked dir/bad.sh:4"
rm -rf "$B32PROBE"

# Enumeration soundness, the OTHER direction: a guard that scans NOTHING must
# fail loudly, never report success. Two geometries produced an empty or bogus
# file list while `b32_bad` stayed 0 and the assertion PASSED (js-core/8 fix
# round 1 re-review, Finding 3):
#   A. a git work tree whose index is empty        -> `git ls-files` prints nothing
#   B. $SRC_ROOT is a SUBDIRECTORY of a git repo   -> `--full-name` prints paths
#      relative to the REPOSITORY root, which joined to $SRC_ROOT do not exist
# B is reachable by vendoring this source into a monorepo (`tools/vibe/`), where
# the marker search stops at `tools/vibe/.spec` while git's root is the monorepo.
# Both are pinned end-to-end below with the real `install.sh` defect spelling.
B32GEO="$(mktemp -d)"
mkdir -p "$B32GEO/repo/vendor/vibe/scripts" "$B32GEO/repo/unrelated"
{
  echo '#!/usr/bin/env bash'
  echo 'set -euo pipefail'
  echo 'arr=()'
  echo "$b32_hostile"
} > "$B32GEO/repo/vendor/vibe/scripts/bad.sh"
echo 'echo fine' > "$B32GEO/repo/unrelated/ok.sh"
git -C "$B32GEO/repo" init -q >/dev/null 2>&1 || true
git -C "$B32GEO/repo" add -A >/dev/null 2>&1 || true

# Case B — root is a repo subdirectory. Assertions match the lint's OWN report
# line, never a bare path: `sed: can't read <path>` on an unopenable file also
# contains the path, so a substring test on the path alone passes on exactly the
# broken geometry it is meant to catch.
b32_sub_out="$(b32_bad=0; b32_broken=0; b32_lint "$B32GEO/repo/vendor/vibe" 2>&1; echo "broken=$b32_broken")"
assert_contains "js-core/8" "bash-3.2 lint finds offenders when the root is a repo subdirectory" \
  "$b32_sub_out" "$B32_LABEL scripts/bad.sh:4"
assert_eq "js-core/8" "bash-3.2 lint enumerates no unreadable paths from a subdirectory" \
  "$(printf '%s\n' "$b32_sub_out" | grep -c "can't read" || true)|${b32_sub_out##*broken=}" "0|0"
# and it must not reach outside its own root
assert_not_contains "js-core/8" "bash-3.2 lint from a subdirectory stays inside that subdirectory" \
  "$b32_sub_out" "unrelated/ok.sh"

# Case A — a git work tree that enumerates nothing at all. Silence here is the
# failure: the lint must name the reason and be counted, not report success.
B32VOID="$(mktemp -d)"
git -C "$B32VOID" init -q >/dev/null 2>&1 || true
b32_void_out="$(b32_bad=0; b32_broken=0; b32_lint "$B32VOID" 2>&1; echo "broken=$b32_broken")"
assert_contains "js-core/8" "bash-3.2 lint names the reason when it enumerates nothing" \
  "$b32_void_out" "enumerated no shell scripts"
assert_contains "js-core/8" "bash-3.2 lint counts an empty enumeration as a failure" \
  "$b32_void_out" "broken=1"
rm -rf "$B32VOID"

# The OTHER enumeration fault, pinned positively rather than only by its absence
# above. Case B's assertions say "no unreadable paths" — true of the fixed
# enumeration, and equally true of a build with the unreadable-path counter
# deleted, so on their own they pin nothing. This drives the branch directly by
# substituting the enumerator inside a subshell: one bogus path, nothing
# readable, and the report must name it and raise b32_broken.
B32UNREADABLE="$(mktemp -d)"
b32_unreadable_out="$(
  b32_bad=0; b32_broken=0
  b32_list_scripts() { printf '%s\0' "$1/nowhere/rooted-wrong.sh"; }
  b32_lint "$B32UNREADABLE" 2>&1
  echo "broken=$b32_broken"
)"
assert_contains "js-core/8" "bash-3.2 lint names an enumerated path it cannot read" \
  "$b32_unreadable_out" "not readable (enumeration is rooted wrong): nowhere/rooted-wrong.sh"
assert_contains "js-core/8" "bash-3.2 lint counts unreadable enumerated paths as a failure" \
  "$b32_unreadable_out" "broken=1"
# and an unreadable path must never be counted as scanned
assert_not_contains "js-core/8" "bash-3.2 lint does not record an unreadable path as scanned" \
  "$b32_unreadable_out" "enumerated no shell scripts"
rm -rf "$B32UNREADABLE"

# PARTIAL enumeration, the fault the cardinality floors above cannot see
# (js-core/8 fix round 3; re-review round 2, Important 4). All three fixtures
# below keep `b32_bad` and the empty/unreadable counters at 0 — the tree looks
# scanned — while the scripts that mattered were never read.

# (a) The coverage cross-check, driven directly: substitute the enumerator with
# one that returns a strict SUBSET, exactly as a narrowed `git ls-files` does.
# Both the report line and the counter are pinned; without the counter the
# report alone would leave the suite green.
B32PARTIAL="$(mktemp -d)"
mkdir -p "$B32PARTIAL/a" "$B32PARTIAL/b"
echo 'echo fine' > "$B32PARTIAL/a/seen.sh"
echo 'echo fine' > "$B32PARTIAL/b/unseen.sh"
b32_partial_out="$(
  b32_bad=0; b32_broken=0
  b32_list_scripts() { printf '%s\0' "$1/a/seen.sh"; }
  b32_lint "$B32PARTIAL" 2>&1
  echo "bad=$b32_bad broken=$b32_broken"
)"
assert_contains "js-core/8" "bash-3.2 lint names a script the enumeration dropped without explanation" \
  "$b32_partial_out" "enumeration omits a shell script the walk found, and git does not call it ignored: b/unseen.sh"
assert_contains "js-core/8" "bash-3.2 lint counts a partial enumeration as a failure (offenders stay 0)" \
  "$b32_partial_out" "bad=0 broken=1"
rm -rf "$B32PARTIAL"

# (b) A git SUBMODULE / nested repo. `git ls-files` reports it as a gitlink and
# never its contents, so the offender inside is invisible to the enumeration
# while the host tree supplies enough scripts to keep every count at 0.
B32SUB="$(mktemp -d)"
mkdir -p "$B32SUB/host/nested"
echo 'echo fine' > "$B32SUB/host/top.sh"
{
  echo '#!/usr/bin/env bash'
  echo 'set -euo pipefail'
  echo 'arr=()'
  echo "$b32_hostile"
} > "$B32SUB/host/nested/bad.sh"
git -C "$B32SUB/host" init -q >/dev/null 2>&1 || true
git -C "$B32SUB/host/nested" init -q >/dev/null 2>&1 || true
git -C "$B32SUB/host/nested" add -A >/dev/null 2>&1 || true
git -C "$B32SUB/host/nested" -c user.email=t@t -c user.name=t commit -qm x >/dev/null 2>&1 || true
git -C "$B32SUB/host" add -A >/dev/null 2>&1 || true
b32_sub2_out="$(b32_bad=0; b32_broken=0; b32_lint "$B32SUB/host" 2>&1; echo "bad=$b32_bad broken=$b32_broken")"
assert_contains "js-core/8" "bash-3.2 lint reports a nested-repo script its git enumeration cannot see" \
  "$b32_sub2_out" "enumeration omits a shell script the walk found"
assert_contains "js-core/8" "bash-3.2 lint counts the nested-repo blind spot (offenders still 0)" \
  "$b32_sub2_out" "bad=0 broken=1"
rm -rf "$B32SUB"

# (c) This source VENDORED under a directory the host repo gitignores — the
# geometry install.sh targets. `_find_repo_root` can stop at the HOST root, git
# then reports the host's own scripts and none of vibe's, and every counter
# stays 0 because one unrelated host script is enough to look like a scan. The
# gitignore genuinely explains the omission, so the coverage check above is
# silent here BY DESIGN — this is what the SELF-COVERAGE floor is for, and the
# assertions below drive that floor's own expression against the fixture
# rather than restating it in prose.
B32VENDOR="$(mktemp -d)"
mkdir -p "$B32VENDOR/host/.agents/skills/vibe/flow/tests"
printf '.agents/\n' > "$B32VENDOR/host/.gitignore"
echo 'echo fine' > "$B32VENDOR/host/unrelated.sh"
echo 'echo fine' > "$B32VENDOR/host/.agents/skills/vibe/flow/tests/run.sh"
git -C "$B32VENDOR/host" init -q >/dev/null 2>&1 || true
git -C "$B32VENDOR/host" add -A >/dev/null 2>&1 || true
b32_vendor_out="$(b32_bad=0; b32_broken=0; b32_lint "$B32VENDOR/host" >/dev/null 2>&1; echo "bad=$b32_bad broken=$b32_broken"; printf '%s' "$B32_SCANNED")"
assert_contains "js-core/8" "vendored-under-gitignore: the cardinality floors stay silent (this is the hole)" \
  "$b32_vendor_out" "bad=0 broken=0"
assert_contains "js-core/8" "vendored-under-gitignore: the host's own scripts ARE scanned" \
  "$b32_vendor_out" "unrelated.sh"
assert_not_contains "js-core/8" "vendored-under-gitignore: none of the vendored tree is scanned" \
  "$b32_vendor_out" ".agents/skills/vibe/flow/tests/run.sh"
# The self-coverage floor's own expression, run against that scanned set: the
# file asserting the lint ran is missing from it, so the floor goes red.
b32_vendor_scanned="$(b32_bad=0; b32_broken=0; b32_lint "$B32VENDOR/host" >/dev/null 2>&1; printf '%s' "$B32_SCANNED")"
assert_eq "js-core/8" "self-coverage floor goes red when the lint's own tree is unscanned" \
  "$(printf '%s\n' "$b32_vendor_scanned" | grep -qxF ".agents/skills/vibe/flow/tests/run.sh" && echo scanned || echo "NOT SCANNED")" \
  "NOT SCANNED"
rm -rf "$B32VENDOR"

# A file that exists but was never `git add`ed is still this lint's business —
# "tracked" must not silently narrow to "staged". Same tree as Case A, one
# uncommitted, unstaged offender.
B32NEW="$(mktemp -d)"
git -C "$B32NEW" init -q >/dev/null 2>&1 || true
{
  echo '#!/usr/bin/env bash'
  echo 'set -euo pipefail'
  echo 'arr=()'
  echo "$b32_hostile"
} > "$B32NEW/fresh.sh"
b32_new_out="$(b32_bad=0; b32_broken=0; b32_lint "$B32NEW" 2>&1)"
assert_contains "js-core/8" "bash-3.2 lint flags an unstaged new script (tracked != staged)" \
  "$b32_new_out" "$B32_LABEL fresh.sh:4"
rm -rf "$B32NEW"

# End-to-end: the real `install.sh:99` defect spelling must be caught in EVERY
# invocation geometry, not just the one the repo happens to have. Same offender
# file, four roots: repo root, repo subdirectory, empty-index tree, no .git.
b32_e2e_modes=0
b32_e2e_caught=0
b32_e2e_check() {
  b32_e2e_modes=$((b32_e2e_modes + 1))
  case "$1" in *"$B32_LABEL $2:4"*) b32_e2e_caught=$((b32_e2e_caught + 1)) ;; esac
}
b32_e2e_check "$(b32_bad=0; b32_broken=0; b32_lint "$B32GEO/repo" 2>&1)" "vendor/vibe/scripts/bad.sh"
b32_e2e_check "$(b32_bad=0; b32_broken=0; b32_lint "$B32GEO/repo/vendor/vibe" 2>&1)" "scripts/bad.sh"
git -C "$B32GEO/repo" rm -r --cached -q . >/dev/null 2>&1 || true
b32_e2e_check "$(b32_bad=0; b32_broken=0; b32_lint "$B32GEO/repo" 2>&1)" "vendor/vibe/scripts/bad.sh"
rm -rf "$B32GEO/repo/.git"
b32_e2e_check "$(b32_bad=0; b32_broken=0; b32_lint "$B32GEO/repo" 2>&1)" "vendor/vibe/scripts/bad.sh"
assert_eq "js-core/8" "bash-3.2 lint catches the install.sh defect in every invocation geometry" \
  "$b32_e2e_caught" "$b32_e2e_modes"
rm -rf "$B32GEO"

echo ""
echo "=== js-core/8 — every .mjs derives its own path via fileURLToPath ==="
# `new URL(import.meta.url).pathname` keeps percent-encoding, so a checkout under
# `…/with space/vibe` yields `…/with%20space/…` and the module crashes at import.
# The sweep to fileURLToPath is complete and mutation-proved load-bearing; this
# grep is what stops a reintroduction, together with CI's spaced-path leg.
#
# Enumeration is b32_list_scripts with a '*.mjs' glob, NOT a raw `find` walk
# (js-core/8 final review, M6). The sibling bash-3.2 lint was rewritten onto
# `git ls-files --cached --others --exclude-standard` (with a `check-ignore`
# fallback) precisely because a filesystem walk reports offenders inside
# gitignored trees that nobody can fix — this repo spawns subagent worktrees
# under .claude/worktrees/, and a `flow/node_modules/` would trip it identically.
# Two enumerators meant one lint was honest about gitignore and the other was not.
url_pathname_bad=0
url_scanned=0
url_scanned_list=""
while IFS= read -r -d '' f; do
  url_scanned=$((url_scanned + 1))
  url_scanned_list="$url_scanned_list${f#"$SRC_ROOT"/}"$'\n'
  if grep -qE 'new URL\(import\.meta\.url\)' "$f"; then
    url_pathname_bad=$((url_pathname_bad + 1)); echo "        offender: ${f#"$SRC_ROOT"/}"
  fi
done < <(b32_list_scripts "$SRC_ROOT/flow" '*.mjs')
assert_eq "js-core/8" "no .mjs re-derives its path from new URL(import.meta.url)" "$url_pathname_bad" "0"
# Same "scanned nothing" reasoning as the bash lint above (js-core/8 fix round 1
# re-review, Finding 3), but asserted against the SCANNED SET rather than
# alongside it. The previous spelling was `url_scanned -gt 0 && -f .../cli.mjs`:
# the walk is `find "$SRC_ROOT/flow" -name '*.mjs'`, so whenever cli.mjs exists
# the walk is non-empty BY CONSTRUCTION and the two conditions could not
# disagree — a tautology presented as a floor (re-review round 2, Minor 3). This
# asserts what it always read as: the engine's entry point was actually READ.
[[ "$url_scanned" -gt 0 ]] || echo "        .mjs lint enumerated no files under $SRC_ROOT/flow"
assert_eq "js-core/8" ".mjs path-derivation lint actually read the engine entry point" \
  "$(printf '%s\n' "$url_scanned_list" | grep -qxF "flow/engine/cli.mjs" && echo ok || echo "flow/engine/cli.mjs was not in the scanned set")" \
  "ok"
# Enumeration teeth in BOTH directions, same shape the bash lint's own probe uses
# (js-core/8 final review, M6). Without this the switch off `find` is an
# unverified claim: a pathspec that silently matched nothing would leave the
# scanned set empty and only the self-coverage floor above would notice, and only
# for one file.
MJSPROBE="$(mktemp -d)"
mkdir -p "$MJSPROBE/ignored" "$MJSPROBE/tracked dir/nested"
printf 'ignored/\n' > "$MJSPROBE/.gitignore"
for d in ignored "tracked dir/nested"; do
  echo 'const p = new URL(import.meta.url).pathname;' > "$MJSPROBE/$d/probe.mjs"
done
git -C "$MJSPROBE" init -q >/dev/null 2>&1 || true
git -C "$MJSPROBE" add -A >/dev/null 2>&1 || true
mjs_probe_list=""
while IFS= read -r -d '' f; do mjs_probe_list="$mjs_probe_list${f#"$MJSPROBE"/}"$'\n'; done < <(b32_list_scripts "$MJSPROBE" '*.mjs')
assert_eq "js-core/8" ".mjs enumeration finds a NESTED tracked file (pathspec is recursive)" \
  "$(printf '%s\n' "$mjs_probe_list" | grep -qxF "tracked dir/nested/probe.mjs" && echo found || echo "MISSED")" "found"
assert_eq "js-core/8" ".mjs enumeration skips gitignored trees (worktrees, node_modules)" \
  "$(printf '%s\n' "$mjs_probe_list" | grep -qxF "ignored/probe.mjs" && echo "LEAKED" || echo skipped)" "skipped"
# Install targets can have no .git at all: the fallback walk must still find it.
rm -rf "$MJSPROBE/.git"
mjs_nogit_list=""
while IFS= read -r -d '' f; do mjs_nogit_list="$mjs_nogit_list${f#"$MJSPROBE"/}"$'\n'; done < <(b32_list_scripts "$MJSPROBE" '*.mjs')
assert_eq "js-core/8" ".mjs enumeration degrades to a find walk with no .git" \
  "$(printf '%s\n' "$mjs_nogit_list" | grep -qxF "tracked dir/nested/probe.mjs" && echo found || echo "MISSED")" "found"
rm -rf "$MJSPROBE"

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
