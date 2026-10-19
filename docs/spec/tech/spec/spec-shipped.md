---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "spec/**"
  - "tests/parity/**"
updated: 2026-10-19
---

# Shipped today

F1 (`spec-path-hotfix`, plan.md) is delivered. `validate.sh`,
`list-specs.sh`, `lessons-for.sh`, and `scan-merges.sh` walk up from the
cwd to the directory holding `.git`; a relative `SPEC_DIR` resolves
against the root found. `promote.sh`, `setup.sh`, and `check-drift.sh`
are unchanged (`check-drift.sh` already searched upward); run `promote.sh`
and `setup.sh` from the project root. The skill, the spec subagents, and
`feature.md` reference scripts by skill-relative path. F25
(`spec-superpowers-default`) is delivered: the pressure harness
(`spec/tests/pressure/run.sh`) passes 12/12 with and without superpowers,
against a 5/12 baseline for the pre-rewrite skill.
F16 adds the `vibe-spec` binary (dev build: `cargo build -p vibe-spec`).
Every command goes through one discovery function. It offers `root`,
byte-identical ports of `list-specs`, `lessons-for`, `scan-merges` and
`validate`, and the `lessons-for --json`, `plan` and `feature` queries.
`tests/parity/run.sh` proves the ports against the frozen oracles over the
corpus. The live leg on this repo is retired (P4-R17), because the oracle
scripts cannot read the layered root. CI runs `vibe-spec validate --strict`,
`check-drift` and `migrate --check`. The skill and the template call
`vibe-spec`; `install.sh` still seeds through `setup.sh` until F22.
F17 adds `validate --strict`, `--format text|json` (`--json`), the check registry and strictness config, and `trace`. `--changed`, `--file` and `--format hook` arrive with F19 (see Hooks). The fixes (R-ID prefix, RFC 2119 boundary, Scope scoping, zero-requirement short-circuit) land in Rust only; `spec/scripts/validate.sh` keeps the oracle's behaviour until the binary ships.
F18 adds `check-drift` (`--pre-merge`, `--json`), `promote` (deltas, the superpowers checklist, `--done`, `--json`), `export --openspec` and `import --openspec` to `vibe-spec`, with byte parity for the two ports over the corpus. Skills, CI and flow called `spec/scripts/*.sh` until F20 (P3c-R2); they now call `vibe-spec`. The OpenSpec gate runs in its own CI job (`tests/openspec/run.sh`, network).
F20 adds `init` and `migrate`, and this repo now keeps its spec as a layered tree at `docs/spec/`.
F19 adds `hook {guard,validate,session-start}`, `validate --changed|--file|--format hook`, `setup`, the shim and wiring, and the `spec-promoter` and `spec-health` agents. See Subagents and Hooks.
