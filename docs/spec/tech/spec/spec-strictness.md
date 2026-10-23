---
type: tech
applies:
  - "crates/vibe-spec/**"
  - ".vibe/spec.json"
updated: 2026-10-19
---

# Strictness

`.vibe/spec.json` `strictness: {"SF-nn": "warn|error|off"}`; a legacy
check keeps its default (errors stay errors), every other check
defaults to `warn`; `--strict` (CI) promotes every non-`off` check to
`error`. A check moves to `error` in this repo's own config only in the
compound that migrates the last live spec still failing it — never
sooner. A `--strict` run that examined zero files is refused, not
reported green: absence of findings is only evidence when presence of
input is proven.

Shipped (F17): every finding names one check in `vibe-spec`'s registry —
SF3, SF4, SF8–SF16 and I1 from the oracle, SF20–SF23 for the families it
left unnumbered (frontmatter, naming, links, tree), SF24–SF36 added by
F17. Keys accept `SF-nn` or `SFnn`; an unknown id warns and is ignored.
A configured level replaces a finding's default (legacy errors stay
errors, everything else warns); `--strict` turns warn into error; an
allow marker `<!-- vibe-spec: allow SF-nn -->` silences that check for
its file before either applies.
