---
type: tech
applies:
  - "contracts/**"
updated: 2026-10-19
---

# `contracts/`

```text
contracts/
├── spec-json.schema.json           # v1 (F2)
├── exit-codes.json                 # v1 (F2), with exit-codes.schema.json
├── fixtures/spec-json/<query>/{valid,invalid}/*.json
├── lib/schema.mjs                  # JSON Schema subset validator (Node, zero deps)
├── run.mjs                         # validates schemas, goldens, exit codes, floors
└── run.sh                          # entry point; also runs contracts/tests/
```

`payload`, `marker` and `provider-manifest` schemas arrive with their
tools' rows.

Additivity rule: a contract changes only by adding optional fields within a
major version. Readers ignore unknown fields. A breaking change bumps the
major and ships behind a new `v`; `doctor` warns when a peer's major differs
from what this tool emits or expects.
