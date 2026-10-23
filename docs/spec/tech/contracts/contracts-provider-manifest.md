---
type: tech
applies:
  - ".vibe/providers/**"
  - "contracts/**"
updated: 2026-10-19
---

# Provider manifest v1

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

`command` and `file` (a static payload path) may both be present: `command`
is tried first; `file` is the fallback source, used only when the command
is absent, fails, times out, or is unresolvable. `vibe-flow init` writes
this manifest pointing at `["vibe-flow", "emit"]` with the payload file
itself as a fallback, so per-turn drift nudges keep working even if
instruct cannot resolve the command. Ordering: instruct's own blocks
render first, then providers in manifest order. Dedupe: instruct keeps an
exact-line set per session across all providers and the `session-start`
render; a duplicate line is dropped, not repeated.

Repo-tier manifests are committed content — a cloned or forked repo
carries them in before any human has reviewed them. Before executing a
`command` from a repo-tier manifest, instruct requires explicit user
approval or a match against a user-configured allowlist; a repo-tier
manifest does not grant execution trust by itself. Global-tier manifests
live outside the repo (`~/.config/vibe/`) as personal config the user
already controls, and run without this gate.
