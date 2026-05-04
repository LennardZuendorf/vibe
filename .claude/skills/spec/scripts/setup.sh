#!/bin/bash
# Initialize a .spec/ directory in the current project
# Copies templates as starting points for product, tech, and plan entrypoints

set -e

SPEC_DIR=".spec"
TEMPLATE_DIR="$HOME/.agents/skills/spec/reference/templates"

green() { echo -e "\033[32m  $1\033[0m"; }
yellow() { echo -e "\033[33m  $1\033[0m"; }
red() { echo -e "\033[31m  $1\033[0m"; }

echo ""
echo "Setting up $SPEC_DIR/ ..."
echo ""

# Check if .spec/ already exists
if [[ -d "$SPEC_DIR" ]]; then
  existing=$(ls "$SPEC_DIR"/*.md 2>/dev/null | wc -l | tr -d ' ')
  if [[ "$existing" -gt 0 ]]; then
    yellow "WARN: $SPEC_DIR/ already exists with $existing file(s)."
    echo ""
    echo "  Existing files:"
    for f in "$SPEC_DIR"/*.md; do
      echo "    - $(basename "$f")"
    done
    echo ""
    yellow "Only missing entrypoints will be created. Existing files will NOT be overwritten."
    echo ""
  fi
else
  mkdir -p "$SPEC_DIR"
  green "Created $SPEC_DIR/"
fi

# Check template directory exists
if [[ ! -d "$TEMPLATE_DIR" ]]; then
  red "ERROR: Template directory not found at $TEMPLATE_DIR"
  red "Make sure the spec skill is installed at ~/.agents/skills/spec/"
  exit 1
fi

# Copy entrypoint templates (only if they don't exist)
copied=0

for entrypoint in product.md tech.md plan.md; do
  if [[ -f "$SPEC_DIR/$entrypoint" ]]; then
    yellow "SKIP: $entrypoint already exists"
  else
    if [[ -f "$TEMPLATE_DIR/$entrypoint" ]]; then
      cp "$TEMPLATE_DIR/$entrypoint" "$SPEC_DIR/$entrypoint"
      green "Created $SPEC_DIR/$entrypoint (from template)"
      ((copied++))
    else
      red "ERROR: Template not found: $TEMPLATE_DIR/$entrypoint"
    fi
  fi
done

# Create lessons.md if it doesn't exist (no template needed — starts empty)
if [[ -f "$SPEC_DIR/lessons.md" ]]; then
  yellow "SKIP: lessons.md already exists"
else
  cat > "$SPEC_DIR/lessons.md" << 'LESSONS_EOF'
# Lessons

Mistakes made and rules to prevent repeating them. Review at the start of every session.

<!-- Format for each lesson:
### [Short description]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this
**Date:** YYYY-MM-DD
-->
LESSONS_EOF
  green "Created $SPEC_DIR/lessons.md"
  ((copied++))
fi

echo ""

# Add .spec/ to .gitignore reminder if git repo
if [[ -d ".git" ]]; then
  if [[ -f ".gitignore" ]]; then
    if ! grep -q "^\.spec/" ".gitignore" 2>/dev/null; then
      yellow "NOTE: .spec/ is not in .gitignore."
      yellow "If you want to track specs in git (recommended), no action needed."
      yellow "If you want to exclude specs, add '.spec/' to .gitignore."
    fi
  fi
fi

echo "========================"
echo ""
if [[ $copied -gt 0 ]]; then
  green "Setup complete. $copied entrypoint(s) created."
else
  yellow "No new files created (all entrypoints already exist)."
fi
echo ""
echo "  Spec writing order:"
echo "    1. product.md           — Define WHAT you're building and WHY"
echo "    2. tech.md              — Define HOW you'll build it"
echo "    3. product-{topic}.md   — Product branch docs (write product first...)"
echo "       tech-{topic}.md     — ...then matching tech branch doc"
echo "    4. plan.md              — Overall implementation roadmap"
echo "    5. plan-{topic}.md      — Feature sub-plans (optional, 3+ milestones)"
echo "    *  lessons.md           — Updated after corrections (created automatically)"
echo ""
echo "  Entrypoint templates (product.md, tech.md, plan.md) have been copied above."
echo "  Branch doc templates for step 3-5 are at:"
echo "    $TEMPLATE_DIR/product-xxx.md"
echo "    $TEMPLATE_DIR/tech-xxx.md"
echo "    $TEMPLATE_DIR/plan-xxx.md"
echo ""
echo "  Validate anytime: bash ~/.agents/skills/spec/scripts/validate.sh"
echo ""
