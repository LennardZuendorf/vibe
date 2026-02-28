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

# Check all .md files exist and have frontmatter
for f in "$SPEC_DIR"/*.md; do
  name=$(basename "$f")
  echo "--- $name ---"

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

  # Entrypoints must have children
  if grep -q "^type: entrypoint" "$f"; then
    if ! grep -q "^children:" "$f"; then
      red "$name: entrypoint missing 'children:' list"
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

  # Check naming convention: {area}-{topic}.md or {area}.md
  if [[ "$name" != "product.md" && "$name" != "tech.md" ]]; then
    if [[ "$name" != product-* && "$name" != tech-* ]]; then
      red "$name: doesn't follow naming convention (must start with 'product-' or 'tech-')"
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
for entrypoint in "$SPEC_DIR"/product.md "$SPEC_DIR"/tech.md; do
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
