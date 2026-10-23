---
type: concept
updated: 2026-10-19
---

# TUI Conventions

`vibe-flow tui` is the one interactive surface in the toolkit. It follows the same conventions everywhere so a returning user never re-learns it:

- **One term per concept.** "State" always means the cursor's current phase; "transition" always means a legal move to a next state; a "signal" is always advisory, never a verdict.
- **Keys.** `r` starts recording a workflow, `n` adds a note to the recording, `s` saves it, `a` aborts the current action — mnemonic letters, each with exactly one meaning.
- **Gated transitions prompt `y/N`** before crossing a human gate — the same confirmation the CLI's `--confirm` flag grants, never a silent auto-yes.
- **The live pane** shows the cursor, legal next states with gate marks, the current orders, the last 20 ledger events, and one doctor line — polled every 500 ms off the ledger's mtime, no push channel.
