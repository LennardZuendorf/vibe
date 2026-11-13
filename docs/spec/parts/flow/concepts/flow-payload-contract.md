---
type: concept
updated: 2026-11-12
---

# Flow payload contract

D22's provider payload, written by F8: flow hands per-turn guidance to instruct through one file and one command, never composed prompt text.

## Payload v1

`.vibe/run/inject/flow.json`:

```json
{ "v": 1, "provider": "flow", "seq": 0,
  "level": "flow: idle · next: …", "edge": "<orders>", "session": ["…"], "events": [] }
```

- `level` — the every-turn line: state, feature, legal `next` (gated edges
  marked `(gate)`).
- `edge` — the full orders block plus the spec pointer, injected on the turn
  after a transition.
- `session` — timeless doctrine lines, capped, for the session-start channel.
- `events` — one-shot `{id, ts, text}` records from guard and Stop, appended by
  the hooks without changing their verdict.
- Accepted difference: a non-string `feature` sorts its keys (no `preserve_order` in `serde_json`); the JS oracle keeps insertion order, and a crate test pins it.

`state set` rewrites `level`/`edge`/`session` and bumps `seq`. `vibe-flow emit`
reprints the payload for the live cursor and never bumps `seq`; a per-turn bump
would inject the edge every turn.

## One write path

Payload writes reuse the ledger's pre-flighted write: the directory is created,
a symlinked leaf is refused (`refusing to append through the link …`), and the
existing leaf is proven readable **before** the cursor moves. A refused write
therefore never leaves `seq` bumped with no content behind it; a payload that
cannot be written is advisory, so the caller warns and continues (FV-R7).

## Provider manifest

`vibe-flow init` writes `.vibe/providers/flow.json` naming
`["vibe-flow", "emit"]` with `.vibe/run/inject/flow.json` as the fallback and seeds the
payload, so the file is readable before the first transition; a second `init` reports no change.

## Standalone AGENTS.md block

Owner `flow` under marker grammar v1 (`<!-- vibe:flow:begin v=1 hash=… -->`).
The body is rendered from `content/policy.json` — the same rules `policy
decide` enforces — so prose and enforcer cannot disagree. The ordered fragment
is `.vibe/run/agents-md/20-flow.md`. A second `render agents-md --write`
produces byte-identical output; `--check` exits 1 on drift.

## The spec peer is optional

`orders`, `doctrine` and `doctor` are parity-checked against the frozen JS
oracles. With no `vibe-spec` binary or no spec root the payload edge carries
only the path pointer and no tagged lessons; that path is a fixture test. Flow
never parses spec markdown — the crate carries no spec or markdown dependency,
and a test fails if one appears.
