#!/usr/bin/env bash
# List all spec files with their type, scope, and area

if [ ! -d ".spec" ]; then
  echo "No .spec/ directory found in current project"
  exit 0
fi

shopt -s nullglob
specs=(.spec/*.md)

if [ ${#specs[@]} -eq 0 ]; then
  echo "No spec files found in .spec/"
  exit 0
fi

for f in "${specs[@]}"; do
  name=$(basename "$f")
  type=$(grep -m1 "^type:" "$f" 2>/dev/null | sed 's/type: //')
  scope=$(grep -m1 "^scope:" "$f" 2>/dev/null | sed 's/scope: //')

  # Determine area from filename
  if [[ "$name" == "product.md" || "$name" == product-* ]]; then
    area="product"
  elif [[ "$name" == "tech.md" || "$name" == tech-* ]]; then
    area="tech"
  elif [[ "$name" == "plan.md" || "$name" == plan-* ]]; then
    area="plan"
  elif [[ "$name" == "lessons.md" ]]; then
    area="lessons"
  else
    area="unknown"
  fi

  echo "- $name [$area] ($type: $scope)"
done
