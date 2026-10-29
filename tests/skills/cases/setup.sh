# shellcheck shell=bash
# tests/skills/cases/setup.sh — self-tests for tests/skills/lib.sh (skill-revision/1).
# Sourced by run.sh after lib.sh. Each lint is checked against a violating
# fixture and a passing control.

_t="$(mktemp -d)"
_cleanup_setup() { rm -rf "$_t"; }
trap _cleanup_setup EXIT

# --- lint_description --------------------------------------------------------
cat > "$_t/good.md" <<'FIX'
---
name: good
description: Use when a repo keeps a spec and the work changes what the product does, so the durable record must be read first.
---
body
FIX
cat > "$_t/bad.md" <<'FIX'
---
name: bad
description: Routes to the right phase for <feature> work.
---
body
FIX
cat > "$_t/assertive.md" <<'FIX'
---
name: assertive
description: |
  You MUST use this before changing what the product does,
  including one-line fixes.
---
body
FIX
cat > "$_t/first.md" <<'FIX'
---
name: first
description: Use when I need to check my notes.
---
body
FIX

out="$(lint_description "$_t/good.md")"; rc=$?
assert_rc "skill-revision/1" "lint_description passes a good description" 0 "$rc"
assert_eq "skill-revision/1" "lint_description prints nothing for a good description" "" "$out"

out="$(lint_description "$_t/bad.md")"; rc=$?
assert_rc "skill-revision/1" "lint_description fails a bad description" 1 "$rc"
assert_eq "skill-revision/1" "lint_description names two violations" "2" "$(printf '%s\n' "$out" | grep -c .)"

lint_description "$_t/assertive.md" assertive >/dev/null; rc=$?
assert_rc "skill-revision/1" "assertive mode skips the Use when check" 0 "$rc"
lint_description "$_t/assertive.md" >/dev/null; rc=$?
assert_rc "skill-revision/1" "default mode requires Use when" 1 "$rc"
lint_description "$_t/first.md" >/dev/null; rc=$?
assert_rc "skill-revision/1" "first person is a violation" 1 "$rc"

long="$(printf 'x%.0s' $(seq 1 520))"
printf -- '---\nname: long\ndescription: Use when %s\n---\n' "$long" > "$_t/long.md"
out="$(lint_description "$_t/long.md")"; rc=$?
assert_rc "skill-revision/1" "over 500 chars is a violation" 1 "$rc"
assert_contains "skill-revision/1" "the violation names the length" "$out" "500"

# --- lint_frontmatter ---------------------------------------------------------
out="$(lint_frontmatter "$_t/good.md" "name description" good)"; rc=$?
assert_rc "skill-revision/1" "lint_frontmatter passes a clean file" 0 "$rc"
printf -- '---\nname: x\ndescription: Use when y\nbogus: 1\n---\n' > "$_t/extra.md"
out="$(lint_frontmatter "$_t/extra.md" "name description" x)"; rc=$?
assert_rc "skill-revision/1" "lint_frontmatter fails an extra key" 1 "$rc"
assert_contains "skill-revision/1" "the extra key is named" "$out" "bogus"
out="$(lint_frontmatter "$_t/good.md" "name description" other)"; rc=$?
assert_rc "skill-revision/1" "lint_frontmatter fails a wrong name" 1 "$rc"
printf 'no frontmatter\n' > "$_t/nofm.md"
lint_frontmatter "$_t/nofm.md" "name description" x >/dev/null; rc=$?
assert_rc "skill-revision/1" "lint_frontmatter fails missing delimiters" 1 "$rc"

# --- words_outside ------------------------------------------------------------
cat > "$_t/words.md" <<'FIX'
---
name: w
description: Use when counting.
---
one two three
<!-- vibe:doctrine -->
inside the doctrine block these words do not count at all
<!-- /vibe:doctrine -->
four five
<!-- vibe:orders:idle -->
orders words do not count either
<!-- /vibe:orders -->
FIX
assert_eq "skill-revision/1" "words_outside counts only words outside frontmatter and blocks" "5" "$(words_outside "$_t/words.md")"

# --- pressure_ok --------------------------------------------------------------
mkp() { # <file> <red reps> <green reps> <verdict>
  { echo "# skill-revision/1 — fixture"; echo "Pressures: time, sunk cost, authority"; echo "## RED (without the change)"
    local i=1; while [[ "$i" -le "$2" ]]; do echo "- rep $i: A — \"quote\""; i=$((i + 1)); done
    echo "## GREEN (with the change)"
    i=1; while [[ "$i" -le "$3" ]]; do echo "- rep $i: B — \"quote\""; i=$((i + 1)); done
    echo "## Verdict"; echo "$4"; } > "$1"
}
mkp "$_t/red-only.md" 5 0 "pending"
pressure_ok "$_t/red-only.md"; rc=$?
assert_rc "skill-revision/1" "pressure_ok fails a RED-only file" 1 "$rc"
mkp "$_t/full.md" 5 5 "GREEN 5/5"
pressure_ok "$_t/full.md"; rc=$?
assert_rc "skill-revision/1" "pressure_ok passes RED and GREEN with 5 reps each" 0 "$rc"
mkp "$_t/norred.md" 5 0 "NO-RED"
pressure_ok "$_t/norred.md"; rc=$?
assert_rc "skill-revision/1" "pressure_ok passes a NO-RED verdict" 0 "$rc"
mkp "$_t/fewred.md" 4 5 "GREEN 5/5"
pressure_ok "$_t/fewred.md"; rc=$?
assert_rc "skill-revision/1" "pressure_ok fails fewer than 5 RED reps" 1 "$rc"
printf 'not a scenario\n' > "$_t/nohdr.md"
pressure_ok "$_t/nohdr.md"; rc=$?
assert_rc "skill-revision/1" "pressure_ok fails a file without the header line" 1 "$rc"

# --- fixed_root_hits ----------------------------------------------------------
printf 'write docs/spec/plan.md\n' > "$_t/hit.md"
assert_eq "skill-revision/1" "fixed_root_hits counts a fixed target" "1" "$(fixed_root_hits "$_t/hit.md" | grep -c .)"
printf 'without the binary docs/spec/ when it exists\nthe spec root (docs/spec) is neutral\nnothing here\n' > "$_t/ok.md"
assert_eq "skill-revision/1" "fixed_root_hits skips the defining lines" "0" "$(fixed_root_hits "$_t/ok.md" | grep -c .)"

# --- links --------------------------------------------------------------------
mkdir -p "$_t/repo/spec/sub" "$_t/repo/flow" "$_t/repo/.agents/skills"
ln -s ../../spec "$_t/repo/.agents/skills/spec"
ln -s ../../flow "$_t/repo/.agents/skills/vibe"
printf 'x\n' > "$_t/repo/spec/sub/a.md"
cat > "$_t/repo/spec/SKILL.md" <<'FIX'
See [a](sub/a.md#frag), [web](https://example.com/x), [anchor](#top) and [gone](sub/missing.md).

```
[fenced](nope.md)
```
FIX
out="$(md_links "$_t/repo/spec/SKILL.md")"
assert_eq "skill-revision/1" "md_links lists relative targets outside fences, fragments stripped" "sub/a.md
sub/missing.md" "$out"
out="$(resolve_links "$_t/repo/spec/SKILL.md" "$_t/repo")"; rc=$?
assert_rc "skill-revision/1" "resolve_links fails on a missing target" 1 "$rc"
assert_contains "skill-revision/1" "resolve_links names the missing target" "$out" "sub/missing.md"
printf 'See [a](sub/a.md).\n' > "$_t/repo/spec/OK.md"
out="$(resolve_links "$_t/repo/spec/OK.md" "$_t/repo")"; rc=$?
assert_rc "skill-revision/1" "resolve_links passes when source and layout resolve" 0 "$rc"

# --- population guard ---------------------------------------------------------
out="$(assert_population "skill-revision/1" "fixture population" 0 1 2>&1)"
assert_contains "skill-revision/1" "assert_population fails under its floor" "$out" "FAIL"
