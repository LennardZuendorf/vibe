---
type: concept
updated: 2026-10-19
---

# v0.5 — the vibe CLI arc (queued)

The follow-on arc: one `vibe` CLI front door over the three tools, plus the cross-tool capabilities no single tool can own — worktree lifecycle, global spec management, global blocks. Architecture: [tech-cli.md](../../../tech/cli.md). Decisions: D40–D43 in [decisions](../../../decisions/README.md). Nothing starts before F22 is DONE; C4 additionally waits on F16, C5 on F14.

**Issue catalog.** #42 tracks this arc; C1–C6 are #43–#48, filed as its sub-issues (2026-09-26), all six on the `v0.5 — CLI front door` milestone; #42 itself carries none.

| Order | Feature | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|
| C1 | cli-core | `crates/vibe`: `spec\|flow\|instruct` dispatch via the shim's discovery tiers; `~/.config/vibe/repos.json` registry; `space list`; `doctor` peer + registry checks | Dispatch matrix with each tier's env deleted (both legs); missing-peer advice lines; registry floor test | NOT STARTED | F22 |
| C2 | worktree-lifecycle | `worktree create/list/remove`: git worktree + isolated `.vibe/run/`; mid-flow remove refusal; deregistration | Refusal discriminates vs naive removal; cursor isolation per worktree; fresh non-git target exits 1 with one line | NOT STARTED | C1 |
| C3 | worktree-bootstrap | Bootstrap on create: `.vibe/spec.json` root pointer, each tool's `init`, AGENTS.md blocks; staleness in `list` | Byte-parity of bootstrapped files vs `install.sh --local` on the same tree; bootstrap idempotent on re-run | NOT STARTED | C2 |
| C4 | global-lessons | Global library at `~/.config/vibe/spec/lessons.md`; `lessons-for` global source (repo wins on tag conflict); `space lessons edit/promote` | Merged-answer goldens (repo-only, global-only, conflict); promote drops repo-pattern prose; absence is clean, not warned | NOT STARTED | F16, C1 |
| C5 | space-blocks | `space blocks` list/edit global tier + pin a personal git source; sync via `vibe-instruct sync` | Pinned-source refuse-when-unpinned; offline sync path; no fetch code in `crates/vibe` (structural assertion) | NOT STARTED | F14, C1 |
| C6 | worktree-archive | `archive`: uncommitted `features/<name>/` residue moved to the main checkout, then remove; registry updates | Residue golden (before ≥1, after ==0, files ≥ floor); committed-edits-travel-by-merge control | NOT STARTED | C2, C4 |
