# v0.4 roadmap

The v0.4 re-architecture ("three tools, one language, no bundle") as a GitHub
issue set, authored from `.spec/plan.md` rows F1–F24. `issues/` holds one
Markdown file per issue (`00-epic.md`, `01-F1-*.md` … `24-F24-*.md`), each
with YAML frontmatter (`id`, `tool`, `phase`, `depends_on`, `parent`, plus
`issue`/`hold` once posted or held — see below) and the same body template
GitHub issues get: Deliverable, Scope, Acceptance, Depends on, Decisions this
implements, Notes.

## Status

The epic and 19 of the 24 features are posted to GitHub. Five features are
**held** — their architecture is still being designed in the originating
conversation, so no issue exists for them yet.

**Epic:** [#22 — v0.4 — three tools, one language, no bundle (tracking)](https://github.com/LennardZuendorf/vibe/issues/22)

| F | Title | Issue |
|---|---|---|
| F1  | spec-path-hotfix    | [#23](https://github.com/LennardZuendorf/vibe/issues/23) |
| F2  | contracts           | held |
| F3  | oracles-parity      | [#24](https://github.com/LennardZuendorf/vibe/issues/24) |
| F4  | workspace-core      | held |
| F5  | flow-core           | [#25](https://github.com/LennardZuendorf/vibe/issues/25) |
| F6  | flow-teeth-hooks    | [#26](https://github.com/LennardZuendorf/vibe/issues/26) |
| F7  | machine-teeth       | [#27](https://github.com/LennardZuendorf/vibe/issues/27) |
| F8  | flow-orders-payload | [#28](https://github.com/LennardZuendorf/vibe/issues/28) |
| F9  | flow-signals-hooks  | held |
| F10 | flow-tui            | held |
| F11 | instruct-core       | held |
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

F2, F4, F9, F10, and F11 are held for architecture design in-thread — each
file carries `hold: architecture` in its frontmatter and a `> **Held.**`
notice under the title until an issue is opened for it.

## Why files instead of issues

These Markdown files remain the source of truth even after posting: they are
what gets regenerated from `.spec/plan.md` if the plan changes, and what a
disabled-Issues repo would fall back to. Each posted file's frontmatter now
carries `issue: <number>` linking it to the GitHub issue it became.

## Posting them

**Prerequisite:** GitHub Issues enabled — repo Settings → General → Features
→ "Issues" checked.

Then, either:

1. **Script:** `bash docs/roadmap/create-issues.sh --dry-run` to preview
   (needs no `gh`), then `bash docs/roadmap/create-issues.sh` to create
   whatever isn't posted yet. Requires `gh` on PATH and `gh auth status`
   clean. Bash 3.2 compatible, shellcheck-clean.
2. **Agent:** ask Claude to post them via the GitHub MCP `issue_write` tool,
   same content, same order.

**Idempotent:** the script (and an agent following this doc) skips any file
whose frontmatter already has `issue:` set (already posted) or `hold:` set
(held for architecture design), printing one line per skip. Running it again
today reports 19 already posted, 5 held, 0 to create — safe to re-run any
time, including once the held features gain their own issue files.

## Bringing a held feature online

When a held feature's architecture is settled: remove its `hold:` line and
the `> **Held.**` notice from the file, fill in its real `## Depends on`
section (the five held features' real dependents already reference them —
update those bodies too once the new issue number exists), and run the
script or ask the agent to post it. It will pick up only that one file; the
rest stay untouched.

## Source of truth

`.spec/plan.md` is canonical. If the plan changes, regenerate these files —
never hand-edit a posted GitHub issue without updating `.spec/plan.md` first,
and never hand-edit these files to drift from the plan.
