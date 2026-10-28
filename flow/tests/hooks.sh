#!/usr/bin/env bash
# flow/tests/hooks.sh — the F6 shim and adapter teeth: `flow/bin/hook.sh`'s
# five-tier discovery (D26), the exit convention it enforces (FV-R5), the plugin
# wiring, and the `port-order` gate that keeps TUI/signals behind `GREEN.hard`.
#
# Every discovery leg deletes the higher tiers' variables from the CHILD env with
# `env -u` — not blanked, deleted — and proves the deletion from inside the child
# (a fake binary dumps its own environment). A leg that only proved "the answer
# came from somewhere" would pass with the wrong tier answering.
#
# Pure bash; no bats. Exit 0 = all pass.

# The `cond && pass || fail` reporting idiom is intentional and safe here:
# pass()/fail() always return 0, so fail never runs spuriously after pass.
# shellcheck disable=SC2015
set -uo pipefail

_find_repo_root() {
  local d; d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  while [[ "$d" != "/" ]]; do
    [[ -d "$d/docs/spec" || -e "$d/.git" ]] && { printf '%s\n' "$d"; return 0; }
    d="$(dirname "$d")"
  done
  return 1
}
SRC_ROOT="$(_find_repo_root)" || { echo "cannot locate repo root (docs/spec/.git)" >&2; exit 1; }

# Hermeticity: an ambient CLAUDE_PROJECT_DIR would point the shim's child at the
# developer's live cursor instead of the sandbox (the same trap the flow suite
# documents). Every fixture that wants it sets it per invocation.
unset CLAUDE_PROJECT_DIR

WORK="$(mktemp -d)"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

PASS=0
FAIL=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
assert_contains()     { if [[ "$3" == *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want contains: $4"; echo "        got: $3"; fi; }
assert_not_contains() { if [[ "$3" != *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want NOT contains: $4"; fi; }
assert_eq()           { if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want: $4"; echo "        got:  $3"; fi; }

SHIM="$SRC_ROOT/flow/bin/hook.sh"
OS="$(uname -s 2>/dev/null | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m 2>/dev/null)"

echo "=== f6/shim — shape: POSIX, ≤40 lines, shellcheck-clean, never a download ==="
assert_eq "f6/shim" "the shim is present" "$([[ -f "$SHIM" ]] && echo yes)" "yes"
lines="$(wc -l < "$SHIM" | tr -d ' ')"
assert_eq "f6/shim" "the shim stays within 40 lines ($lines)" "$([[ "$lines" -le 40 ]] && echo ok)" "ok"
assert_contains "f6/shim" "the shim declares the POSIX shell" "$(head -1 "$SHIM")" "#!/bin/sh"
assert_not_contains "f6/shim" "the shim never downloads (no curl/wget/git clone)" "$(grep -nE 'curl|wget|git +clone|npm|pip' "$SHIM")" "curl"
assert_eq "f6/shim" "the shim is executable (hooks.json invokes it directly)" \
  "$([[ -x "$SHIM" ]] && echo yes)" "yes"
if command -v shellcheck >/dev/null 2>&1; then
  sc="$(shellcheck --shell=sh "$SHIM" 2>&1)"
  assert_eq "f6/shim" "shellcheck is silent over the shim (POSIX mode)" "$sc" ""
else
  echo "  note: shellcheck absent — the lint leg runs in CI"
fi

# The fallback tier is the SAME frozen file the parity oracles compare, so the
# shipped degrade path and the port's reference can never drift apart.
for ev in pre-tool-use-guard stop-gate; do
  fb="$SRC_ROOT/flow/bin/fallback/$ev.sh"
  if [[ -f "$fb" ]] && cmp -s "$fb" "$SRC_ROOT/flow/hooks-fallback/$ev.sh"; then
    pass "f6/shim" "bin/fallback/$ev.sh is the frozen oracle, byte for byte"
  else
    fail "f6/shim" "bin/fallback/$ev.sh is missing or has drifted from flow/hooks-fallback/$ev.sh"
  fi
done

echo ""
echo "=== f6/shim — five-tier discovery, both legs, higher tiers deleted ==="
# A sandbox whose .agents/skills/vibe is a real copy of flow/ (never a symlink to
# the live tree: the stop fixture writes a cursor). Path parity with the install
# layout is what the adapters see at a target.
SBX="$WORK/sbx"
mkdir -p "$SBX/flow" "$SBX/.agents/skills" "$SBX/.spec"
find "$SRC_ROOT/flow" -mindepth 1 -maxdepth 1 ! -name state.json \
  -exec cp -R {} "$SBX/flow/" \;
ln -s ../../flow "$SBX/.agents/skills/vibe"

# A relocatable plugin copy: bin/hook.sh (+ bin/fallback/*) exactly as a plugin
# ships. Tier 5 uses a copy WITHOUT the fallback directory.
mk_plugin() { # <dir> [with-fallback]
  mkdir -p "$1/bin"
  cp "$SHIM" "$1/bin/hook.sh"
  if [[ "${2:-yes}" == "yes" ]]; then
    mkdir -p "$1/bin/fallback"
    cp "$SRC_ROOT"/flow/bin/fallback/*.sh "$1/bin/fallback/"
  fi
}

# Fake binary per tier+event: it dumps its own environment (so the test can prove
# the higher-tier variable was DELETED, not blanked) and prints one proved block
# line naming itself and the event it was asked for.
mk_fake() { # <path> <label>
  local bin="$1" label="$2"
  mkdir -p "$(dirname "$bin")"
  {
    echo '#!/bin/sh'
    echo "env > \"$WORK/env.$label\""
    # "$2" must reach the fake literally — it is the fake's own second argument
    # (the event name), not something to expand here.
    # shellcheck disable=SC2016
    printf 'printf "block:%s %%s\\n" "$2"\n' "$label"
  } > "$bin"
  chmod +x "$bin"
}

# run_shim <plugin> <path> <event> <stdin-file> [env assignments...]
# Higher tiers are DELETED (-u) here, then the caller's assignments add back
# exactly the tier under test.
run_shim() {
  local plugin="$1" path="$2" event="$3" stdin="$4"; shift 4
  env -u VIBE_FLOW_BIN -u CLAUDE_PLUGIN_ROOT "$@" \
    CLAUDE_PROJECT_DIR="$SBX" PATH="$path" \
    sh "$plugin/bin/hook.sh" "$event" < "$stdin" 2>&1
  echo "rc=$?"
}
printf '' > "$WORK/empty"
printf '%s' '{"tool_name":"Write","tool_input":{"file_path":"docs/spec/lessons.md"}}' > "$WORK/guard.json"
printf '%s' '{}' > "$WORK/stop.json"

HOSTPATH="${PATH:-/usr/bin:/bin}"

# tier 1 — $VIBE_FLOW_BIN
P1="$WORK/p1"; mk_plugin "$P1"
mk_fake "$WORK/bin1/vibe-flow" tier1
for ev in pre-tool-use-guard stop-gate; do
  out="$(run_shim "$P1" "$HOSTPATH" "$ev" "$([[ $ev == stop-gate ]] && echo "$WORK/stop.json" || echo "$WORK/guard.json")" VIBE_FLOW_BIN="$WORK/bin1/vibe-flow")"
  assert_contains "f6/tier1" "\$VIBE_FLOW_BIN answers the $ev leg" "$out" "block:tier1 $ev"
  assert_contains "f6/tier1" "the $ev leg exits 2 on a proved block" "$out" "rc=2"
done
assert_contains "f6/tier1" "the child really saw the override (positive control)" "$(cat "$WORK/env.tier1")" "VIBE_FLOW_BIN=$WORK/bin1/vibe-flow"

# tier 2 — $CLAUDE_PLUGIN_ROOT/bin/<os>-<arch>/vibe-flow
P2="$WORK/p2"; mk_plugin "$P2"
mk_fake "$P2/bin/$OS-$ARCH/vibe-flow" tier2
for ev in pre-tool-use-guard stop-gate; do
  out="$(run_shim "$P2" "$HOSTPATH" "$ev" "$([[ $ev == stop-gate ]] && echo "$WORK/stop.json" || echo "$WORK/guard.json")" CLAUDE_PLUGIN_ROOT="$P2")"
  assert_contains "f6/tier2" "the bundled $OS-$ARCH binary answers the $ev leg" "$out" "block:tier2 $ev"
  assert_contains "f6/tier2" "the $ev leg exits 2 on a proved block" "$out" "rc=2"
done
assert_not_contains "f6/tier2" "VIBE_FLOW_BIN is DELETED from the tier-2 child env" "$(cat "$WORK/env.tier2")" "VIBE_FLOW_BIN="

# tier 3 — `vibe-flow` on PATH
P3="$WORK/p3"; mk_plugin "$P3"
mk_fake "$WORK/bin3/vibe-flow" tier3
for ev in pre-tool-use-guard stop-gate; do
  out="$(run_shim "$P3" "$WORK/bin3:$HOSTPATH" "$ev" "$([[ $ev == stop-gate ]] && echo "$WORK/stop.json" || echo "$WORK/guard.json")")"
  assert_contains "f6/tier3" "a vibe-flow on PATH answers the $ev leg" "$out" "block:tier3 $ev"
  assert_contains "f6/tier3" "the $ev leg exits 2 on a proved block" "$out" "rc=2"
done
env3="$(cat "$WORK/env.tier3")"
assert_not_contains "f6/tier3" "VIBE_FLOW_BIN is DELETED from the tier-3 child env" "$env3" "VIBE_FLOW_BIN="
assert_not_contains "f6/tier3" "CLAUDE_PLUGIN_ROOT is DELETED from the tier-3 child env" "$env3" "CLAUDE_PLUGIN_ROOT="

# tier 4 — the frozen bash fallback. PATH carries no vibe-flow and no node, so
# only bin/fallback/<event>.sh can answer, and it must still carry the hard block.
P4="$WORK/p4"; mk_plugin "$P4"
NOBIN="$WORK/nobin"
mkdir -p "$NOBIN"
for t in sh bash uname tr dirname grep sed cat head; do
  p="$(command -v "$t" 2>/dev/null)" && ln -sf "$p" "$NOBIN/$t"
done
# ...and a second farm with jq but still NO node: the frozen bash nudge below needs
# the machine's `next` array, which only the jq path can read.
FBJQ="$WORK/fbjq"
cp -R "$NOBIN" "$FBJQ"
jqp="$(command -v jq 2>/dev/null)" && ln -sf "$jqp" "$FBJQ/jq"
# tier 4 has two runtimes, in this order: the JS engine while it still ships
# (FV-R9 — the engine keeps running until F23, so the shim must not regress to
# pre-port behaviour while node is available), then the frozen bash fallback.
# The engine and the fallback disagree on exactly one thing this suite can see:
# the stuck-phase nudge (predicate 3) exists in the frozen fallback and was
# deleted from the engine. That divergence is the discriminator below — it fails
# if the tier order is flipped.
: > "$SBX/flow/warnings.log"
printf '{"flow":"feature","phase":"impl","feature":"demo","updated":"2026-01-01T00:00:00Z"}\n' \
  > "$SBX/flow/state.json"
out="$(run_shim "$P4" "$HOSTPATH" stop-gate "$WORK/stop.json")"
assert_contains "f6/tier4" "the JS engine answers the stop leg while it still ships" "$out" "rc=0"
assert_not_contains "f6/tier4" "the engine's stop gate queues no stuck-phase nudge (F23 shape)" \
  "$(cat "$SBX/flow/warnings.log")" "still in"
# ...and with node gone the frozen bash answers, which is why the engine tier
# exists: pre-port behaviour still nudges on every Stop.
out="$(run_shim "$P4" "$FBJQ" stop-gate "$WORK/stop.json")"
assert_contains "f6/tier4" "with node gone the frozen bash answers (pre-port nudge back)" \
  "$(cat "$SBX/flow/warnings.log")" "still in"
rm -f "$SBX/flow/state.json" "$SBX/flow/warnings.log"
# The guard's hard block, with no binary and no node anywhere on PATH.
out="$(run_shim "$P4" "$NOBIN" pre-tool-use-guard "$WORK/guard.json")"
assert_contains "f6/tier4" "the guard's hard block survives on the frozen fallback alone" "$out" "vibe-guard: BLOCKED"
assert_contains "f6/tier4" "the fallback guard block exits 2" "$out" "rc=2"
assert_not_contains "f6/tier4" "no node was reachable on that PATH (control)" "$(PATH="$NOBIN" command -v node 2>/dev/null)" "node"
assert_not_contains "f6/tier4" "no vibe-flow was reachable on that PATH (control)" "$(PATH="$NOBIN" command -v vibe-flow 2>/dev/null)" "vibe-flow"
# The stop gate's evidence tooth, same PATH: quick.verify with no receipt blocks.
mkdir -p "$SBX/flow/evidence"
printf '{"flow":"quick","phase":"verify","feature":null,"updated":"2026-01-01T00:00:00Z"}\n' \
  > "$SBX/flow/state.json"
out="$(run_shim "$P4" "$NOBIN" stop-gate "$WORK/stop.json")"
assert_contains "f6/tier4" "the evidence tooth survives on the frozen fallback alone" "$out" "vibe-gate: BLOCKED"
assert_contains "f6/tier4" "the fallback receipt block exits 2" "$out" "rc=2"
rm -f "$SBX/flow/state.json"

# tier 5 — nothing to run: exit 0 with exactly one stderr line, never a block.
P5="$WORK/p5"; mk_plugin "$P5" no
for ev in pre-tool-use-guard stop-gate; do
  out="$(run_shim "$P5" "$NOBIN" "$ev" "$WORK/empty")"
  assert_contains "f6/tier5" "an absent binary, engine and fallback still answers the $ev leg" "$out" "no vibe-flow binary, engine, or fallback"
  assert_contains "f6/tier5" "the $ev leg allows (exit 0) rather than blocking" "$out" "rc=0"
  assert_eq "f6/tier5" "the $ev leg prints exactly one stderr line" \
    "$(printf '%s\n' "$out" | grep -c 'no vibe-flow binary')" "1"
done

echo ""
echo "=== f6/shim — a block by output, never by exit code or by crash ==="
P6="$WORK/p6"; mk_plugin "$P6"
# (a) a crash: garbage on both streams, non-zero status -> allow, and the stack is
#     NOT relayed to the user.
mk_crash() {
  mkdir -p "$(dirname "$1")"
  { echo '#!/bin/sh'; echo 'echo "Traceback: block: fake"'; echo 'echo "panic: boom" >&2'; echo 'exit 3'; } > "$1"
  chmod +x "$1"
}
mk_crash "$WORK/bin6/vibe-flow"
out="$(run_shim "$P6" "$WORK/bin6:$HOSTPATH" pre-tool-use-guard "$WORK/empty")"
assert_contains "f6/exit" "a crashed child yields allow (exit 0)" "$out" "rc=0"
assert_not_contains "f6/exit" "a crashed child's stack trace is not relayed" "$out" "Traceback"
# ...and a crash with a payload that the frozen fallback blocks on still enforces
# the hard block: degrade to bash, never to nothing (js-core/7).
out="$(run_shim "$P6" "$WORK/bin6:$HOSTPATH" pre-tool-use-guard "$WORK/guard.json")"
assert_contains "f6/exit" "a crashed tier degrades to the frozen fallback, which still blocks" "$out" "rc=2"
assert_contains "f6/exit" "the degraded block is the guard's own message" "$out" "vibe-guard: BLOCKED"
# (b) exit code 2 with no proved line is still an allow — the code is not the proof.
{
  echo '#!/bin/sh'
  echo 'echo "something went wrong" >&2'
  echo 'exit 2'
} > "$WORK/bin6/vibe-flow"
chmod +x "$WORK/bin6/vibe-flow"
out="$(run_shim "$P6" "$WORK/bin6:$HOSTPATH" pre-tool-use-guard "$WORK/empty")"
assert_contains "f6/exit" "exit 2 with no proved block line is an allow" "$out" "rc=0"
# (c) one `block:<reason>` line (and exit 0!) is a block — the output is the proof.
{
  echo '#!/bin/sh'
  echo 'echo "block:only one proved line"'
} > "$WORK/bin6/vibe-flow"
chmod +x "$WORK/bin6/vibe-flow"
out="$(run_shim "$P6" "$WORK/bin6:$HOSTPATH" pre-tool-use-guard "$WORK/empty")"
assert_contains "f6/exit" "one \`block:<reason>\` line blocks even with exit 0" "$out" "rc=2"
assert_contains "f6/exit" "the proved reason is relayed" "$out" "block:only one proved line"
# (d) reasonless and banner-shaped verdicts never block. The anchor is the line
#     start: a stack trace that merely mentions a verdict shape is not one.
for garbage in 'block:' 'usage: vibe-flow hook <event>' 'Traceback: block:not at line start'; do
  {
    echo '#!/bin/sh'
    printf 'printf "%%s\\n" "%s"\n' "$garbage"
  } > "$WORK/bin6/vibe-flow"
  chmod +x "$WORK/bin6/vibe-flow"
  out="$(run_shim "$P6" "$WORK/bin6:$HOSTPATH" pre-tool-use-guard "$WORK/empty")"
  assert_contains "f6/exit" "unproved shape never blocks: $garbage" "$out" "rc=0"
done

echo ""
echo "=== f6/fail-safe — a present-but-corrupt policy still blocks a hard rule ==="
# Issue #26: "present-but-corrupt policy on guard/stop -> exit 2, pinned with a
# control case". The port answers policy data it cannot use by delegating to
# `detect-context.sh decide`, whose three hard blocks are coded, not data — so a
# truncated policy.json must still block the guarded path. The control is the
# same corrupt policy allowing an unguarded one: without it this leg would pass
# for a guard that blocks everything.
CORRUPT="$WORK/corrupt"
mkdir -p "$CORRUPT/flow" "$CORRUPT/.agents/skills" "$CORRUPT/.spec"
find "$SRC_ROOT/flow" -mindepth 1 -maxdepth 1 ! -name state.json \
  -exec cp -R {} "$CORRUPT/flow/" \;
ln -s ../../flow "$CORRUPT/.agents/skills/vibe"
printf '{' > "$CORRUPT/flow/content/policy.json"
printf '%s' '{"tool_name":"Write","tool_input":{"file_path":"docs/spec/lessons.md"}}' > "$WORK/c-guarded.json"
printf '%s' '{"tool_name":"Write","tool_input":{"file_path":"notes/todo.md"}}' > "$WORK/c-unGuarded.json"
P7="$WORK/p7"; mk_plugin "$P7"
run_corrupt() { # <payload-file>
  env -u VIBE_FLOW_BIN -u CLAUDE_PLUGIN_ROOT CLAUDE_PROJECT_DIR="$CORRUPT" \
    sh "$P7/bin/hook.sh" pre-tool-use-guard < "$1" 2>&1
  echo "rc=$?"
}
out="$(run_corrupt "$WORK/c-guarded.json")"
assert_contains "f6/fail-safe" "a corrupt policy still blocks a hard-ruled path" "$out" "vibe-guard: BLOCKED"
assert_contains "f6/fail-safe" "the corrupt-policy block exits 2" "$out" "rc=2"
out="$(run_corrupt "$WORK/c-unGuarded.json")"
assert_contains "f6/fail-safe" "CONTROL: the same corrupt policy still allows an unguarded path" "$out" "rc=0"
assert_not_contains "f6/fail-safe" "CONTROL: the corrupt policy did not become a block-everything guard" "$out" "BLOCKED"

echo ""
echo "=== f6/shim — nameless and unknown events read no stdin ==="
mkfifo "$WORK/fifo"
sleep 15 > "$WORK/fifo" &
HOLDER=$!
for ev in "" bogus-state; do
  sh "$SHIM" $ev < "$WORK/fifo" &
  probe=$!
  done_ok=no
  for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    kill -0 "$probe" 2>/dev/null || { done_ok=yes; break; }
    sleep 0.1
  done
  if [[ "$done_ok" == yes ]]; then
    wait "$probe"; rc=$?
    assert_eq "f6/stdin" "event '${ev:-<none>}' exits 0 without reading stdin" "$rc" "0"
  else
    kill "$probe" 2>/dev/null
    fail "f6/stdin" "event '${ev:-<none>}' blocked on stdin — it must not read one"
  fi
done
kill "$HOLDER" 2>/dev/null

echo ""
echo "=== f6/wiring — plugin hooks.json and the Claude adapter both route through the shim ==="
HJ="$SRC_ROOT/flow/hooks/hooks.json"
if [[ -f "$HJ" ]]; then
  pass "f6/wiring" "flow/hooks/hooks.json exists"
  assert_eq "f6/wiring" "hooks.json is valid JSON" "$(jq -e . "$HJ" >/dev/null 2>&1 && echo ok)" "ok"
  for pair in "PreToolUse pre-tool-use-guard" "Stop stop-gate"; do
    ev="${pair%% *}"; hook="${pair##* }"
    cmds="$(jq -r --arg e "$ev" '[.hooks[$e][]?.hooks[]?.command // empty] | join(" | ")' "$HJ")"
    assert_contains "f6/wiring" "$ev invokes the shim with '$hook'" "$cmds" "bin/hook.sh\" $hook"
    # the literal `${CLAUDE_PLUGIN_ROOT}` is what hooks.json must contain
    # shellcheck disable=SC2016
    assert_contains "f6/wiring" "$ev command is \${CLAUDE_PLUGIN_ROOT}-relative" "$cmds" '${CLAUDE_PLUGIN_ROOT}'
  done
  timeouts="$(jq -r '[.hooks[][]?.hooks[] | select(.timeout != 10)] | length' "$HJ")"
  assert_eq "f6/wiring" "every hooks.json entry carries timeout: 10" "$timeouts" "0"
  matcher="$(jq -r '.hooks.PreToolUse[0].matcher' "$HJ")"
  for tool in Edit Write NotebookEdit Bash; do
    assert_contains "f6/wiring" "the PreToolUse matcher still intercepts $tool" "$matcher" "$tool"
  done
else
  fail "f6/wiring" "flow/hooks/hooks.json is missing"
fi
# The adapter is a shim caller now, not a node-first engine caller.
for hook in pre-tool-use-guard stop-gate; do
  body="$(cat "$SRC_ROOT/.claude/hooks/$hook.sh")"
  assert_contains "f6/wiring" ".claude/hooks/$hook.sh routes through flow/bin/hook.sh" "$body" 'flow/bin/hook.sh'
  assert_not_contains "f6/wiring" ".claude/hooks/$hook.sh no longer execs the JS engine" "$body" "_engine-hook.sh"
done
# ...and the two events nothing else owns are still node-first: the flip is the
# flow half only (FV-R9 keeps the instruct flip at F12).
for hook in session-start-doctrine user-prompt-submit-inject; do
  assert_contains "f6/wiring" ".claude/hooks/$hook.sh still uses the engine shim" \
    "$(cat "$SRC_ROOT/.claude/hooks/$hook.sh")" "_engine-hook.sh"
done

echo ""
echo "=== f6/port-order — the gate before the TUI and the signals crate ==="
PO="$SRC_ROOT/tests/port-order.sh"
if [[ -f "$PO" ]]; then
  pass "f6/port-order" "tests/port-order.sh exists"
  # Two directions on a fixture tree: code ahead of the marker fails, the marker
  # clears it — and the marker alone (no code) never fails.
  FIX="$WORK/po"
  mkdir -p "$FIX/tests/parity" "$FIX/crates/vibe-flow/src/tui"
  out="$(bash "$PO" "$FIX" 2>&1; echo "rc=$?")"
  assert_contains "f6/port-order" "a tui/ dir without GREEN.hard fails the gate" "$out" "rc=1"
  assert_contains "f6/port-order" "the failure names the marker" "$out" "GREEN.hard"
  : > "$FIX/tests/parity/GREEN.hard"
  out="$(bash "$PO" "$FIX" 2>&1; echo "rc=$?")"
  assert_contains "f6/port-order" "the same tree passes once the marker exists" "$out" "rc=0"
  # The other code path: the signals crate (with the tui dir gone, so the failure
  # cannot be blamed on it).
  rm -f "$FIX/tests/parity/GREEN.hard"
  rm -rf "$FIX/crates/vibe-flow/src/tui"
  mkdir -p "$FIX/crates/vibe-signals/src"
  out="$(bash "$PO" "$FIX" 2>&1; echo "rc=$?")"
  assert_contains "f6/port-order" "a vibe-signals crate without GREEN.hard fails the gate" "$out" "rc=1"
  assert_not_contains "f6/port-order" "the signals failure does not blame the TUI" "$out" "src/tui"
  # This repo: the gate must pass here (no tui/signals yet, marker present).
  out="$(bash "$PO" "$SRC_ROOT" 2>&1; echo "rc=$?")"
  assert_contains "f6/port-order" "the gate passes on this tree" "$out" "rc=0"
else
  fail "f6/port-order" "tests/port-order.sh is missing"
fi

echo ""
echo "========================================"
echo "  $PASS passed, $FAIL failed"
if [[ "$FAIL" -ne 0 ]]; then
  echo "=== flow shim suite FAILED ==="
  exit 1
fi
echo "=== flow shim suite passed ==="
