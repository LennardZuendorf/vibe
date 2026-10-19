---
type: concept
updated: 2026-10-19
---

# Information Hierarchy

| Surface | Primary Question It Answers |
|---|---|
| `AGENTS.md` / `CLAUDE.md` (per-owner managed blocks) | How should this runtime behave in this repo, by whose rule? |
| `.vibe/*.json` (`spec.json`, `flow.json`, `instruct.json`) | How is each tool configured in this repo? |
| `.vibe/run/` (cursor, ledger, payloads) | What phase are we in, and what just happened? |
| `docs/spec/**` | What are we building, why, how, and what remains? |
| The three skills (`spec`, `flow`, `instruct`) | What should the agent do right now, for which tool? |
| The `flow` TUI | What are the live cursor, legal next states, and recent history, at a glance? |
| `vibe worktree` (v0.5) | Where does this feature run, and what state is its isolated workspace in? |
| `~/.config/vibe/` global tier: registry, global lessons, global blocks (v0.5) | What do I carry across repos and machines — and what is every registered project doing? |
