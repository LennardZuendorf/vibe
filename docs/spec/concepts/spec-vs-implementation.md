---
type: concept
updated: 2026-11-16
---

# Spec vs Implementation

F1–F4, F16–F20, and F25 form the completed Rust spec baseline. Track 0, F5 and F6 are implemented in source. F7 and F8 are DONE: F7 merged on 2026-11-13 as the linear stack [#80](https://github.com/LennardZuendorf/vibe/pull/80) (`fd70635`) → [#81](https://github.com/LennardZuendorf/vibe/pull/81) (`0ad85ad`) → [#82](https://github.com/LennardZuendorf/vibe/pull/82) (`7d25b9e`), and F8 shipped as [#93](https://github.com/LennardZuendorf/vibe/pull/93) (`7c868f1`), which closed [#28](https://github.com/LennardZuendorf/vibe/issues/28). F9/F10 have no dependency left — the merge their rows named has landed — so they are held on an owner timing decision, not a gate. The instruct arc ([#83](https://github.com/LennardZuendorf/vibe/issues/83)–[#86](https://github.com/LennardZuendorf/vibe/issues/86), filed 2026-11-11) is deferred by an owner decision, and [#83](https://github.com/LennardZuendorf/vibe/issues/83) states that the arc does not authorize implementation; the packaging, release and retirement rows remain pending behind it. F1 delivered the spec-path fix: the scripts `validate.sh`, `list-specs.sh`, `lessons-for.sh`, and `scan-merges.sh` walk up from the cwd to the directory holding `.git`, and a relative `SPEC_DIR` now resolves against the root found. `promote.sh`, `setup.sh`, and `check-drift.sh` are unchanged.

C0 shared configuration is implemented under `vibe config`, with stable
`vibe hook session-start` and embedded `vibe spec`. The configuration namespace
change is DONE; old configuration commands have no aliases.
See [the binary contract](../tech/cli/cli-binary.md).

The remaining v0.5 arc (C1–C6) is likewise spec-ahead — planned, not started. It gates on F22 (installer/release surface), with C4 after F16 and C5 after F14.
