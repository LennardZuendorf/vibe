# v0.4 roadmap

The v0.4 re-architecture ("three tools, one language, no bundle") as a GitHub
issue set, authored from `docs/spec/plan.md` rows F1–F25. `issues/` holds one
Markdown file per issue (`00-epic.md`, `01-F1-*.md` … `25-F25-*.md`), each
with YAML frontmatter (`id`, `tool`, `phase`, `depends_on`, `parent`, plus
`issue`/`hold` once posted or held — see below) and the same body template
GitHub issues get: Deliverable, Scope, Acceptance, Depends on, Decisions this
implements, Notes.

## Status

The [product milestones](../spec/concepts/product-milestones.md) are the current delivery plan. The F-row briefs below retain the Rust port scope.

The [spec feature sequence](../spec/concepts/feature-sequence.md) owns current status and gates.
The table below maps the original feature briefs to their live issues. F2, F4 and F25 are delivered without standalone issues.
F9 remains advisory. F10 follows F9 and the working runner. Rust Instruct is active planning.

**Epic:** [#22 — v0.4 — three tools, one language, no bundle (tracking)](https://github.com/LennardZuendorf/vibe/issues/22)

| F | Title | Issue |
|---|---|---|
| F1  | spec-path-hotfix    | [#23](https://github.com/LennardZuendorf/vibe/issues/23) |
| F2  | contracts           | delivered; no standalone issue |
| F3  | oracles-parity      | [#24](https://github.com/LennardZuendorf/vibe/issues/24) |
| F4  | workspace-core      | delivered; no standalone issue |
| F5  | flow-core           | [#25](https://github.com/LennardZuendorf/vibe/issues/25) |
| F6  | flow-teeth-hooks    | [#26](https://github.com/LennardZuendorf/vibe/issues/26) |
| F7  | machine-teeth       | [#27](https://github.com/LennardZuendorf/vibe/issues/27) |
| F8  | flow-orders-payload | [#28](https://github.com/LennardZuendorf/vibe/issues/28) |
| F9  | flow-signals-hooks  | [#100](https://github.com/LennardZuendorf/vibe/issues/100) |
| F10 | flow-tui            | [#102](https://github.com/LennardZuendorf/vibe/issues/102) |
| F11 | instruct-core       | active planning; related contract [#83](https://github.com/LennardZuendorf/vibe/issues/83) |
| F12 | instruct-providers  | [#29](https://github.com/LennardZuendorf/vibe/issues/29) |
| F13 | instruct-sources    | [#30](https://github.com/LennardZuendorf/vibe/issues/30) |
| F14 | instruct-adapters   | [#31](https://github.com/LennardZuendorf/vibe/issues/31) |
| F15 | instruct-migration  | [#32](https://github.com/LennardZuendorf/vibe/issues/32) |
| F16 | spec-core           | [#33](https://github.com/LennardZuendorf/vibe/issues/33) |
| F17 | spec-validators     | [#34](https://github.com/LennardZuendorf/vibe/issues/34) |
| F18 | spec-delta          | [#35](https://github.com/LennardZuendorf/vibe/issues/35) |
| F19 | spec-hooks-agents   | [#36](https://github.com/LennardZuendorf/vibe/issues/36) |
| F20 | spec-migrate        | [#37](https://github.com/LennardZuendorf/vibe/issues/37) |
| F21 | packaging           | [#38](https://github.com/LennardZuendorf/vibe/issues/38) |
| F22 | release-install     | [#39](https://github.com/LennardZuendorf/vibe/issues/39) |
| F23 | retire-legacy       | [#40](https://github.com/LennardZuendorf/vibe/issues/40) |
| F24 | doc-truth           | [#41](https://github.com/LennardZuendorf/vibe/issues/41) |
| F25 | spec-superpowers-default | delivered; no standalone issue |

The Flow v2 continuation is [#101](https://github.com/LennardZuendorf/vibe/issues/101).
The active skill-revision closeout is [#103](https://github.com/LennardZuendorf/vibe/issues/103).

## Milestones

Each phase is a GitHub Milestone, assigned to every posted issue in that
phase (held features carry the matching `milestone:` in frontmatter, ready
to apply the moment they're posted). The tracking epics (#22, #42) carry no
milestone — each spans every phase in its arc.

| Milestone | Phase | Issues |
|---|---|---|
| [v0.4 Phase 0 — foundations](https://github.com/LennardZuendorf/vibe/milestone/1) | F1–F4 | #23, #24 (F2 and F4 delivered) |
| [v0.4 Phase 1 — flow](https://github.com/LennardZuendorf/vibe/milestone/2) | F5–F10 | #25–#28, #100–#102 |
| [v0.4 Phase 2 — instruct](https://github.com/LennardZuendorf/vibe/milestone/3) | F11–F15 | #29–#32 (earlier Instruct phase) |
| [v0.4 Phase 3 — spec](https://github.com/LennardZuendorf/vibe/milestone/4) | F16–F20, F25 | #33–#37 |
| [v0.4 Phase 4 — distribution + retirement](https://github.com/LennardZuendorf/vibe/milestone/5) | F21–F24 | #38–#41 |
| [v0.5 — CLI front door](https://github.com/LennardZuendorf/vibe/milestone/6) | C1–C6 | #43–#48 |

## Issue briefs

These files preserve the original deliverables and acceptance criteria. The
linked issues carry current status and discussion. Reconcile both with the
[canonical plan](../spec/plan.md) when a gate or scope changes.

When posting a new brief, write the returned issue number into its `issue:`
frontmatter. Skip briefs that already have an issue, are on hold, or carry
`status: delivered`. Remove a hold only after the owner approves the scope.
