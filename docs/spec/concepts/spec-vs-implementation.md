---
type: concept
updated: 2026-10-25
---

# Spec vs Implementation

F1–F4, F16–F20, and F25 are delivered. F5 is delivered and F6 is in flight on `issue-25-flow-v2`; F7–F8 are not started; F9–F10 are held; F11 and F12–F24 are queued. F1 delivered the spec-path fix: the scripts `validate.sh`, `list-specs.sh`, `lessons-for.sh`, and `scan-merges.sh` walk up from the cwd to the directory holding `.git`, and a relative `SPEC_DIR` now resolves against the root found. `promote.sh`, `setup.sh`, and `check-drift.sh` are unchanged.

The v0.5 arc (C1–C6) is likewise spec-ahead — planned, not started. It gates on F22 (installer/release surface), with C4 after F16 and C5 after F14.
