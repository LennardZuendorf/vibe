---
type: concept
updated: 2026-11-12
---

# Feature Sequence

Whole-feature delivery order with **binary** gates — a feature starts only when its upstream is `DONE`. Phases: 0 foundations · 1 flow · 2 instruct · 3 spec · 4 distribution + retirement.

Parallel tracks after F4: flow (F5→F10), instruct (F11→F15, F12 waits for F8), spec (F16→F20). F1 is a same-week hotfix on the current bash tree, independent of the Rust arc.

**Superseded rows.** Rows 15–20 of the pre-v0.4 sequence never started; F1–F25 absorbs them: machine-teeth → F7, plugin-runtime → F21/F22, spec-js → F16/F17, delegation-redirect → F9, spec-delta → F18, doc-truth → F24.

**Delivered ahead of the arc.** `host-support` (2026-10-02, on the v0.3 bash tree): the four-host project skill/command packages — Claude Code, OpenCode, Codex (skill invocation only), Factory Droid — plus the installer's `--platform claude,opencode,codex,droid,none` selection, CLI detection default, per-target ownership manifest, and surgical uninstall. That is a slice of F21/F22 scope with different carriers; both rows keep their Rust-era acceptance and stay open, and their issue docs carry the landed-ahead notes.

**Active focus.** The [spec sequence](../parts/spec/concepts/spec-feature-sequence.md) records the completed Rust spec baseline. Track 0 and F5/F6 are `DONE` ([#57](https://github.com/LennardZuendorf/vibe/pull/57)). F7 is `DELIVERED` on the linear stack [#80](https://github.com/LennardZuendorf/vibe/pull/80) → [#81](https://github.com/LennardZuendorf/vibe/pull/81) → [#82](https://github.com/LennardZuendorf/vibe/pull/82), `CLEAN` and green with merge pending; F8 is carded as [#28](https://github.com/LennardZuendorf/vibe/issues/28) on that stack top, and F9/F10 stay held. **The active implementation plan is the Flow rows only.** On 2026-11-11 the owner deferred the instruct work and its integration contracts — [#83](https://github.com/LennardZuendorf/vibe/issues/83) (Rust Instruct API + trusted provider interface), [#84](https://github.com/LennardZuendorf/vibe/issues/84) (port composition, session-safe delivery), [#85](https://github.com/LennardZuendorf/vibe/issues/85) (provider coordination across Claude Code, OpenCode and Codex), and [#86](https://github.com/LennardZuendorf/vibe/issues/86) (bounded provider + promotion contracts for Flow integration). Those four issues say in their own words that they do not authorize implementation: the instruct rows F11–F15 are **deferred, not cancelled**, and no instruct work starts before a later owner decision. Packaging, release and retirement (F21–F24) stay pending behind them.
