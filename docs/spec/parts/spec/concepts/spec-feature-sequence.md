---
type: concept
updated: 2026-11-12
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
| F19 | spec-hooks-agents | spec | Validate hooks (`validate --changed`/`--file`, exit 0); `using-spec` SessionStart router; blocking `PreToolUse` tooth (D47); agents `spec-promoter` + `spec-health` registered on both platforms; `spec-interviewer`/`spec-tracer` retired | Changed/unchanged root; tooth blocks unskilled edit and a discriminating test fails on `exit 0`; captured subagent payload fixture; `claude plugin validate --strict` in its own CI job on a pinned CLI, fail-closed when the CLI is missing (`spec/tests/run.sh`'s own call still skips on a host without Claude Code) | DONE | F16 |
| F20 | spec-migrate | spec | `setup.sh` port (`init`); `migrate --to docs/spec` (population assertion, allow-markers) + `--layout layered`; this repo → layered `docs/spec/` in one commit; the legacy `.spec/archive/` is deleted, not carried over | Dry-run table; zero literals after; full suite + `validate --strict` green | DONE | F17, F19 | <!-- literal -->
| F25 | spec-superpowers-default | spec | Assertive, trigger-only spec skill rewrite (pressure-tested); `using-spec` router text; superpowers-first delegate lines in `flow/`; `superpowers-frozen` write-policy rule; `docs/superpowers/` un-ignored | Pressure baseline 100 % with and without superpowers; flow + spec suites green | DONE | — |

## Spec projects and MCP

Back to [spec](../spec.md). Existing DONE F-rows retain their delivery meaning.
Tasks 1–9 are implemented and locally verified with independent review.
Release distribution and installer delivery remain separate.
The implemented surface adds shared embedded spec execution, registered two-mode project storage,
branch documents/lifecycle, explicit and conservative delayed reconciliation,
and local Rust stdio MCP. [Projects and MCP](../../../tech/spec/spec-projects-mcp.md)
records the durable contract. End-to-end acceptance lives in
`crates/vibe-spec/tests/project_e2e.rs` plus the central-only executable smoke
in the vibe integration tests; source presence is not a passing receipt.

Approval includes sole `vibe -> vibe-spec`, the reviewed spec MCP dependency
closure and a spec-only C1/F22 exception. General C1–C6, Flow, Instruct and
F21/F22 remain unchanged. This repository keeps `docs/spec/` canonical.
The root [branch-work row](../../../plan.md#branch-work) records completion;
the [contract](spec-framework-contract.md) and live technology note retain scope.
