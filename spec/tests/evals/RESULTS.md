# spec — selection eval results

Queries: [selection.json](selection.json) (10 should-select, 10 near-miss; the
near-misses include the six near-miss rows of the pressure harness). Listing:
`tests/evals/listing.md` (dated 2026-10-24), with only the `spec` line replaced
by the candidate description. One fresh headless session per query
(`claude -p`, no tools, no slash commands, no user settings, the listing as the
system prompt) answers with one skill name or `none`.

Pass (sonnet): ≥9/10 should-select pick `spec` and ≤1/10 near-misses pick `spec`.
Haiku floor: 8/10 and 2/10.

## skill-revision/3 — description without the literal tag, ≤500 chars

Old description: the fd6ce39 text (507 chars, an HTML comment inside).
New description: unit 3 (498 chars, no angle brackets; "how should X be built"
replaces "how should we build X" so the first-person lint passes).

| Run | Model | Should-select → `spec` | Near-miss → `spec` | Verdict |
|---|---|---:|---:|---|
| RED, old description | claude-sonnet-5-5 | 10/10 | 0/10 | pass (no RED failure: regression control) |
| GREEN, new description | claude-sonnet-5-5 | 10/10 | 0/10 | pass |
| GREEN, new description | haiku | 10/10 | 0/10 | pass |

Near-miss answers that were not `none`:

- sonnet, old and new: `nm-review` → `code-review:code-review`.
- haiku, new: `nm-review` → `code-review:code-review`; `nm-gitstatus`, `nm-lastcommit`, `nm-regex`, `nm-retry` → `using-spec`; `nm-rebase` → `superpowers:using-superpowers`. None picked `spec`.

The old description already selected well; this unit's change is driven by the
validator and the lint (length, angle brackets), and the evals show it costs no
selection accuracy.
