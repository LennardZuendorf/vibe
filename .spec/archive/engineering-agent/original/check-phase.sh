#!/bin/bash
# PreToolUse hook: Enforce phase gates for the develop skill
#
# This hook is called before Edit, Write, and NotebookEdit tool uses.
# It delegates to the phase-gate.sh script for actual enforcement.
#
# stdin receives JSON with tool_name and tool_input from Claude Code.
# Exit 0 = allow, Exit 2 = block (stderr shown to Claude as error).

set -euo pipefail

INPUT=$(cat)

# Find the phase-gate script (check common locations)
SCRIPT=""
for candidate in \
    "$PWD/.agents/skills/develop/scripts/phase-gate.sh" \
    "$HOME/.agents/skills/develop/scripts/phase-gate.sh" \
    "$(git rev-parse --show-toplevel 2>/dev/null)/.agents/skills/develop/scripts/phase-gate.sh"
do
    if [ -f "$candidate" ]; then
        SCRIPT="$candidate"
        break
    fi
done

# If no phase-gate script found, allow (framework not installed)
if [ -z "$SCRIPT" ]; then
    exit 0
fi

# Delegate to phase-gate
echo "$INPUT" | bash "$SCRIPT"
