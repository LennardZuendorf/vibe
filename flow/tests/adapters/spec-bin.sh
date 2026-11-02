#!/usr/bin/env bash
# vibe-spec: literal-file
# flow/tests/adapters/spec-bin.sh — vibe#71 sub-gap 4, end to end.
#
# The shipped `spec` skill shells out to `vibe-spec`; nothing used to install the
# binary, so a global-install user had skills calling a command they do not have
# and no checkout to build it in. The chosen path (see the PR body) is an
# explicit "no binary without cargo" degradation — install.sh REPORTS the status
# and never fails, `doctor` reports it as a warn, the skill names what still
# works — plus an OPT-IN local build (`--build-spec-bin`) for a user who does
# have cargo. This leg proves both halves against a throwaway HOME, and proves
# the fallback it advertises actually validates a flat legacy root.
#
# Hermetic: every leg runs with its own temp HOME, from a temp working dir (so
# the per-repo seed cannot fire), on a PATH farm that carries no host CLI and no
# cargo unless the leg puts a stub there. The ambient machine decides nothing.
#
#   bash flow/tests/adapters/spec-bin.sh

# pass/fail always return 0, so `cond && pass || fail` cannot misfire.
# shellcheck disable=SC2015
set -uo pipefail

_find_repo_root() {
  local d; d="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  while [[ "$d" != "/" ]]; do
    [[ -d "$d/docs/spec" || -d "$d/.spec" || -e "$d/.git" ]] && { printf '%s\n' "$d"; return 0; } # literal
    d="$(dirname "$d")"
  done
  return 1
}
REPO_ROOT="$(_find_repo_root)" || { echo "cannot locate repo root" >&2; exit 1; }
INSTALL="$REPO_ROOT/install.sh"
DOCTOR="$REPO_ROOT/flow/scripts/doctor.sh"
TAG="spec-bin/71"

PASS=0 FAIL=0
pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }
assert_contains()     { if [[ "$3" == *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want contains: $4"; fi; }
assert_not_contains() { if [[ "$3" != *"$4"* ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want NOT contains: $4"; fi; }
assert_eq()           { if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else fail "$1" "$2"; echo "        want: $4"; echo "        got:  $3"; fi; }

tmp() { mktemp -d "${TMPDIR%/}/vibe-specbin.XXXXXX"; }

# mkfarm — a broad POSIX toolset and NO cargo, NO vibe-spec, NO host CLI. The
# list matches global-install.sh's farm: a leg that needs an optional tool adds a
# stub itself, so a pass never depends on what this machine happens to have.
mkfarm() {
  local dir; dir="$(tmp)"
  local t p
  for t in bash sh mkdir dirname basename date mktemp mv cp rm rmdir sed grep head tail cat env awk find readlink ln chmod cmp diff sort cksum node git id uname touch; do
    p="$(command -v "$t" 2>/dev/null)" && ln -s "$p" "$dir/$t"
  done
  printf '%s\n' "$dir"
}

# run_install HOME PATHSTR ARGS... — invoke the installer the way a user does,
# with every ambient host override removed so the HOME under test is the only
# state in play. Prints combined output; returns the installer's status.
run_install() {
  local home="$1" pathstr="$2"; shift 2
  local work; work="$(tmp)"
  ( cd "$work" && env -u CLAUDE_CONFIG_DIR -u CODEX_HOME -u FACTORY_HOME \
      -u XDG_CONFIG_HOME -u CLAUDE_PROJECT_DIR -u VIBE_SPEC_BIN \
      HOME="$home" PATH="$pathstr" XDG_CONFIG_HOME="$home/.config" \
      bash "$INSTALL" "$@" 2>&1 )
  local rc=$?
  return $rc
}

echo "=== $TAG — install.sh and doctor tell the truth about the vibe-spec binary ==="

FARM="$(mkfarm)"; trap 'rm -rf "$FARM"' EXIT

# An EMPTY CARGO_TARGET_DIR for every leg that expects the binary to be absent:
# this repo's own target/ may hold a real build, and a leg that silently picked
# it up would pass on a developer machine and fail on a clean one.
export CARGO_TARGET_DIR; CARGO_TARGET_DIR="$(tmp)"

# ── 1. absent binary: reported, non-fatal, and the skills still install ──────
H="$(tmp)"
out="$(run_install "$H" "$FARM" --global --host codex)"; rc=$?
assert_eq "$TAG" "absent binary: --global still exits 0 (a missing binary degrades, it does not fail)" "$rc" "0"
assert_contains "$TAG" "absent binary: the status line says NOT installed" "$out" "vibe-spec: NOT installed"
assert_contains "$TAG" "absent binary: it names the fallback that still validates a flat legacy root" "$out" "<skill>/scripts/validate.sh"
assert_contains "$TAG" "absent binary: it names the install route" "$out" "cargo install --locked --path crates/vibe-spec"
[[ -f "$H/.codex/skills/spec/SKILL.md" ]] \
  && pass "$TAG" "absent binary: the skills are still installed" \
  || fail "$TAG" "absent binary: the skills are still installed"

# ── 2. binary on PATH: reported as installed, with no degradation text ───────
H2="$(tmp)"; P2="$(mkfarm)"
printf '#!/bin/sh\necho "vibe-spec 0.4.0"\n' > "$P2/vibe-spec"; chmod +x "$P2/vibe-spec"
out="$(run_install "$H2" "$P2" --global --host codex)"      # a fresh HOME makes the first write visible
assert_contains "$TAG" "present binary: the status line names the binary it found" "$out" "vibe-spec: installed ($P2/vibe-spec)"
assert_not_contains "$TAG" "present binary: no degradation paragraph" "$out" "NOT installed"
out="$(run_install "$H2" "$P2" --global --host codex --dry-run)"
assert_contains "$TAG" "present binary: --dry-run reports the same status" "$out" "vibe-spec: installed ($P2/vibe-spec)"

# ── 3. --build-spec-bin without cargo: a WARN, not a failure ────────────────
H3="$(tmp)"
out="$(run_install "$H3" "$FARM" --global --host codex --build-spec-bin)"; rc=$?
assert_eq "$TAG" "--build-spec-bin without cargo exits 0 (degrade, never hard-fail)" "$rc" "0"
assert_contains "$TAG" "--build-spec-bin without cargo says what is missing" "$out" "needs cargo on PATH"
assert_contains "$TAG" "--build-spec-bin without cargo still reports the binary absent" "$out" "vibe-spec: NOT installed"
[[ ! -e "$H3/.local/bin/vibe-spec" ]] \
  && pass "$TAG" "--build-spec-bin without cargo writes no binary" \
  || fail "$TAG" "--build-spec-bin without cargo writes no binary"

# ── 4. --build-spec-bin with cargo: builds, installs, notes PATH ─────────────
# A stub cargo writes the artifact where the installer looks ($CARGO_TARGET_DIR),
# so this proves the copy/install/report path without a 90-second real build. The
# real build was exercised by hand on this host; see the PR body.
H4="$(tmp)"; P4="$(mkfarm)"; BIN4="$H4/.local/bin"; TD4="$(tmp)"
cat > "$P4/cargo" <<'STUB'
#!/bin/sh
# stub cargo: create the release artifact the installer copies
mkdir -p "$CARGO_TARGET_DIR/release"
printf '#!/bin/sh\necho "vibe-spec 9.9.9"\n' > "$CARGO_TARGET_DIR/release/vibe-spec"
chmod 755 "$CARGO_TARGET_DIR/release/vibe-spec"
exit 0
STUB
chmod +x "$P4/cargo"
out="$(CARGO_TARGET_DIR="$TD4" run_install "$H4" "$P4" --global --host codex --build-spec-bin --bin-dir "$BIN4")"; rc=$?
assert_eq "$TAG" "--build-spec-bin exits 0" "$rc" "0"
assert_contains "$TAG" "--build-spec-bin reports the installed binary and its version" "$out" "vibe-spec: installed ($BIN4/vibe-spec, vibe-spec 9.9.9)"
assert_contains "$TAG" "--build-spec-bin notes a bin dir that is not on PATH" "$out" "$BIN4 is not on PATH"
if [[ -x "$BIN4/vibe-spec" ]]; then
  assert_eq "$TAG" "--build-spec-bin put a working binary at --bin-dir" "$("$BIN4/vibe-spec" --version)" "vibe-spec 9.9.9"
else
  fail "$TAG" "--build-spec-bin put a working binary at --bin-dir (missing $BIN4/vibe-spec)"
fi
assert_not_contains "$TAG" "--build-spec-bin does not also print the degradation paragraph" "$out" "NOT installed"
out="$(CARGO_TARGET_DIR="$TD4" run_install "$H4" "$P4" --global --host codex --build-spec-bin --bin-dir "$BIN4" --dry-run)"
assert_contains "$TAG" "--build-spec-bin --dry-run states the plan and builds nothing" "$out" "would build vibe-spec (cargo build --release -p vibe-spec)"

# ── 5. never clobber a foreign file at the destination ──────────────────────
H5="$(tmp)"; P5="$(mkfarm)"; BIN5="$H5/.local/bin"; TD5="$(tmp)"
cp "$P4/cargo" "$P5/cargo"
mkdir -p "$BIN5"; printf '#!/bin/sh\necho "not ours"\n' > "$BIN5/vibe-spec"; chmod +x "$BIN5/vibe-spec"
out="$(CARGO_TARGET_DIR="$TD5" run_install "$H5" "$P5" --global --host codex --build-spec-bin --bin-dir "$BIN5")"; rc=$?
assert_eq "$TAG" "a foreign file at the destination does not fail the install" "$rc" "0"
assert_contains "$TAG" "a foreign file at the destination is left alone, with a warning" "$out" "exists and is not a vibe-spec binary"
assert_eq "$TAG" "the foreign file is unchanged" "$("$BIN5/vibe-spec")" "not ours"

# ── 6. --bin-dir needs a value ──────────────────────────────────────────────
out="$(run_install "$H5" "$FARM" --global --bin-dir)"; rc=$?
assert_eq "$TAG" "--bin-dir without a value is a usage error" "$rc" "1"
assert_contains "$TAG" "--bin-dir without a value says what it needs" "$out" "--bin-dir needs a directory"

# --bin-dir alone names no binary to move: say so rather than reading as done.
out="$(run_install "$H5" "$FARM" --global --host codex --bin-dir "$H5/bins")"; rc=$?
assert_eq "$TAG" "--bin-dir without --build-spec-bin still exits 0" "$rc" "0"
assert_contains "$TAG" "--bin-dir without --build-spec-bin says it is ignored" "$out" "--bin-dir only applies with --build-spec-bin"

# ── 7. doctor reports the binary, present and absent, and never fails ───────
ROOT7="$(tmp)"; mkdir -p "$ROOT7"
out="$(env -u VIBE_SPEC_BIN PATH="$FARM" bash "$DOCTOR" "$ROOT7" 2>&1)"; rc=$?
assert_eq "$TAG" "doctor exits 0 with no binary (warn-only by design)" "$rc" "0"
assert_contains "$TAG" "doctor warns that vibe-spec is missing" "$out" "warn tool.vibe-spec vibe-spec not installed"
assert_contains "$TAG" "doctor names the script fallback path" "$out" "bash $ROOT7/.agents/skills/spec/scripts/validate.sh"
out="$(VIBE_SPEC_BIN="$P2/vibe-spec" PATH="$FARM" bash "$DOCTOR" "$ROOT7" 2>&1)"; rc=$?
assert_eq "$TAG" "doctor exits 0 with the binary present" "$rc" "0"
assert_contains "$TAG" "doctor reports the binary present via \$VIBE_SPEC_BIN" "$out" "ok   tool.vibe-spec vibe-spec present ($P2/vibe-spec)"

# ── 8. the advertised fallback really validates a flat legacy root ──────────
# The skill's absence paragraph promises the bundled bash scripts still work on a
# flat legacy tree. Prove the promise instead of trusting the prose.
H8="$(tmp)"
( cd "$H8" && bash "$REPO_ROOT/spec/scripts/setup.sh" ) >/dev/null 2>&1; rc=$?
assert_eq "$TAG" "flat legacy root fixture seeds (setup.sh)" "$rc" "0"
out="$( cd "$H8" && bash "$REPO_ROOT/spec/scripts/validate.sh" 2>&1 )"; rc=$?
assert_eq "$TAG" "the bundled validate.sh validates a flat legacy root with no binary" "$rc" "0"
assert_contains "$TAG" "its report is a real validation summary" "$out" "Errors:   0"

# ── 9. the shipped skill states the degradation (prose pin) ────────────────
# A behaviour leg cannot see what the agent is told; this pins the two sentences
# that make the absence non-fatal instead of an error.
SKILL="$REPO_ROOT/spec/SKILL.md"
assert_contains "$TAG" "spec/SKILL.md says the binary is absent from a plain install" "$(cat "$SKILL")" "A plain install ships no \`vibe-spec\`"
assert_contains "$TAG" "spec/SKILL.md forbids claiming a pass without validation" "$(cat "$SKILL")" "never claim a pass"
assert_contains "$TAG" "spec/SKILL.md names the bundled scripts as the fallback" "$(cat "$SKILL")" "{validate,check-drift,list-specs}.sh"

echo ""
echo "=== results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
