---
type: tech
applies:
  - "AGENTS.md"
  - "CLAUDE.md"
  - "crates/vibe-core/**"
updated: 2026-10-19
---

# Marker grammar v1

```text
<!-- vibe:<owner>:begin v=1 hash=<sha256:12> -->
...block body...
<!-- vibe:<owner>:end -->
```

Owners and render order: `spec` (10), `flow` (20), `instruct` (30). Each
tool writes its own block standalone (`<tool> render agents-md --write`)
plus its fragment to `.vibe/run/agents-md/<NN>-<owner>.md`; instruct's
`write` recomposes all fragments in order and appends its rules block.
**Standalone bytes equal composed bytes** — uninstalling instruct changes
nothing for spec's or flow's block. Rules: bytes outside markers are never
touched; a malformed, nested, or duplicated marker pair exits 2 and writes
nothing; a managed block found in an ancestor AGENTS.md/CLAUDE.md is
refused, never overwritten; `--check` exits 1 on drift (hash mismatch), for
CI use. The grammar is implemented once, in `vibe-core`; no tool spells a
marker itself.
