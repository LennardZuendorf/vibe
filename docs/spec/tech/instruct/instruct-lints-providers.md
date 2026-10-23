---
type: tech
applies:
  - "instruct/**"
  - ".vibe/providers/**"
  - ".vibe/run/inject/**"
updated: 2026-10-19
---

# Lints

`vibe-instruct lint` fails a render on: a channel budget exceeded; a
block's justification framed negatively without a stated reason ("don't
X" without why); the same text duplicated across two channels; a
repo-shared block declaring a `paths:`-scoped rule with no matching
`.claude/rules/` target; a managed block found in an ancestor AGENTS.md or
CLAUDE.md (must refuse, never overwrite); a block declaring
`enforced-by:` for a channel where the named mechanism does not exist.
One fixture per rule; a clean fixture tree passes with zero findings.

## Provider execution

instruct reads payload files under `.vibe/run/inject/` and manifests
under `.vibe/providers/` — see [tech-contracts.md](../contracts.md) for
the schemas. From instruct's side: providers run after instruct's own
blocks, in manifest order; a provider never learns another tool's
internals, only its declared channel/trigger/budget. instruct owns the
session ledger (`.vibe/run/instruct/sessions/<id>.json`) that decides
which `seq` and which event `id`s have already been shown.
