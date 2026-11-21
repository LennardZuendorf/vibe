---
type: concept
updated: 2026-11-21
---

# Spec vs Implementation

F1–F4, F16–F20 and F25 are the completed Spec and foundations scope. Flow F5–F8 are merged and the [Flow sequence](../parts/flow/concepts/flow-feature-sequence.md) records their evidence. [#96](https://github.com/LennardZuendorf/vibe/pull/96) also merged an opt-in v2 runtime core. Trusted verification and scoped action authority remain open in [#101](https://github.com/LennardZuendorf/vibe/issues/101). F9 can enter design on owner approval. F10 still waits on F9.

Instruct F11–F15 and its [integration contracts](https://github.com/LennardZuendorf/vibe/issues/83) are back in active planning. The Rust runtime is pending. The existing distribution and retirement gates remain in the [shared sequence](shared-feature-sequence.md). No versioned release artifacts are available.

C0 shared configuration is implemented under `vibe config`, with stable
`vibe hook session-start` and embedded `vibe spec`. The configuration namespace
change is DONE; old configuration commands have no aliases.
See [the binary contract](../tech/cli/cli-binary.md).

The merged [Spec project and MCP commands](../tech/spec/spec-projects-mcp.md) add registered repo-first/global-first storage, managed documents and local stdio MCP. They are a limited source carve-out. The general v0.5 arc (C1–C6) remains pending. The [product milestones](product-milestones.md) allow the reference installer and worktree APIs to be planned early.
