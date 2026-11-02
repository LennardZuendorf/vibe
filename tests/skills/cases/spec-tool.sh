# shellcheck shell=bash
# tests/skills/cases/spec-tool.sh — Track C (vibe-spec) text checks (skill-revision/12, 17).
# Sourced by run.sh after lib.sh. The behaviour itself is pinned by the crate
# tests; these checks read the built tool's own help.

_vs() { cargo run -q --locked -p vibe-spec -- "$@" 2>&1; }

_help="$(_vs --help)"
assert_contains "skill-revision/12" "--help names VIBE_SPEC_ROOT" "$_help" "VIBE_SPEC_ROOT"
assert_contains "skill-revision/12" "--help lists the exit codes" "$_help" "2 usage"
case "$_help" in
  *"port of"*) fail "skill-revision/12" "--help drops port history" ;;
  *) pass "skill-revision/12" "--help drops port history" ;;
esac
_n=0
for _c in list-specs lessons-for scan-merges promote; do
  _out="$(_vs "$_c" --help)"
  case "$_out" in
    "Usage: vibe-spec $_c"*) _n=$((_n + 1)) ;;
    *) fail "skill-revision/12" "$_c --help prints usage" ;;
  esac
done
assert_population "skill-revision/12" "raw-tail commands with help" "$_n" 4

# --- skill-revision/17: the SessionStart router is silent without a spec root --
_tmp="$(mktemp -d)"
mkdir "$_tmp/.git"
_bin="$REPO_ROOT/target/debug/vibe-spec"
[[ -x "$_bin" ]] || { cargo build -q --locked -p vibe-spec 2>/dev/null; }
_out="$(cd "$_tmp" && printf '{}' | "$_bin" hook session-start 2>/dev/null)"
assert_eq "skill-revision/17" "session-start prints {} in a repo with no spec root" "{}" "$_out"
mkdir -p "$_tmp/docs/spec"
printf '# Spec\n' > "$_tmp/docs/spec/README.md"
_out="$(cd "$_tmp" && printf '{}' | "$_bin" hook session-start 2>/dev/null)"
assert_contains "skill-revision/17" "session-start routes when a spec root exists" "$_out" "using-spec"
rm -rf "$_tmp"
