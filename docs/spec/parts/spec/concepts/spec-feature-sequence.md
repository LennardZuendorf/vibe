---
type: concept
updated: 2026-10-19
---

# Spec feature sequence

Rows of the Feature Sequence owned by the spec tool; delivery order and binary gates are in [the feature sequence](../../../concepts/feature-sequence.md).

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F1 | spec-path-hotfix | spec | Skill-relative path fix in `validate.sh`/`list-specs.sh`/`lessons-for.sh`/`scan-merges.sh`; upward marker search; widened path-guard test | `spec/tests/run.sh` green from a subdirectory and a bare tmp dir | DONE | — |
| F2 | contracts | spec | `contracts/` Spec JSON v1 schema + goldens (`root`, `lessons-for`, `plan`, `feature`), exit-code contract, `run.sh`; payload, provider manifest and marker grammar move to their tools' rows | Green on empty tools; wired into CI | DONE | — |
| F16 | spec-core | spec | `vibe-spec` crate; root discovery; spec oracles frozen; `validate`/`list-specs`/`lessons-for`/`scan-merges` ported | Oracle diff + population floor; subdir invocation; exit 0/3 goldens | DONE | F3, F4 |
| F17 | spec-validators | spec | Substring-bug fixes; strictness config + `--strict`; grammar checks + unified R-ID; `trace`; layered-layout checks (D44: allowlist, budgets, link resolution, unique names, typed frontmatter, `code:` existence, hub reachability); superpowers artifact name + `**Spec:**` link checks | One landed mutant per bug fails; warn/error/off matrix; OpenSpec samples pass; layered fixture tree passes, each mutated rule fails | DONE | F16 |
| F18 | spec-delta | spec | Header-keyed ADDED/MODIFIED/REMOVED promotion engine; `promote.sh` port incl. superpowers promotion checklist and `promote <feature> --done` (D46); `check-drift` port with `--pre-merge`; `export\|import --openspec` | Golden trees; MODIFIED replaces in place; round-trip byte-identity; `--done` refuses on unchanged `docs/spec/` | DONE | F17 |
| F19 | spec-hooks-agents | spec | Validate hooks (`validate --changed`/`--file`, exit 0); `using-spec` SessionStart router; blocking `PreToolUse` tooth (D47); agents `spec-promoter` + `spec-health` registered on both platforms; `spec-interviewer`/`spec-tracer` retired | Changed/unchanged root; tooth blocks unskilled edit and a discriminating test fails on `exit 0`; captured subagent payload fixture; `claude plugin validate` | IN PROGRESS — local gate green; CI pending push | F16 |
| F20 | spec-migrate | spec | `setup.sh` port (`init`); `migrate --to docs/spec` (population assertion, allow-markers) + `--layout layered`; this repo → layered `docs/spec/` in one commit; `docs/spec/archive/` deleted (the legacy `.spec/archive/` is not carried over) | Dry-run table; zero literals after; full suite + `validate --strict` green | NOT STARTED | F17, F19 | <!-- literal -->
| F25 | spec-superpowers-default | spec | Assertive, trigger-only spec skill rewrite (pressure-tested); `using-spec` router text; superpowers-first delegate lines in `flow/`; `superpowers-frozen` write-policy rule; `docs/superpowers/` un-ignored | Pressure baseline 100 % with and without superpowers; flow + spec suites green | DONE | — |
