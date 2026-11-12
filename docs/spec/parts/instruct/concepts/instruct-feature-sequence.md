---
type: concept
updated: 2026-11-12
---

# Instruct feature sequence

Rows of the Feature Sequence owned by the instruct tool; delivery order and binary gates are in [the feature sequence](../../../concepts/feature-sequence.md).

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F11 | instruct-core | instruct | Config + four-tier layering; block/channel port; marker writer v1; lints/budgets | Fixture tree renders expected winners; corrupted markers exit 2 | NOT STARTED (held) | F4 |
| F12 | instruct-providers | instruct | Payload reader + session ledger; manifest/command providers; recompose + drift event | Turn-one edge; unchanged seq level-only; standalone == composed bytes | NOT STARTED | F8, F11 |
| F13 | instruct-sources | instruct | dir/git/https sources, pins, cache, lockfile, `sync`, offline path | Unpinned repo-scope refused; offline sync exits 0 | NOT STARTED | F11 |
| F14 | instruct-adapters | instruct | Claude + OpenCode adapters; generated local files; `.claude/rules` emitter; `init`/`doctor` | Hook stdin fixtures → expected stdout; resume re-emits staleness only | NOT STARTED | F12, F13 |
| F15 | instruct-migration | instruct | Split today's Active-Rules block into committed vs global blocks; retire the old marker grammar | This repo's AGENTS.md passes `write --check` + `lint` | NOT STARTED | F14 |

**Deferred (owner decision, 2026-11-11).** These rows are not active work: the owner narrowed the implementation plan to the Flow rows ([Active focus](../../../concepts/feature-sequence.md)). The instruct integration beyond these rows is recorded as [#83](https://github.com/LennardZuendorf/vibe/issues/83), [#84](https://github.com/LennardZuendorf/vibe/issues/84) and [#85](https://github.com/LennardZuendorf/vibe/issues/85), with the Flow-facing spec contract as [#86](https://github.com/LennardZuendorf/vibe/issues/86). All are deferred, not cancelled — each says so itself — and their gates above still hold when work resumes.
