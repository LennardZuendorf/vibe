---
type: tech
applies:
  - "oracles/**"
  - "tests/parity/**"
  - "crates/vibe-parity/**"
updated: 2026-10-24
---

# Parity pins and port order

Pins are per argument set and may be scoped to one OS (`"os": "macos"`
for BSD-tool differences; an os pin is never stale-checked). Thirteen pins (ten from F16, three from F17): legacy `--format json` output is valid JSON — quotes, backslashes
and control characters are escaped, and invalid UTF-8 (such as a preview
cut inside a sequence) becomes U+FFFD (`lessons-for`, `scan-merges`);
`lessons-for` tags and the `validate` I1 own-feature filter match
literally, not as regexes; SF14 compares scope items as bytes (no awk
escape or numeric comparison); the no-root `validate` error names
`vibe-spec init`; BSD `wc -l` pads the SF15 count (macOS only).
F17's pins are the three bug fixes whose output differs from the oracle: whole-word R-IDs, capital whole-word RFC 2119 keywords, and Scope words inside the `## Scope` section.

Port order, blast-radius first: guard → stop → cursor/`state set` → policy
→ root → machine/orders/doctrine → payload/compose → doctor →
merge-settings → merge-agents/render (instruct) → spec scripts. The JS
engine is deleted in one commit, only when every unit is parity-green;
oracles stay in the tree permanently. CI job `port-order` fails the build
if `crates/vibe-flow/src/tui/` or `crates/vibe-signals/` exists while
`tests/parity/GREEN.hard` is absent — the TUI and signals cannot get ahead
of the two hard blocks.

Carried into later rows: F5 freezes `flow/reference/templates/AGENTS.md`
(merge-agents reads it) and adds the policy-dialect matcher; F6 bounds
git's own discovery in the sandbox (`GIT_CEILING_DIRECTORIES` or a real
`git init`) before stop-gate oracles run.

## Changed-path scans use NUL output scoped to the project
The engine Stop gate's staleness scan (`flow/engine/commands/hook.mjs`, the behaviour F6 must keep) runs `git -C <root> rev-parse --is-inside-work-tree --show-prefix`, then `git -C <root> status --porcelain -z -uall -- .`. It parses records by NUL, consumes the origin field of an `R` or `C` record, strips the prefix, and drops entries outside it. Porcelain v1 paths are toplevel-relative, so a scan without the prefix strip misses every change in a nested project. A stat error other than `ENOENT` or `ENOTDIR` blocks (fail closed), and so does a not-found name that decoded to U+FFFD. The bash fallback (`flow/hooks-fallback/stop-gate.sh`) reads `-z -- .` through process substitution and `read -r -d ''`, never through `$(...)`, which drops NULs. It keeps three gaps, each pinned in `parity.test.mjs` as KNOWN DIVERGENCE: no `-uall`, a stat error is skipped, and a non-UTF-8 name is judged by its raw bytes. The fallback's TDD predicate keeps its plain-porcelain read on purpose; the engine's `predicateTdd` reads the legacy text rebuilt from the decoded records.

## Content confinement is physical, not lexical
`confinePath` stays as the cheap lexical pre-check. `confineRealWhy(base, rel, {forWrite})` is the capability gate. It runs `realpathSync` on both the anchor and the target (macOS `/tmp` is `/private/tmp`), then a separator-aware prefix test. In-repo symlinks are allowed. For a missing write target it resolves the deepest existing ancestor. A dangling write link is refused. Every config-named or walked read goes through it: `walkMarkdown`, `loadBlocksDir` (shipped and `.vibe/blocks`), `applyConfigBlocks`, both reads in `lessonsFor`, and `render`'s `writeTarget`. A refusal is reported, never skipped silently. The frozen JS oracles keep the lexical behaviour, and the Rust port (F11) must pin these cases as expected divergences.
