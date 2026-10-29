# shellcheck shell=bash
# tests/skills/lib.sh — lint helpers for the shipped skills (skill-revision/1).
# Sourced by run.sh and by every tests/skills/cases/*.sh. Bash 3.2 safe: no
# associative arrays, no mapfile, no jq. Each lint prints one line per
# violation and returns 1 when it printed any.

PASS="${PASS:-0}"
FAIL="${FAIL:-0}"

pass() { echo "  PASS [$1] $2"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL [$1] $2"; FAIL=$((FAIL + 1)); }

# assert_eq <id> <desc> <expected> <actual>
assert_eq() {
  if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else
    fail "$1" "$2"; printf '        expected: %s\n        actual:   %s\n' "$3" "$4"; fi
}
# assert_rc <id> <desc> <expected rc> <actual rc>
assert_rc() {
  if [[ "$3" == "$4" ]]; then pass "$1" "$2"; else
    fail "$1" "$2"; echo "        expected rc $3, got rc $4"; fi
}
# assert_contains <id> <desc> <haystack> <needle>
assert_contains() {
  if [[ "$3" == *"$4"* ]]; then pass "$1" "$2"; else
    fail "$1" "$2"; echo "        expected to contain: $4"; fi
}
# assert_population <id> <desc> <count> <floor> — a guard that examined too little fails.
assert_population() {
  if [[ "$3" -ge "$4" ]]; then pass "$1" "$2 ($3 >= $4)"; else
    fail "$1" "$2 (examined $3, floor $4)"; fi
}

# desc_of FILE — the frontmatter description, folded onto one line.
desc_of() {
  awk '
    NR == 1 && $0 != "---" { exit }
    NR > 1 && $0 == "---" { exit }
    /^description:/ {
      d = 1; sub(/^description:[ \t]*/, "")
      if ($0 == "|" || $0 == ">" || $0 == "|-" || $0 == ">-") next
      out = $0; next
    }
    d && /^[A-Za-z][A-Za-z0-9_-]*:/ { d = 0 }
    d { line = $0; sub(/^[ \t]+/, "", line); sub(/[ \t]+$/, "", line)
        if (line != "") out = (out == "" ? line : out " " line) }
    END { sub(/[ \t]+$/, "", out); print out }
  ' "$1"
}

# lint_description FILE [assertive] — one line per violation.
lint_description() {
  local d n bad=0
  d="$(desc_of "$1")"
  n="${#d}"
  if [[ "$n" -gt 500 ]]; then echo "$1: description is $n chars (max 500)"; bad=1; fi
  if [[ "$d" == *"<"* || "$d" == *">"* ]]; then echo "$1: description has an angle bracket"; bad=1; fi
  if printf '%s' "$d" | grep -Eiq "(^|[^A-Za-z])(I|I'm|I'll|I've|my|we|our)([^A-Za-z]|$)"; then
    echo "$1: description is in the first person"; bad=1
  fi
  if [[ "${2:-}" != "assertive" && "$d" != *"Use when"* ]]; then
    echo "$1: description lacks \"Use when\""; bad=1
  fi
  return "$bad"
}

# lint_frontmatter FILE ALLOWED_KEYS EXPECTED_NAME — delimiters, key allowlist, name.
lint_frontmatter() {
  local f="$1" allowed=" $2 " want="$3" bad=0 key name close
  if [[ "$(head -n 1 "$f")" != "---" ]]; then echo "$f: frontmatter does not open with ---"; return 1; fi
  close="$(awk 'NR > 1 && $0 == "---" { print NR; exit }' "$f")"
  if [[ -z "$close" ]]; then echo "$f: frontmatter is not closed"; return 1; fi
  while IFS= read -r key; do
    [[ -z "$key" ]] && continue
    if [[ "$allowed" != *" $key "* ]]; then echo "$f: unexpected frontmatter key: $key"; bad=1; fi
  done <<EOK
$(sed -n "2,$((close - 1))p" "$f" | grep -Eo '^[A-Za-z][A-Za-z0-9_-]*:' | sed 's/:$//')
EOK
  name="$(sed -n "2,$((close - 1))p" "$f" | sed -n 's/^name:[ \t]*//p' | head -n 1 | sed 's/[ \t]*$//')"
  if [[ "$name" != "$want" ]]; then echo "$f: name is '$name', expected '$want'"; bad=1; fi
  return "$bad"
}

# words_outside FILE — words outside the frontmatter and the vibe:doctrine / vibe:orders blocks.
words_outside() {
  awk '
    NR == 1 && $0 == "---" { fm = 1; next }
    fm && $0 == "---" { fm = 0; next }
    fm { next }
    /<!-- vibe:(doctrine|orders)/ { skip = 1; next }
    /<!-- \/vibe:(doctrine|orders)/ { skip = 0; next }
    !skip { n += NF }
    END { print n + 0 }
  ' "$1"
}

# md_links FILE — relative link targets outside code fences; fragments and URLs dropped.
md_links() {
  awk '/^[ \t]*```/ { f = !f; next } !f { print }' "$1" \
    | grep -Eo '\]\([^)[:space:]]+\)' \
    | sed -e 's/^](//' -e 's/)$//' -e 's/#.*$//' \
    | grep -Ev '^$|^[A-Za-z][A-Za-z0-9+.-]*:' || true
}

# _layout_path FILE LAYOUT_ROOT — FILE's path inside the .agents/skills layout.
_layout_path() {
  local rel="${1#"$2"/}"
  case "$rel" in
    spec/using-spec/*) printf '%s/.agents/skills/using-spec/%s\n' "$2" "${rel#spec/using-spec/}" ;;
    spec/*) printf '%s/.agents/skills/spec/%s\n' "$2" "${rel#spec/}" ;;
    flow/*) printf '%s/.agents/skills/vibe/%s\n' "$2" "${rel#flow/}" ;;
    *) printf '%s\n' "$1" ;;
  esac
}

# resolve_links FILE LAYOUT_ROOT — every link resolves in source and in the layout.
resolve_links() {
  local f="$1" root="$2" bad=0 t sdir ldir
  sdir="$(dirname "$f")"
  ldir="$(dirname "$(_layout_path "$f" "$root")")"
  while IFS= read -r t; do
    [[ -z "$t" ]] && continue
    [[ -e "$sdir/$t" ]] || { echo "$f: unresolved in source: $t"; bad=1; }
    [[ -e "$ldir/$t" ]] || { echo "$f: unresolved in layout: $t"; bad=1; }
  done <<EOL
$(md_links "$f")
EOL
  return "$bad"
}

# fixed_root_hits FILE… — lines spelling docs/spec/ as a target, minus the defining lines.
fixed_root_hits() {
  local f
  for f in "$@"; do
    grep -nF 'docs/spec/' "$f" 2>/dev/null \
      | grep -vF -e 'without the binary' -e 'spec root (docs/spec)' \
      | sed "s|^|$f:|" || true
  done
}

# pressure_ok FILE — a pressure scenario with a RED section of >=5 reps and a GREEN of >=5 reps or a NO-RED verdict.
pressure_ok() {
  local f="$1" red green
  head -n 1 "$f" | grep -q '^# skill-revision/' || return 1
  red="$(awk '/^## /{s=$0} /^- rep [0-9]+:/ && s ~ /^## RED/ { n++ } END { print n + 0 }' "$f")"
  green="$(awk '/^## /{s=$0} /^- rep [0-9]+:/ && s ~ /^## GREEN/ { n++ } END { print n + 0 }' "$f")"
  [[ "$red" -ge 5 ]] || return 1
  [[ "$green" -ge 5 ]] && return 0
  awk '/^## /{s=$0} /NO-RED/ && s ~ /^## Verdict/ { found = 1 } END { exit !found }' "$f"
}
