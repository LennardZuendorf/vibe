---
type: hub
applies:
  - "instruct/**"
  - ".vibe/instruct.json"
  - ".vibe/blocks/**"
  - ".vibe/local/**"
  - "flow/content/**"
updated: 2026-10-19
---

# Instruct — Tech

[tech.md](../tech.md)

Branch doc for the instruct contract: how every injected sentence is
authored, layered across tiers, composed into channels, and budgeted.
Absorbs the former `tech-content.md`. **Shipped today:** `flow/content/`
(`policy.json`, `vibe.default.json`, `blocks/**.md`) plus a root `vibe.json`
override, rendered by the Node engine — one shipped/repo split, no global
tier, no sources, no providers. This doc describes the v0.4 target.

## Notes

- [Tiers](instruct/instruct-tiers.md) — global, repo-shared, repo-local, session-runtime
- [Budgets, channels, block format](instruct/instruct-budgets-blocks.md) — channel budgets and the block file format
- [Lints and provider execution](instruct/instruct-lints-providers.md) — lint rules and the provider side of the payload contract
- [Sources and sync](instruct/instruct-sources.md) — pinned sources, sync, session-start staleness line
- [File targets, rewrite triggers, commands](instruct/instruct-targets-commands.md) — platform targets and the v1 command set
- [Migration](instruct/instruct-migration.md) — splitting today's Active-Rules block
