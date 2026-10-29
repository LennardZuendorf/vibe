---
type: tech
applies:
  - "AGENTS.md"
  - "CLAUDE.md"
  - "crates/vibe-core/**"
updated: 2026-10-24
---

# Marker grammar v1

```text
<!-- vibe:<owner>:begin v=1 hash=<fnv1a64:16> -->
...block body...
<!-- vibe:<owner>:end -->
```

`hash` is the FNV-1a 64-bit hash of the block body as written between the marker
lines, as 16 lowercase hex digits. Markers are change detectors, not a security
mechanism, so the workspace takes no hash crate. In files that use `#` comments
(TOML) the same lines read `# vibe:<owner>:begin v=1 hash=<hash>`.

Owners and render order: `spec` (10), `flow` (20), `instruct` (30). The `store`
owner belongs to the `vibe` binary (D48): it writes `vibe:store` blocks into
harness instruction files and `config.toml`, outside the render order above. Each
tool writes its own block standalone (`<tool> render agents-md --write`)
plus its fragment to `.vibe/run/agents-md/<NN>-<owner>.md`; instruct's
`write` recomposes all fragments in order and appends its rules block.
**Standalone bytes equal composed bytes** — uninstalling instruct changes
nothing for spec's or flow's block. Rules: bytes outside markers are never
touched; a malformed, nested, or duplicated marker pair exits 2 and writes
nothing (the `vibe` binary instead refuses that one target, leaves the file
untouched and exits 1 after projecting the others); a managed block found in an ancestor AGENTS.md/CLAUDE.md is
refused, never overwritten; `--check` exits 1 on drift (hash mismatch), for
CI use. The grammar is implemented once, in `vibe-core`; no tool spells a
marker itself.
