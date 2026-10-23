---
type: concept
updated: 2026-10-19
---

# Design Philosophy

1. **File-based contracts.** Specs, cursor, injection payload, and marker
   blocks are ordinary files any tool or agent can read and validate.
2. **Separation of durability.** The spec tree is committed memory; the
   cursor, ledger, and injection payload are runtime state, gitignored under
   `.vibe/run/`.
3. **Independence over convenience.** Each tool installs and works alone.
   Anti-bundling is a design value, not a phase: no tool ever requires
   another. (v0.5's `vibe` CLI is a front door over independent tools, not a
   bundle of them.)
4. **One core crate, three binaries.** `vibe-core` holds every primitive a
   tool needs — root resolution, atomic writes, marker grammar, hook-input
   parsing. No tool crate depends on another.
5. **Contracts are the only coupling.** Cross-tool traffic is versioned files
   under `.vibe/` plus one soft exec, never an import.
6. **Injection is budgeted and trigger-classed.** Per-turn payload splits
   level / edge / event so live state never balloons the transcript.
7. **Enforce in code, explain in prose, never both.** An invariant the guard
   blocks is rendered in the guard's verdict, not preloaded into always-on
   context.
8. **The writer is the gate.** `state set` itself refuses an illegal or
   unconfirmed transition — approval is a mechanism, not a convention.
9. **Ported code proves itself.** Every Rust unit ships against a frozen
   oracle and a differential parity harness before it replaces the original.
