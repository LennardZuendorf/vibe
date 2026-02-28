#!/bin/bash
# Validate .spec/ documents for consistency
# Checks: frontmatter, cross-references, naming conventions, orphaned files

SPEC_DIR=".spec"
ERRORS=0
WARNINGS=0

red() { echo -e "\033[31m  ERROR: $1\033[0m"; ((ERRORS++)); }
yellow() { echo -e "\033[33m  WARN:  $1\033[0m"; ((WARNINGS++)); }
green() { echo -e "\033[32m  OK:    $1\033[0m"; }

echo "Validating $SPEC_DIR/..."
echo ""

if [[ ! -d "$SPEC_DIR" ]]; then
  red "No .spec/ directory found. Run setup first: bash ~/.agents/skills/spec/scripts/setup.sh"
  exit 1
fi

shopt -s nullglob
specs=("$SPEC_DIR"/*.md)

if [[ ${#specs[@]} -eq 0 ]]; then
  red "No spec files found in $SPEC_DIR/"
  exit 1
fi

# Check all .md files exist and have frontmatter
for f in "${specs[@]}"; do
  name=$(basename "$f")
  echo "--- $name ---"

  # lessons.md doesn't require frontmatter
  if [[ "$name" == "lessons.md" ]]; then
    green "$name: checked (lessons file, no frontmatter required)"
    echo ""
    continue
  fi

  # Check frontmatter exists
  if ! head -1 "$f" | grep -q "^---$"; then
    red "$name: missing YAML frontmatter"
    continue
  fi

  # Check required frontmatter fields
  if ! grep -q "^type:" "$f"; then
    red "$name: missing 'type:' in frontmatter"
  fi
  if ! grep -q "^updated:" "$f"; then
    red "$name: missing 'updated:' in frontmatter"
  fi

  # Entrypoints: product.md and tech.md must have children, plan.md warns if missing
  if grep -q "^type: entrypoint" "$f"; then
    if ! grep -q "^children:" "$f"; then
      if [[ "$name" == "plan.md" ]]; then
        yellow "$name: entrypoint has no 'children:' list (add sub-plans if needed)"
      else
        red "$name: entrypoint missing 'children:' list"
      fi
    fi
  fi

  # Branch docs must have parent
  if grep -q "^type: branch" "$f"; then
    if ! grep -q "^parent:" "$f"; then
      red "$name: branch doc missing 'parent:' field"
    fi
    if ! grep -q "^scope:" "$f"; then
      yellow "$name: branch doc missing 'scope:' field"
    fi
    if ! grep -q "^covers:" "$f"; then
      yellow "$name: branch doc missing 'covers:' field"
    fi
  fi

  # Check naming convention: {area}-{topic}.md or entrypoints
  if [[ "$name" != "product.md" && "$name" != "tech.md" && "$name" != "plan.md" && "$name" != "lessons.md" ]]; then
    # Branch docs must start with product-, tech-, or plan-
    if [[ "$name" != product-* && "$name" != tech-* && "$name" != plan-* ]]; then
      red "$name: must start with 'product-', 'tech-', or 'plan-' (e.g., product-design.md, tech-api.md, plan-editor.md)"
    fi

    # Validate topic part is lowercase with hyphens only
    if [[ "$name" =~ ^(product|tech|plan)-(.+)\.md$ ]]; then
      topic="${BASH_REMATCH[2]}"
      if [[ ! "$topic" =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?$ ]]; then
        red "$name: topic must be lowercase with hyphens only, no leading/trailing hyphens (found: '$topic')"
      fi
    fi
  fi

  # Check for broken internal links
  while IFS= read -r link; do
    target=$(echo "$link" | sed 's/.*(\(.*\))/\1/' | sed 's/#.*//')
    if [[ -n "$target" && "$target" != http* && "$target" != ../  ]]; then
      if [[ ! -f "$SPEC_DIR/$target" ]]; then
        red "$name: broken link to '$target'"
      fi
    fi
  done < <(grep -oE '\[.*?\]\([^)]+\.md[^)]*\)' "$f")

  green "$name: checked"
  echo ""
done

# Check that entrypoint children actually exist
# Check that all entrypoint children actually exist
for entrypoint in "$SPEC_DIR"/product.md "$SPEC_DIR"/tech.md "$SPEC_DIR"/plan.md; do
  if [[ -f "$entrypoint" ]]; then
    name=$(basename "$entrypoint")
    in_children=false
    while IFS= read -r line; do
      if [[ "$line" == "children:" ]]; then
        in_children=true
        continue
      fi
      if $in_children; then
        if [[ "$line" =~ ^[[:space:]]*-[[:space:]]+(.*) ]]; then
          child="${BASH_REMATCH[1]}"
          if [[ ! -f "$SPEC_DIR/$child" ]]; then
            red "$name: child '$child' listed in frontmatter but file doesn't exist"
          fi
        else
          in_children=false
        fi
      fi
    done < "$entrypoint"
  fi
done

# Check CLAUDE.md references
if [[ -f "CLAUDE.md" ]]; then
  echo "--- CLAUDE.md ---"
  while IFS= read -r link; do
    target=$(echo "$link" | sed 's/.*(\(.*\))/\1/' | sed 's/#.*//')
    if [[ "$target" == .spec/* && ! -f "$target" ]]; then
      red "CLAUDE.md: broken link to '$target'"
    fi
  done < <(grep -oE '\[.*?\]\([^)]+\.md[^)]*\)' "CLAUDE.md")
  green "CLAUDE.md: checked"
  echo ""
fi

echo "========================"
echo "Errors:   $ERRORS"
echo "Warnings: $WARNINGS"

if [[ $ERRORS -gt 0 ]]; then
  exit 1
fi
exit 0
