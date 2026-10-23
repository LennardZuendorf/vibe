---
type: tech
applies:
  - ".vibe/run/inject/**"
  - "contracts/**"
updated: 2026-10-19
---

# Payload v1

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
    {"id": "e17", "ts": "2026-09-04T10:00:00Z", "text": "guard: blocked docs/spec/plan.md"}
  ]
}
```

Fields: `level` — every-turn line (budget in
[tech-instruct.md](../instruct.md)). `edge` — full orders, injected when
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
