---
type: tech
applies:
  - "oracles/**"
  - "tests/parity/**"
  - "crates/vibe-parity/**"
updated: 2026-11-03
---

# Parity pins and port order

A pin states the values it expects, not just the field. `expect` is required
and validated at load: an `exit` pin names both codes; a `stdout`/`stderr` pin
names a prefix of each side's first differing line (`null` = no such line, `""`
= an empty line); a `files` pin names each differing key, which side left it,
and the line it starts with. Two failures follow: `pin drift` — the field
diverges on other values than the pin states, so the reason below would say the
opposite of what happens — and `stale pin` — the field no longer diverges.
Drift is a run failure, not a suite error, and prints expected and observed.
A pin that states nothing — which let a crashing candidate read as `PINNED` on
an `exit` pin — is a suite error.

Bytes, not lines, decide whether a stream or file differs: splitting drops a
final newline, so two sides equal *as lines* can still differ, and a line
comparison would call that a pass. A difference with no differing line is
**unpinnable** (`stdout: unpinnable — the lines all agree, and only the final
newline differs`): the run fails, since an empty-side pin is refused at load,
and pins on that field are claimed rather than reported stale. `files` follows
the same rule instead of reading as two empty files.

A rendered line escapes non-UTF-8 bytes as `\xNN`, so pins are ambiguous in
principle with candidate text that spells that escape; accepted.

Pins are per argument set and may be scoped to one OS (`"os": "macos"` for
BSD-tool differences; never stale-checked). macOS CI does not run parity: the `parity`
job is ubuntu-only and `tests/run.sh` never calls it, so an os-scoped pin runs
only under `bash tests/parity/run.sh` by hand on that OS — the macOS pin is
unverified until someone does.

Twenty pins cover the spec half (one os-scoped), ten the flow half: legacy
`--format json` output is valid JSON — quotes, backslashes and control
characters are escaped, and invalid UTF-8 (a preview cut inside a sequence)
becomes U+FFFD (`lessons-for`, `scan-merges`); `lessons-for` tags and the
`validate` I1 own-feature filter match literally, not as regexes; SF14 compares
scope items as bytes (no awk escape or numeric comparison); the no-root
`validate` error names `vibe-spec init`; BSD `wc -l` pads the SF15 count
(macOS only). F17's pins are the three bug fixes whose output differs from the
oracle: whole-word R-IDs, capital whole-word RFC 2119 keywords, and Scope words
inside the `## Scope` section.

Port order, blast-radius first: guard → stop → cursor/`state set` → policy →
root → machine/orders/doctrine → payload/compose → doctor → merge-settings →
merge-agents/render (instruct) → spec scripts. The JS engine is deleted in one
commit, only when every unit is parity-green; oracles stay in the tree
permanently. CI job `port-order` fails the build if `crates/vibe-flow/src/tui/`
or `crates/vibe-signals/` exists while `tests/parity/GREEN.hard` is absent.

Carried into later rows: F5 freezes `flow/reference/templates/AGENTS.md`
(merge-agents reads it) and adds the policy-dialect matcher; F6 bounds git's
own discovery in the sandbox (`GIT_CEILING_DIRECTORIES` or a real `git init`).
