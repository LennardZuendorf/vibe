---
type: tech
applies:
  - "contracts/exit-codes.json"
  - "crates/**"
updated: 2026-10-19
---

# Exit codes and versioning

| Context | Code | Meaning |
|---|---|---|
| any CLI | 0 | ok |
| any CLI | 1 | findings or refused (e.g. drift found; `state set` outside `next`, unconfirmed gate) |
| any CLI | 2 | usage error |
| `vibe-spec` JSON query | 3 | no spec tree found |
| hook | 0 | pass, or degraded (missing keystone: root, cursor, machine) |
| hook | 2 | block — guard/stop hard block, including present-but-corrupt policy (fail-safe) |

Every command, legacy ones included, exits 3 when an explicit root (`--root`,
`$VIBE_SPEC_ROOT`, config) names a missing directory (R4). Without an explicit
root, only `root`, `--json`, `plan`, `feature`, `validate --format json` and
`trace` exit 3, while text-mode `validate` keeps the oracle's exit 1;
`validate --json` and `trace --json` print their JSON object on exit 1 too, and
`trace` prints nothing on the zero-unit refusal.

## Versioning and compatibility

Each tool releases on its own semver, tags `spec-v*`, `flow-v*`,
`instruct-v*`; `vibe-core` is unversioned (internal). The one compatibility
rule spanning all four contracts: additive-only within a major, unknown
fields ignored by readers, and `doctor` is the single place that checks a
peer's contract major against what this tool expects.
