#!/usr/bin/env bash
# create-issues.sh — post docs/roadmap/issues/*.md as GitHub issues once Issues
# is enabled on the repo. Requires `gh` on PATH and an authenticated session.
# `--dry-run` previews the plan and needs neither.
#
# Bash-3.2 compatible on purpose (stock macOS): no `declare -A`, no mapfile.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
issues_dir="${script_dir}/issues"
dry_run=0
[[ "${1:-}" == "--dry-run" ]] && dry_run=1

# --- frontmatter helpers -------------------------------------------------------
body_of() { # strip YAML frontmatter, print body to stdout
  awk 'BEGIN{n=0} /^---$/{ if (n<2) {n++; next} } n>=2{print}' "$1"
}
field_of() { # field_of <file> <key> — scalar frontmatter value, quotes stripped
  awk -v key="$2" '
    /^---$/ { n++; if (n>=2) exit; next }
    n==1 && index($0, key ": ")==1 {
      v=substr($0, length(key)+3); gsub(/^"|"$/, "", v); print v; exit
    }' "$1"
}
deps_of() { # comma-separated F-ids from `depends_on: [F5, F6]`, or empty
  awk -F'[][]' '/^depends_on:/{print $2; exit}' "$1" | tr -d ' '
}

# Population floor: an empty issues dir must fail loudly, never post nothing.
child_count="$(find "${issues_dir}" -maxdepth 1 -name '[0-9][0-9]-F*.md' | wc -l | tr -d ' ')"
epic_file="${issues_dir}/00-epic.md"
[[ -f "${epic_file}" ]] || { echo "error: ${epic_file} missing" >&2; exit 1; }
[[ "${child_count}" -ge 1 ]] || { echo "error: no child issue files under ${issues_dir}" >&2; exit 1; }
epic_title="$(field_of "${epic_file}" title)"

if [[ "${dry_run}" -eq 1 ]]; then
  echo "[dry-run] would create epic: ${epic_title}"
  for f in "${issues_dir}"/[0-9][0-9]-F*.md; do
    echo "[dry-run] would create: $(field_of "${f}" title)  (depends on: $(deps_of "${f}"))"
  done
  echo "[dry-run] ${child_count} child issues"
  exit 0
fi

command -v gh >/dev/null 2>&1 || { echo "error: gh not found on PATH" >&2; exit 1; }
gh auth status >/dev/null 2>&1 || { echo "error: gh not authenticated (run: gh auth login)" >&2; exit 1; }

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT
mkdir -p "${tmp_dir}/num"

echo "creating epic..."
body_of "${epic_file}" > "${tmp_dir}/epic-body.md"
epic_url="$(gh issue create --title "${epic_title}" --body-file "${tmp_dir}/epic-body.md")"
epic_num="${epic_url##*/}"
echo "epic: #${epic_num} (${epic_url})"

for f in "${issues_dir}"/[0-9][0-9]-F*.md; do
  fid="$(field_of "${f}" id)"
  title="$(field_of "${f}" title)"
  child_body="${tmp_dir}/${fid}-body.md"
  body_of "${f}" > "${child_body}"
  printf '\nParent: #%s\n' "${epic_num}" >> "${child_body}"
  echo "creating ${fid}..."
  child_url="$(gh issue create --title "${title}" --body-file "${child_body}")"
  printf '%s' "${child_url##*/}" > "${tmp_dir}/num/${fid}"
  echo "  -> #${child_url##*/}"
done

num_of() { # num_of <F-id> — issue number, or the id itself when unknown
  if [[ -f "${tmp_dir}/num/$1" ]]; then cat "${tmp_dir}/num/$1"; else printf '%s' "$1"; fi
}

echo "updating epic checklist..."
checklist="${tmp_dir}/checklist.md"
: > "${checklist}"
for phase in 0 1 2 3 4; do
  printf '\n### Phase %s\n' "${phase}" >> "${checklist}"
  for f in "${issues_dir}"/[0-9][0-9]-F*.md; do
    [[ "$(field_of "${f}" phase)" == "${phase}" ]] || continue
    fid="$(field_of "${f}" id)"
    name="$(basename "${f}" .md | sed -E 's/^[0-9]+-F[0-9]+-//')"
    deps="$(deps_of "${f}")"
    starts_when="—"
    if [[ -n "${deps}" ]]; then
      starts_when=""
      old_ifs="${IFS}"; IFS=','
      for d in ${deps}; do
        starts_when="${starts_when:+${starts_when}, }${d} (#$(num_of "${d}"))"
      done
      IFS="${old_ifs}"
    fi
    echo "- [ ] #$(num_of "${fid}") ${fid} ${name} — starts when ${starts_when}" >> "${checklist}"
  done
done
printf '\nParallel tracks after F4: flow (F5→F10), instruct (F11→F15; F12 waits for F8), spec (F16→F20).\n' >> "${checklist}"

body_of "${epic_file}" | awk -v cl="${checklist}" '
  /<!-- checklist filled in by create-issues.sh -->/ {
    while ((getline line < cl) > 0) print line; close(cl); next
  }
  { print }' > "${tmp_dir}/epic-final.md"
gh issue edit "${epic_num}" --body-file "${tmp_dir}/epic-final.md" >/dev/null

echo ""
echo "F<n> -> #<num>  title"
echo "epic -> #${epic_num}  ${epic_title}"
for f in "${issues_dir}"/[0-9][0-9]-F*.md; do
  fid="$(field_of "${f}" id)"
  echo "${fid} -> #$(num_of "${fid}")  $(field_of "${f}" title)"
done
