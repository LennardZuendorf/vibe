---
type: area
code:
  - crates/vibe/src/main.rs
  - crates/vibe/src/cmd
  - crates/vibe/src/store
  - crates/vibe/src/adapters
  - crates/vibe/src/templates/opencode-plugin.js
  - crates/vibe/tests/cli_surface.rs
updated: 2026-11-12
---

# Configuration tool

Back to [CLI](../cli.md). Contract: [global store and projection](../../../tech/cli/cli-store.md).

`vibe config` owns `init`, `import-mcp`, `status`, `sync`, `install`,
`uninstall` and `doctor`. These operations share the existing local store,
projection handlers and ownership ledger. The parser groups them without
changing schemas, flags or storage. Old commands are removed without aliases.

`main.rs` owns the public tree and dispatch. `store/` owns initialization and
MCP import; `cmd/` owns projection orchestration, wiring, status and diagnostics.
Adapters and the OpenCode template retain `vibe hook session-start` as the
stable runtime entry. Embedded `vibe spec` keeps its complete raw argument tail.

Public CLI tests pin exact command rows, all seven operations, rejected old
spellings without effects, dry-run behavior and unchanged hook/spec entrypoints.
The package suite also generates OpenCode plugin fixtures for its Node tests.
Local checks do not establish live-user migration or installed-host behavior.
