---
type: concept
updated: 2026-10-19
---

# Feature Sequence

Whole-feature delivery order with **binary** gates — a feature starts only when its upstream is `DONE`. Phases: 0 foundations · 1 flow · 2 instruct · 3 spec · 4 distribution + retirement.

Parallel tracks after F4: flow (F5→F10), instruct (F11→F15, F12 waits for F8), spec (F16→F20). F1 is a same-week hotfix on the current bash tree, independent of the Rust arc.

**Superseded rows.** Rows 15–20 of the pre-v0.4 sequence never started; F1–F25 absorbs them: machine-teeth → F7, plugin-runtime → F21/F22, spec-js → F16/F17, delegation-redirect → F9, spec-delta → F18, doc-truth → F24.

**Active focus.** The spec program (design: `docs/superpowers/specs/2026-10-13-spec-v04-design.md`) runs on one branch: amendment → F1 → F25 → F2 ∥ F3 → F4 → F16 → F17 → F18 → F19 → F20 → spec slices of F21–F24. F2 and F4 are un-held and scope-cut to the spec tool. F9, F10, F11 stay held; the flow (F5–F8) and instruct (F12–F15) tracks are unchanged and not part of this program.
