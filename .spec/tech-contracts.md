---
type: tech-topic
parent: tech.md
scope: cross-tool contracts
covers: provider payload, marker grammar, spec JSON, provider manifests, exit codes, versioning
updated: 2026-09-04
---

# Cross-Tool Contracts — Tech

[tech.md](tech.md)

Branch doc for the only coupling between the three tools: versioned files
under `.vibe/`, tested byte-exact by `contracts/run.sh` in every tool's CI.
A tool never imports a peer; it reads these files or execs one soft peer
call, probed, absence-is-degrade.

## `contracts/`

```text
contracts/
├── payload.schema.json             # v1
├── marker.schema.json              # v1
├── spec-json.schema.json           # v1
├── provider-manifest.schema.json   # v1
├── fixtures/**                     # golden payloads, blocks, spec JSON
└── run.sh                          # validates every tool's output against the schemas
```

Additivity rule: a contract changes only by adding optional fields within a
major version. Readers ignore unknown fields. A breaking change bumps the
major and ships behind a new `v`; `doctor` warns when a peer's major differs
from what this tool emits or expects.

## Payload v1

Each provider writes `.vibe/run/inject/<provider>.json` atomically
(tmp + rename):

```json
{
  "v": 1,
  "provider": "flow",
  "seq": 42,
  "level": "flow: feature.impl · auth · next gate verify>compound",
  "edge": "<full orders markdown>",
  "session": ["doctrine line 1", "doctrine line 2"],
  "events": [
    {"id": "e17", "ts": "2026-09-04T10:00:00Z", "text": "guard: blocked .spec/plan.md"}
  ]
}
```

Fields: `level` — every-turn line (budget in
[tech-instruct.md](tech-instruct.md)). `edge` — full orders, injected when
`seq` has advanced past the session ledger; a fresh or `--resume`d session
has no ledger, so orders arrive on turn one regardless. `session` — timeless
doctrine lines for the `session-start` channel, ≤10. `events` — each `{id,
ts, text}` injected exactly once, then marked seen.

flow rewrites `level`/`edge`/`session` and bumps `seq` on `state set`; guard
and stop hooks append `events`. instruct keeps a per-session ledger at
`.vibe/run/instruct/sessions/<session_id>.json`, shaped `{provider: {seq,
event_ids}}`, to decide what is already seen. Budgets are enforced by
instruct at render time; a payload that would exceed a channel's budget is
truncated with a trailing `[truncated: run vibe-flow orders]` line rather
than silently dropped. Provider timeouts: 500 ms default, 1500 ms total wall
cap across all providers in a turn; a timeout or crash skips that provider
and emits one event line, never blocks the turn. `v` major mismatch → one
warn event, payload skipped.

## Provider manifest v1

`.vibe/providers/<id>.json`, repo or global tier only — never read from a
fetched source:

```json
{
  "id": "flow",
  "command": ["vibe-flow", "emit"],
  "channel": "user-prompt",
  "trigger": "edge",
  "budget_lines": 15,
  "timeout_ms": 500
}
```

`command` XOR `file` (a static payload path as fallback source). `vibe-flow
init` writes this manifest pointing at `["vibe-flow", "emit"]` with the
payload file itself as a fallback, so per-turn drift nudges keep working
even if instruct cannot resolve the command. Ordering: instruct's own
blocks render first, then providers in manifest order. Dedupe: instruct
keeps an exact-line set per session across all providers and the
`session-start` render; a duplicate line is dropped, not repeated.

## Marker grammar v1

```text
<!-- vibe:<owner>:begin v=1 hash=<sha256:12> -->
...block body...
<!-- vibe:<owner>:end -->
```

Owners and render order: `spec` (10), `flow` (20), `instruct` (30). Each
tool writes its own block standalone (`<tool> render agents-md --write`)
plus its fragment to `.vibe/run/agents-md/<NN>-<owner>.md`; instruct's
`write` recomposes all fragments in order and appends its rules block.
**Standalone bytes equal composed bytes** — uninstalling instruct changes
nothing for spec's or flow's block. Rules: bytes outside markers are never
touched; a malformed, nested, or duplicated marker pair exits 2 and writes
nothing; a managed block found in an ancestor AGENTS.md/CLAUDE.md is
refused, never overwritten; `--check` exits 1 on drift (hash mismatch), for
CI use. The grammar is implemented once, in `vibe-core`; no tool spells a
marker itself.

## Spec JSON v1

Four read-only queries, each `--json`, each exit 0 with a JSON object on
success:

| Query | Answers |
|---|---|
| `root` | absolute spec root + how it was found (`source`) |
| `lessons-for --tag <state>` | lessons tagged for the given flow state |
| `plan --feature <f>` | that feature's plan rows |
| `feature <f>` | that feature's product/tech/plan summary |

Exit 3 = no spec tree found at any point in the discovery order — the one
query-specific exit code (see the exit-code contract below). Degrade path:
if `vibe-spec` is absent from PATH, a peer checks only that
`<specroot>/features/<f>/` exists, using its own copy of the discovery
order, and falls back to a bare path pointer instead of the summarized
answer — never a hard failure.

## Exit-code contract

| Context | Code | Meaning |
|---|---|---|
| any CLI | 0 | ok |
| any CLI | 1 | refused (e.g. `state set` outside `next`, unconfirmed gate) |
| any CLI | 2 | usage error |
| `vibe-spec` JSON query | 3 | no spec tree found |
| hook | 0 | pass, or degraded (missing keystone: root, cursor, machine) |
| hook | 2 | block — guard/stop hard block, including present-but-corrupt policy (fail-safe) |

## Versioning and compatibility

Each tool releases on its own semver, tags `spec-v*`, `flow-v*`,
`instruct-v*`; `vibe-core` is unversioned (internal). The one compatibility
rule spanning all four contracts: additive-only within a major, unknown
fields ignored by readers, and `doctor` is the single place that checks a
peer's contract major against what this tool expects.
