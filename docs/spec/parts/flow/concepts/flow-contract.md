---
type: concept
updated: 2026-11-03
---

# Vibe Flow Contract

Cursor JSON is unchanged in shape, at a new path,
`.vibe/run/flow/state.json`:

```json
{
  "flow": "idle | setup | strategy | feature | quick",
  "phase": "idle | detect | apply | brainstorm | spec | design | plan | impl | verify | compound | triage | fix",
  "feature": null,
  "updated": "2026-06-02T00:00:00Z"
}
```

`state-machine.json` defines each state's skill link, delegates, write
surface, exit predicate and legal `next`; edge-keyed `gates` name the two
human approvals. F5/F6 currently validate target names and write the new
cursor atomically. Reads prefer the new cursor, falling back to the legacy
cursor after a not-found read. This branch predates the newer ancestor,
receipt-location and physical-confinement hardening in its refreshed parent.

F7 legal-edge/approval enforcement, ledger and receipt verdicts remain
pending. F8 Rust injection/payloads, signals and TUI remain pending.

CLI usage errors exit 1, not hook block status 2. Spawned policy verdicts
remove trailing newlines only; leading whitespace never becomes a verdict.
Persisted numeric/boolean features still name evidence receipts; containers
use compact JSON. Only null or an empty string lacks a feature name.
