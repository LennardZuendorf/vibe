#!/usr/bin/env bash
# vibe-spec pre-commit v1
# Installed by `vibe-spec setup --git-hooks`. Blocks a commit that breaks the spec.
if ! command -v vibe-spec >/dev/null 2>&1; then
  echo "vibe-spec: not on PATH; skipping the spec pre-commit check" >&2
  exit 0
fi
vibe-spec root >/dev/null 2>&1 || exit 0
VIBE_SPEC_STRICTNESS='{"SF29":"error"}' vibe-spec validate --changed --strict
exit $?
