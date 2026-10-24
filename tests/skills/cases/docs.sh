# shellcheck shell=bash
# shellcheck disable=SC2016  # backticks in the expected doc text are literal
# tests/skills/cases/docs.sh — Track E: shipped instructions and repo docs
# (skill-revision/25-27). Sourced by run.sh after lib.sh. Owned by Track E.

_ENGINE="$REPO_ROOT/flow/engine/cli.mjs"

# --- skill-revision/25 — neutral delegation defaults -------------------------
_dt="$(mktemp -d)"
_cleanup_docs() { rm -rf "$_dt"; }
trap _cleanup_docs EXIT
mkdir -p "$_dt/.agents/skills"
ln -s "$REPO_ROOT/flow" "$_dt/.agents/skills/vibe"
_shipped="$(CLAUDE_PROJECT_DIR="$_dt" node "$_ENGINE" render agents-md 2>&1)"

for _tier in haiku sonnet opus; do
  assert_contains "skill-revision/25" "shipped agents-md names $_tier" "$_shipped" "\`$_tier\`"
done
for _w in '\bme\b' '\bmy\b' 'fable' 'ultracode'; do
  if printf '%s\n' "$_shipped" | grep -Eiq "$_w"; then
    fail "skill-revision/25" "shipped agents-md has no personal wording ($_w)"
  else
    pass "skill-revision/25" "shipped agents-md has no personal wording ($_w)"
  fi
done
assert_contains "skill-revision/25" "shipped agents-md keeps delegation.workflows" "$_shipped" "Dynamic workflows"
_dr="$(grep -c 'render agents-md' "$REPO_ROOT/flow/content/vibe.default.json" || true)"
_dr_bad="$(grep -E 'vibe render agents-md' "$REPO_ROOT/flow/content/vibe.default.json" | grep -vc 'cli.mjs render agents-md' || true)"
assert_eq "skill-revision/25" "default config has no bare 'vibe render agents-md'" "0" "$_dr_bad"
assert_population "skill-revision/25" "default config render commands examined" "$_dr" 2
_repo_render="$(CLAUDE_PROJECT_DIR="$REPO_ROOT" node "$_ENGINE" render agents-md 2>&1)"
assert_contains "skill-revision/25" "this repo's render keeps the Fable rule" "$_repo_render" "fable"
assert_contains "skill-revision/25" "this repo's render keeps the ultracode rule" "$_repo_render" "ultracode"
_lines="$(printf '%s\n' "$_repo_render" | wc -l | tr -d ' ')"
if [[ "$_lines" -le 80 ]]; then pass "skill-revision/25" "repo render is <=80 lines ($_lines)"; else fail "skill-revision/25" "repo render is <=80 lines ($_lines)"; fi
_shl="$(printf '%s\n' "$_shipped" | wc -l | tr -d ' ')"
if [[ "$_shl" -le 80 ]]; then pass "skill-revision/25" "shipped render is <=80 lines ($_shl)"; else fail "skill-revision/25" "shipped render is <=80 lines ($_shl)"; fi
node "$_ENGINE" render --check >/dev/null 2>&1; assert_rc "skill-revision/25" "render --check exits 0" 0 "$?"

# --- skill-revision/26 — template and managed block --------------------------
_tmpl="$REPO_ROOT/flow/reference/templates/AGENTS.md"
_tt="$(tr -s '[:space:]' ' ' < "$_tmpl")"
for _gone in 'The gate teeth live in' 'Approvals are enforced by' 'until F22 ships the binary'; do
  if [[ "$_tt" == *"$_gone"* ]]; then fail "skill-revision/26" "template drops '$_gone'"; else pass "skill-revision/26" "template drops '$_gone'"; fi
done
assert_contains "skill-revision/26" "template says gates are a convention" "$_tt" \
  'Gates are a convention today: `/flow` asks for a `confirm` token, and no hook or script checks it.'
assert_contains "skill-revision/26" "template says the writer does not gate" "$_tt" \
  '`set-state.sh` writes the cursor; it does not gate. No hook enforces the two approvals; honor them yourself.'
assert_contains "skill-revision/26" "template working model line" "$_tt" \
  'design → plan →[gate] impl → verify →[gate] compound'
_cmds="$(awk '/^## Commands/{f=1;next} /^## /{f=0} f' "$_tmpl")"
assert_contains "skill-revision/26" "Commands block names vibe-spec validate" "$_cmds" 'vibe-spec validate'
if printf '%s\n' "$_cmds" | grep -Eq 'legacy|validate\.sh'; then
  fail "skill-revision/26" "Commands block has no legacy hedge"
else pass "skill-revision/26" "Commands block has no legacy hedge"; fi
_blk() { awk '/<!-- vibe:instructions:start -->/{f=1} f{print} /<!-- vibe:instructions:end -->/{f=0}' "$1"; }
_d="$(diff <(_blk "$REPO_ROOT/AGENTS.md") <(_blk "$_tmpl") | wc -l | tr -d ' ')"
assert_eq "skill-revision/26" "AGENTS.md vibe:instructions block equals the template" "0" "$_d"
_hand="$(awk '/<!-- vibe:instructions:start -->/{f=1} /<!-- vibe:rules -->/{f=0} f' "$_tmpl")"
_states="$(printf '%s\n' "$_hand" | grep -Eo '\b(feature|strategy|setup|quick)\.[a-z]+' | sort -u | tr '\n' ' ')"
assert_eq "skill-revision/26" "template hand-authored half names only setup.apply" "setup.apply " "$_states"
node "$_ENGINE" render --check >/dev/null 2>&1; assert_rc "skill-revision/26" "render --check exits 0" 0 "$?"
