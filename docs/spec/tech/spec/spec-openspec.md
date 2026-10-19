---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "tests/openspec/**"
updated: 2026-10-19
---

# OpenSpec interchange

Grammar-compatible, layout-native: vibe adopts OpenSpec's requirement
grammar inside its own document model, not its directory layout.

Adopted verbatim: `### Requirement: <ID> — <name>` (SHALL/MUST),
`#### Scenario:` with bulleted GIVEN/WHEN/THEN, `## ADDED|MODIFIED|REMOVED
Requirements` deltas.

| vibe | OpenSpec |
|---|---|
| a root document with blocks under `## Requirements` (`product-auth.md`, `concepts/invoice.md`) | `openspec/specs/<path without .md>/spec.md`; `## Purpose` is generated |
| fallback feature `product.md` | `changes/<f>/proposal.md` + one delta spec for its target's capability (plain `## Requirements` blocks export as ADDED) |
| feature `plan.md` units | `changes/<f>/tasks.md` (`- [ ] 1.<n> <title>`, `[x]` when DONE) |
| the five feature files (`product\|tech\|design\|plan\|research.md`); any other file in the folder is not exported (stderr note `export: <f>/<file>: not exported`) | carried byte for byte in `changes/<f>/design.md` as `## vibe:<file> <len>` blocks after `<!-- vibe:export v1 -->` |

Requirements are keyed by their full header name, normalized as OpenSpec
does (closing `#` run dropped, trimmed, case-sensitive). `RENAMED` is not
supported: write REMOVED + ADDED. A REMOVED block carries a `**Reason**:`
line. `promote <f>` applies a fallback feature's delta sections to its
`parent:` document, or to `<root>/product.md` when there is no `parent:`
(REMOVED, then MODIFIED, then ADDED; all or nothing). `<!-- merge -->`
blocks in the feature's `tech.md` append to `<root>/tech.md`, or to
`--target <file>`.

`vibe-spec export --openspec <dir>` refuses an existing `<dir>/openspec/`
and a MODIFIED that drops a scenario its target still has. `import
--openspec <dir> [--feature <name>]` writes fallback feature folders only,
never root documents. It restores carried files byte for byte, but refuses
when the change's requirements were edited on the OpenSpec side. It refuses
`RENAMED` and changes that touch more than one capability. A change without
carried files is imported as a new feature. Round trip: export -> import is
byte-identical. `openspec validate --strict` runs on the exported directory
in CI (`tests/openspec/run.sh`), never on the tree itself.

**Lossy / not adopted.** The round trip is lossless for requirements,
scenarios, deltas and tasks. It is lossy for: `updated:` dates on a foreign
import (stamped with today's date), branch docs, lessons, drift state, feature
files that are not carried (a stderr note names each), and `proposal.md` or
`design.md` prose edited on the OpenSpec side (import prints `import: <id>:
<file> edits are not imported`). An edited `tasks.md` is refused instead:
edit `plan.md` in vibe. Not adopted: the `openspec/` root as the source of
truth, archive semantics, and `RENAMED`. `promote <f> --done` deletes the
bound superpowers artifacts permanently (a filesystem remove; untracked files
included).
