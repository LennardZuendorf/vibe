---
type: concept
updated: 2026-10-19
---

# Design Principles

1. **Composition over reimplementation.** Flow phases route to existing skills instead of copying their workflows.
2. **Specs are memory, flow is runtime.** The spec tree records durable thinking; the flow cursor records the current agent state.
3. **Agent skills are the command surface.** The recurring workflow is expressed as skills agents can invoke, not as loose markdown snippets.
4. **Adapters stay thin.** Platform-specific files translate runtime events into tool invocations; they own no state of their own.
5. **Superpowers first, vibe as memory.** Feature design and plans use superpowers' own artifacts; vibe promotes their durable facts into the layered spec, then deletes them (D45, D46).
6. **Small shims, shared machinery.** State transitions and deterministic checks belong in the core crate; each tool's surface stays concise.
7. **Context is the scarce resource.** Every always-on line competes with the work. Prefer a hook (free) over prose; prefer edge-triggered over per-turn; prefer an on-demand skill over a preloaded block.
8. **Say what to do, and why.** Injected text is positive and motivated — the desired action plus its reason — not a list of prohibitions. Prohibitions that matter belong in the guard, where they are enforced rather than hoped for.
9. **Data over prose.** Anything the machine knows (write surfaces, legal edges, delegates, invariants) is interpolated from data, never restated in English.
10. **Independence over convenience — each tool is a whole product.** spec, flow, and instruct each install, run, and prove useful with the other two absent; convenience coupling belongs in the installer, not the binaries.
11. **Contracts, not imports.** Tools exchange versioned files under `.vibe/` and one soft probe call, never a shared library or an in-process import.
12. **Signals advise, gates enforce.** A progress signal is evidence for the TUI and injected text; only the state-machine writer and the guard refuse an action.
