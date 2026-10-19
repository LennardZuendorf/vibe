---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "docs/spec/**"
  - "docs/superpowers/**"
updated: 2026-10-19
---

# Adherence checks

All warn-first, one SF-number each: layout allowlist (root hubs + `README.md`/`AGENTS.md`/`lessons.md`,
kind folders `areas|flows|concepts|platform|tech|decisions|questions`,
`parts/<part>/` with the same kinds, `features/<name>/{product,tech,design,plan,research}.md`,
`archive/<name>/**`, `quick/<slug>.md`, `templates/**`, and
`docs/superpowers/{specs,plans}/*.md` — anything else fails); budgets
(hub ≤80, note ≤60); link resolution; unique names; typed frontmatter
with `code:` existence on areas and `applies:` on tech; every kind note
reachable from a hub; required feature files present; frontmatter
`type/scope/children/updated` complete and enum-valid; every `children:`
path exists and every branch doc backlinks its parent; feature folder
name matches its plan row, a row with no folder is drift; `updated:`
older than the file's last commit; grammar (≥1 scenario per requirement,
WHEN and THEN present, SHALL/MUST word-bounded).

Shipped (F17): SF24 requirement population, SF25 scenario WHEN/THEN,
SF26 frontmatter values, SF27 children and backlinks, SF28 feature
folder → plan row, SF29 freshness against git, and on layered roots
(a root hub declares `type: hub`) SF30 allowlist, SF31 budgets, SF32
names, SF33 typed notes (`code:`, `applies:`), SF34 hub reachability;
in both layouts SF35/SF36 check `docs/superpowers/` names and each
plan's `**Spec:**` path. `trace` reads units from fallback feature plans
only; superpowers plans carry no unit ids.
`trace` refuses on zero units (exit 1, nothing on stdout). A shallow clone skips commit citations (`history: none`, stderr note) unless `--since` names a reachable ref. Any word-bounded `<feature>/<n>` token counts as a citation, including path fragments under `tests/`.

`vibe-spec trace [--since <ref>]`: cross-references `<name>/n` unit
citations from `git log` and `tests/**` against the plan, reporting
uncited units and orphan citations — a plan row nothing ever touched, or
a commit that cites a unit that does not exist.
