---
type: concept
updated: 2026-10-19
---

# Cross-Tool Contracts

Four versioned contracts under `contracts/`, additive within a major,
tested byte-exact by every tool's CI:

- **Payload v1** — `.vibe/run/inject/<provider>.json`: `level`/`edge`/`events`
  fields plus `session` doctrine lines and `seq` for edge detection.
- **Provider manifest v1** — `.vibe/providers/<id>.json`: command or file
  source, channel, trigger, budget, timeout.
- **Marker grammar v1** — `<!-- vibe:<owner>:begin v=1 hash=<sha256:12> -->`
  … `<!-- vibe:<owner>:end -->`, owners spec/flow/instruct.
- **Spec JSON v1** — `root`, `lessons-for`, `plan`, `feature` queries,
  exit 3 on no tree. v0.5 adds an additive `global: true` flag on
  `lessons-for`.

Full schemas, examples, and the exit-code contract:
[tech-contracts.md](../tech/contracts.md).
