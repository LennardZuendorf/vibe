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

# --- skill-revision/27 — repo docs -------------------------------------------
_README="$REPO_ROOT/README.md"
_AGENTS="$REPO_ROOT/AGENTS.md"
_FREADME="$REPO_ROOT/flow/README.md"
_FAGENTS="$REPO_ROOT/flow/AGENTS.md"
_flat() { tr -s '[:space:]' ' ' < "$1"; }
_has() { grep -Eq -- "$2" "$1"; }
_section() { awk -v h="$2" '$0 == h { f = 1; next } /^## / && f { exit } f' "$1"; }

_n=0
for _f in "$_README" "$_AGENTS" "$_FREADME" "$_FAGENTS"; do [[ -f "$_f" ]] && _n=$((_n + 1)); done
assert_population "skill-revision/27" "doc files examined" "$_n" 4

# Hook and invariant counts drift; no doc states one.
_count_re='(^|[^A-Za-z])([Tt]hree|[Ff]our|[Ff]ive|[Nn]ine|[Tt]en) (hook|hooks|scripts|write invariants|that block|that warn)'
_hits=0
for _f in "$_README" "$_AGENTS" "$_FREADME" "$_FAGENTS"; do _has "$_f" "$_count_re" && _hits=$((_hits + 1)); done
assert_eq "skill-revision/27" "no doc states a hook, script or invariant count" "0" "$_hits"
_hits=0
for _f in "$_README" "$_AGENTS" "$_FREADME" "$_FAGENTS"; do _has "$_f" 'validate\.sh' && _hits=$((_hits + 1)); done
assert_eq "skill-revision/27" "no doc names validate.sh" "0" "$_hits"

# README claims
_has "$_README" 'no build step|no runtime' && _r=1 || _r=0
assert_eq "skill-revision/27" "README has no 'no build step' / 'no runtime' claim" "0" "$_r"
_has "$_README" 'built%20with-[^)]*[Rr]ust' && _r=0 || _r=1
assert_eq "skill-revision/27" "README built-with badge names Rust" "0" "$_r"
_has "$_README" 'F1–F24|create-issues\.sh' && _r=1 || _r=0
assert_eq "skill-revision/27" "README has no F1–F24 count and no create-issues.sh" "0" "$_r"
_has "$_README" 'cargo install --locked' && _r=0 || _r=1
assert_eq "skill-revision/27" "README installs vibe-spec with cargo install --locked" "0" "$_r"
assert_contains "skill-revision/27" "README tells how to check the binary" "$(_flat "$_README")" 'vibe-spec --version'
assert_contains "skill-revision/27" "README says install.sh does not ship the binary" "$(_flat "$_README")" '`install.sh` does not ship the binary yet'
assert_contains "skill-revision/27" "README shipped heading names the dev build" "$(cat "$_README")" 'v0.3.x, plus the vibe-spec dev build'
_has "$_README" '^#+ Install' && _r=0 || _r=1
assert_eq "skill-revision/27" "README has an Install section" "0" "$_r"

# flow README claims
_has "$_FREADME" 'no `?/spec`? command' && _r=1 || _r=0
assert_eq "skill-revision/27" "flow README does not deny a /spec command" "0" "$_r"
_nscripts=0; _miss=""
for _s in "$REPO_ROOT"/flow/scripts/*; do
  [[ -f "$_s" ]] || continue
  _nscripts=$((_nscripts + 1))
  grep -qF "[$(basename "$_s")](scripts/$(basename "$_s"))" "$_FREADME" || _miss="$_miss $(basename "$_s")"
done
assert_population "skill-revision/27" "flow scripts examined" "$_nscripts" 10
assert_eq "skill-revision/27" "flow README script table lists every script" "" "$_miss"
for _d in 'content/' 'engine/' 'hooks-fallback/'; do
  assert_contains "skill-revision/27" "flow AGENTS lists $_d" "$(cat "$_FAGENTS")" "\`$_d\`"
done

# Orders arrive on the turn after the cursor moves; orders.sh recovers them.
for _f in "$_README" "$_FREADME"; do
  assert_contains "skill-revision/27" "${_f#"$REPO_ROOT"/} says when orders arrive" "$(_flat "$_f")" 'on the turn after the cursor moves'
  assert_contains "skill-revision/27" "${_f#"$REPO_ROOT"/} says orders.sh recovers them" "$(_flat "$_f")" 'recovers them'
done

# The file-tool list given for the flow guard equals the matcher in merge-settings.sh.
_matcher="$(sed -n 's/^#.*PreToolUse .*(matcher \([A-Za-z|]*\)).*/\1/p' "$REPO_ROOT/flow/scripts/merge-settings.sh" | head -n 1)"
_want="($(printf '%s' "$_matcher" | tr '|' '/'))"
assert_eq "skill-revision/27" "matcher read from merge-settings.sh" "(Edit/Write/NotebookEdit/Bash)" "$_want"
_lists=0; _bad=""
for _f in "$_README" "$_AGENTS" "$_FREADME" "$_FAGENTS"; do
  while IFS= read -r _l; do
    [[ -z "$_l" ]] && continue
    _lists=$((_lists + 1))
    [[ "$_l" == "$_want" ]] || _bad="$_bad $_f:$_l"
  done <<EOL
$(grep -Eo '\((Edit|Write|NotebookEdit|Bash)(/(Edit|Write|NotebookEdit|Bash))+\)' "$_f")
EOL
done
assert_population "skill-revision/27" "guard file-tool lists examined" "$_lists" 2
assert_eq "skill-revision/27" "every guard file-tool list equals the matcher" "" "$_bad"

# AGENTS.md user-owned sections
_route="$(_section "$_AGENTS" '## Task routing' | tr -s '[:space:]' ' ')"
assert_contains "skill-revision/27" "Task routing names the superpowers spec path" "$_route" 'docs/superpowers/specs/*-<feature>-design.md'
assert_contains "skill-revision/27" "Task routing names the superpowers plan path" "$_route" 'plans/*-<feature>.md'
assert_contains "skill-revision/27" "Task routing keeps the feature folder as the fallback" "$_route" 'fallback'
assert_contains "skill-revision/27" "Task routing names the fallback folder" "$_route" 'docs/spec/features/<feature>/'
_layout="$(_section "$_AGENTS" '## Repo layout')"
for _d in 'flow/' 'spec/' 'crates/' 'contracts/' 'oracles/' 'tests/' 'docs/superpowers/'; do
  assert_contains "skill-revision/27" "Repo layout lists $_d" "$_layout" "$_d"
done
_sl="$(_section "$_AGENTS" '## Spec layout' | tr -s '[:space:]' ' ' | tr '[:upper:]' '[:lower:]')"
assert_contains "skill-revision/27" "Spec layout names hubs" "$_sl" 'hub'
assert_contains "skill-revision/27" "Spec layout names lessons.md" "$_sl" 'lessons.md'
assert_contains "skill-revision/27" "Spec layout names kind folders" "$_sl" 'kind folder'
assert_contains "skill-revision/27" "Boundaries name the real done-check" "$(_flat "$_AGENTS")" 'Run `vibe-spec validate --strict` before claiming work is done'
_cl="$(grep -c 'Track E (shipped instructions and repo docs): pending' "$REPO_ROOT/CHANGELOG.md" || true)"
assert_eq "skill-revision/27" "CHANGELOG Track E line is rewritten" "0" "$_cl"
