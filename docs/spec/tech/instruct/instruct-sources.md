---
type: tech
applies:
  - "instruct/**"
  - ".vibe/instruct.json"
  - "instruct.lock.json"
updated: 2026-10-19
---

# Sources and sync

```json
{"type": "dir|git|https", "path|url": "...", "ref": "...", "commit": "...",
 "sha256": "...", "ttl": "24h", "scope": "global|repo"}
```

Repo-scope sources must pin (`commit` or `sha256`) or the loader refuses —
a clone must never fetch floating content. Global sources may float on
`ref`. `vibe-instruct sync`: fetch, verify pins, update
`instruct.lock.json`, rewrite generated files (below); offline keeps the
cache, prints one warning, exits 0. Session start does no network and no
subprocess, ≤200 ms wall: it compares the lock and cache mtimes against
each source's TTL and prints one line —
`instruct: 2 sources stale — run vibe-instruct sync` — or nothing. Fetched
content is data: only `blocks/*.md` and channel toggles are read from a
fetched `instruct.json`; providers, hook definitions, and any path
outside the source's own cache directory are ignored, with a warning.
