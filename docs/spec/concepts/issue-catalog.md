---
type: concept
updated: 2026-11-16
---

# Issue catalog (GitHub)

**Issue catalog (GitHub).** The F-rows and issues are 1:1 where an issue exists; a row with no issue is either unstarted spec-program work or held on an owner decision rather than a technical gate. The instruct issues #83–#86 are deferred integration contracts; [#83](https://github.com/LennardZuendorf/vibe/issues/83) states in its own words that it does not authorize implementation. #22 is the v0.4 tracking issue; #18 (repo/branch-scoped cursor) is absorbed by this arc — the cursor path lands in F5, the plugin-carried runtime in F21; #12 (plugin never registered, closed) is fixed by design in F21/F22. Each phase of the Feature Sequence is a GitHub Milestone (`v0.4 Phase 0–4`, see [docs/roadmap/README.md](../../roadmap/README.md#milestones)); #22 itself carries no milestone since it spans all five.

| Row | Issue | Row | Issue |
|---|---|---|---|
| F1 | #23 | F13 | #30 |
| F3 | #24 | F14 | #31 |
| F5 | #25 | F15 | #32 |
| F6 | #26 | F16 | #33 |
| F7 | #27 (CLOSED 2026-11-14) | F17 | #34 |
| F8 | #28 (CLOSED 2026-11-13) | F18 | #35 |
| F12 | #29 | F19 | #36 |
| F2, F4 | to file (spec program) | F20 | #37 |
| F9, F10 | no issue — held, owner timing | F25 | to file (spec program) |
| instruct arc (F11–F15) | deferred — #83–#86 (2026-11-11) | S1 space-setup | #52 (OPEN) |

#27 and #28 are closed: F7 merged as the stack [#80](https://github.com/LennardZuendorf/vibe/pull/80) → [#81](https://github.com/LennardZuendorf/vibe/pull/81) → [#82](https://github.com/LennardZuendorf/vibe/pull/82), and F8 as [#93](https://github.com/LennardZuendorf/vibe/pull/93) on 2026-11-13. F9/F10 name no dependency: the merge their rows named has landed, so they wait on an owner timing decision. The instruct rows carry issues but are deferred, not cancelled — each of #83–#86 opens by recording that it is deferred, and #83 adds that it does not authorize implementation. #52 is the durable record of the closed `space-setup` PR #49: its proposed `S1` track is not yet accepted and the branch is unmerged.
