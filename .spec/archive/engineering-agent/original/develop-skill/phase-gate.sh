#!/bin/bash
# Phase Gate Enforcement Script
# Called by PreToolUse hooks to enforce phase boundaries
#
# Reads the current phase from .spec/.phase and the tool being used from stdin.
# Exits 0 to allow, exits 2 to block with message.
#
# Usage: echo '{"tool_name":"Edit","tool_input":{"file_path":"/path/to/file"}}' | ./phase-gate.sh

set -euo pipefail

# Read tool call info from stdin
INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null || echo "")
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || echo "")

# Find the .spec directory (walk up from cwd)
find_spec_dir() {
    local dir="$PWD"
    while [ "$dir" != "/" ]; do
        if [ -d "$dir/.spec" ]; then
            echo "$dir/.spec"
            return 0
        fi
        dir=$(dirname "$dir")
    done
    return 1
}

SPEC_DIR=$(find_spec_dir 2>/dev/null || echo "")

# If no .spec directory or no .phase file, allow everything (framework not initialized)
if [ -z "$SPEC_DIR" ] || [ ! -f "$SPEC_DIR/.phase" ]; then
    exit 0
fi

PHASE=$(cat "$SPEC_DIR/.phase" 2>/dev/null | tr -d '[:space:]')

# If phase is empty or DONE, allow everything
if [ -z "$PHASE" ] || [ "$PHASE" = "DONE" ] || [ "$PHASE" = "IMPLEMENT" ]; then
    exit 0
fi

# Helper: check if file is inside .spec/
is_spec_file() {
    local fp="$1"
    case "$fp" in
        */.spec/*|*.spec/*) return 0 ;;
        *) return 1 ;;
    esac
}

# Helper: check if file is the phase file itself
is_phase_file() {
    local fp="$1"
    case "$fp" in
        */.spec/.phase) return 0 ;;
        *) return 1 ;;
    esac
}

# Helper: check if file is the research file
is_research_file() {
    local fp="$1"
    case "$fp" in
        */.spec/research.md|*.spec/research.md) return 0 ;;
        *) return 1 ;;
    esac
}

# Phase: RESEARCH — block all writes except .spec/.phase and .spec/research.md
if [ "$PHASE" = "RESEARCH" ]; then
    case "$TOOL_NAME" in
        Edit|Write|NotebookEdit)
            if is_phase_file "$FILE_PATH" || is_research_file "$FILE_PATH"; then
                exit 0
            fi
            echo "BLOCKED: You are in the RESEARCH phase. Only .spec/research.md and .spec/.phase can be written. Complete your research and get user confirmation before moving to SPEC phase." >&2
            exit 2
            ;;
    esac
fi

# Phase: SPEC — only allow writes to .spec/ files
if [ "$PHASE" = "SPEC" ]; then
    case "$TOOL_NAME" in
        Edit|Write|NotebookEdit)
            if is_spec_file "$FILE_PATH" || is_phase_file "$FILE_PATH"; then
                exit 0
            fi
            echo "BLOCKED: You are in the SPEC phase. Only .spec/ files can be written. Write your specs first, get user approval, then move to PLAN phase." >&2
            exit 2
            ;;
    esac
fi

# Phase: PLAN — only allow writes to .spec/plan* files and .spec/.phase
if [ "$PHASE" = "PLAN" ]; then
    case "$TOOL_NAME" in
        Edit|Write|NotebookEdit)
            if is_phase_file "$FILE_PATH"; then
                exit 0
            fi
            if is_spec_file "$FILE_PATH"; then
                case "$FILE_PATH" in
                    */plan*) exit 0 ;;
                esac
            fi
            echo "BLOCKED: You are in the PLAN phase. Only .spec/plan*.md files can be written. Create your plan, get user approval, then move to IMPLEMENT phase." >&2
            exit 2
            ;;
    esac
fi

# Phase: REVIEW — allow edits (fixes) but block creating new files
if [ "$PHASE" = "REVIEW" ]; then
    case "$TOOL_NAME" in
        Write)
            if is_spec_file "$FILE_PATH" || is_phase_file "$FILE_PATH"; then
                exit 0
            fi
            echo "BLOCKED: You are in the REVIEW phase. No new files allowed — only fixes to existing files (use Edit, not Write). If you need a new file, go back and update the plan." >&2
            exit 2
            ;;
    esac
fi

# Default: allow
exit 0
