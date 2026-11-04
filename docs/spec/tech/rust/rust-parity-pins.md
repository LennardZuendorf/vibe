---
type: tech
applies:
  - "oracles/**"
  - "tests/parity/**"
  - "crates/vibe-parity/**"
updated: 2026-11-04
---

# Parity pins and port order

A pin requires validated `expect` values. An `exit` pin names both codes.
A `stdout`/`stderr` pin names prefixes of both first differing lines:
`null` means no such line; `""` matches only an empty line.
A `files` pin names the exact set of differing keys, each side's presence,
and prefixes of its first differing line (first line for a one-sided file).
Missing or malformed expectations fail suite load. A crash cannot satisfy
an exit pin that states another code. `pin drift` fails the run and prints
expected and observed values; `stale pin` means the field no longer diverges.

Byte comparison decides stream and file differences. Equal lines with a
changed final newline remain unequal and **unpinnable**: the run fails,
including when a pin names that field. Such pins are claimed, not stale.
Rendered lines escape invalid UTF-8 bytes as `\xNN`; ambiguity with text
that literally spells that escape is an accepted limitation.

Pins match argument sets and may be OS-scoped (`"os": "macos"` for BSD
variation); OS pins are never stale-checked. The parity CI job runs only on
Ubuntu, and `tests/run.sh` does not call it. Run `bash tests/parity/run.sh`
on macOS to verify the macOS pin; report that result separately.

Thirty-seven spec pins and ten flow pins cover the current suites.
Legacy `--format json` escapes quotes, backslashes and control characters;
invalid UTF-8 becomes U+FFFD (`lessons-for`, `scan-merges`). Tags and the
`validate` I1 own-feature filter match literally. SF14 compares scope bytes
without awk escape or numeric coercion. The no-root `validate` error names
`vibe-spec init`; BSD `wc -l` pads SF15 counts. F17 pins whole-word R-IDs,
capital whole-word RFC 2119 keywords, and Scope words inside `## Scope`.

Port order, by blast radius: guard → stop → cursor/`state set` → policy →
root → machine/orders/doctrine → payload/compose → doctor → merge-settings →
merge-agents/render (instruct) → spec scripts. Delete the JS engine only
when every unit is parity-green; retain frozen oracles permanently.
CI `port-order` rejects `crates/vibe-flow/src/tui/` or `crates/vibe-signals/`
unless `tests/parity/GREEN.hard` exists.

F5 freezes `flow/reference/templates/AGENTS.md` for merge-agents and adds
the policy-dialect matcher. F6 bounds sandbox git discovery with
`GIT_CEILING_DIRECTORIES` or a real `git init` before stop-gate oracles run.

## Changed-path scans use NUL output scoped to the project
The engine Stop gate's staleness scan (`flow/engine/commands/hook.mjs`, the behaviour F6 must keep) runs `git -C <root> rev-parse --is-inside-work-tree --show-prefix`, then `git -C <root> status --porcelain -z -uall -- .`. It parses records by NUL, consumes the origin field of an `R` or `C` record, strips the prefix, and drops entries outside it. Porcelain v1 paths are toplevel-relative, so a scan without the prefix strip misses every change in a nested project. A stat error other than `ENOENT` or `ENOTDIR` blocks (fail closed), and so does a not-found name that decoded to U+FFFD. The bash fallback (`flow/hooks-fallback/stop-gate.sh`) reads `-z -- .` through process substitution and `read -r -d ''`, never through `$(...)`, which drops NULs. It keeps three gaps, each pinned in `parity.test.mjs` as KNOWN DIVERGENCE: no `-uall`, a stat error is skipped, and a non-UTF-8 name is judged by its raw bytes. The fallback's TDD predicate keeps its plain-porcelain read on purpose; the engine's `predicateTdd` reads the legacy text rebuilt from the decoded records.

## Content confinement is physical, not lexical
`confinePath` stays as the cheap lexical pre-check. `confineRealWhy(base, rel, {forWrite})` is the capability gate. It runs `realpathSync` on both the anchor and the target (macOS `/tmp` is `/private/tmp`), then a separator-aware prefix test. In-repo symlinks are allowed. For a missing write target it resolves the deepest existing ancestor. A dangling write link is refused. Every config-named or walked read goes through it: `walkMarkdown`, `loadBlocksDir` (shipped and `.vibe/blocks`), `applyConfigBlocks`, both reads in `lessonsFor`, and `render`'s `writeTarget`. A refusal is reported, never skipped silently. The frozen JS oracles keep the lexical behaviour, and the Rust port (F11) must pin these cases as expected divergences.
