---
type: tech
applies:
  - "oracles/**"
  - "tests/parity/**"
  - "crates/vibe-parity/**"
updated: 2026-10-19
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
