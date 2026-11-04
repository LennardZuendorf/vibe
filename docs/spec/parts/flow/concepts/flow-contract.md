---
type: concept
updated: 2026-11-04
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

`state-machine.json` defines each `<flow>.<phase>` state's phase-file link,
delegates, write surface, exit predicate, and legal `next`; edge-keyed `gates`
name the two human approvals.

**F5/F6 source behavior:** the writer validates the target state name and saves
atomically. Rust writes the new cursor; active JS and shell readers/writers
prefer it once present, with the legacy cursor retained as a read fallback.
An invalid new cursor or ancestor never selects legacy state. Commands report
malformed/degraded state. Fallback requires established absence below valid,
searchable ancestors; directory listing permission is unnecessary.
Stop scans scoped NUL-delimited Git paths and escapes control bytes in diagnostics.
Cursor writes refuse pre-existing links that resolve outside the physical
project root. Stop prefers receipts in `.vibe/run/flow/evidence/`; it uses a
legacy receipt only when the corresponding new receipt is absent. A stale or
invalid new receipt or ancestor cannot fall back to an older location. Only
established absence below valid searchable ancestors selects legacy evidence.
Receipts need regular-file metadata, not read permission or parsed contents.
Every producer preserves the same relative `feature-${feature}.md` or `quick.md`
name, including accepted feature path components. This is the existing
receipt freshness contract; it does not enforce a verdict field.

**F7 and later target:** `vibe-flow state set` enforces legal `next` edges and
requires `--confirm` at human gates; `/flow` becomes a thin wrapper over that
check. The ledger records transitions, hook verdicts, and advisory signals;
TUI and playback consume the same cursor and ledger. These target guarantees
are not implemented by the F5/F6 writer. Full target contract:
[tech-rust.md](../../../tech/rust.md).

CLI usage errors exit 1, not hook block status 2. Spawned policy verdicts
remove trailing newlines only; leading whitespace never becomes a verdict.
Persisted numbers and true still name evidence receipts; containers use compact
JSON. Null, false and an empty string are unnamed. Numeric zero follows Bash
with jq and names a receipt; JS warns instead. Bash without jq reads only
quoted feature strings. The cursor writer still accepts only strings or null.
