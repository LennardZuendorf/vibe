---
type: concept
updated: 2026-11-21
---

# Feature Sequence

Whole-feature delivery order with **binary** gates — a feature starts only when its upstream is `DONE`. Phases: 0 foundations · 1 flow · 2 instruct · 3 spec · 4 distribution + retirement.

Parallel tracks after F4: flow (F5→F10), instruct (F11→F15, F12 waits for F8), spec (F16→F20). F1 is a same-week hotfix on the current bash tree, independent of the Rust arc.

**Superseded rows.** Rows 15–20 of the pre-v0.4 sequence never started; F1–F25 absorbs them: machine-teeth → F7, plugin-runtime → F21/F22, spec-js → F16/F17, delegation-redirect → F9, spec-delta → F18, doc-truth → F24.

**Delivered ahead of the arc.** `host-support` (2026-10-02, on the v0.3 bash tree): the four-host project skill/command packages — Claude Code, OpenCode, Codex (skill invocation only), Factory Droid — plus the installer's `--platform claude,opencode,codex,droid,none` selection, CLI detection default, per-target ownership manifest, and surgical uninstall. That is a slice of F21/F22 scope with different carriers; both rows keep their Rust-era acceptance and stay open, and their issue docs carry the landed-ahead notes.

**Current priority.** Use the [product milestones](product-milestones.md) for the approved delivery order and acceptance. The F rows preserve the original Rust implementation order and evidence. Rust Instruct has returned to active planning. F9 signals remain advisory and F10 follows the working agent loop and F9. Full legacy retirement keeps its migration and parity gates.

GitHub [#22](https://github.com/LennardZuendorf/vibe/issues/22) tracks the Rust program and [#42](https://github.com/LennardZuendorf/vibe/issues/42) tracks the front-door scope.
