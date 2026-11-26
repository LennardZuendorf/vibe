---
type: decision
updated: 2026-11-26
---

- **D49 — The spec root stays the durable owner.** Ruling on [#104](https://github.com/LennardZuendorf/vibe/issues/104): Vibe's spec root (`docs/spec/**`) remains the single owner of durable requirements — product, architecture, design, plan and lessons (D33, D44). OpenSpec does not maintain durable capability specs: `openspec/specs/<capability>/spec.md` is a change-format surface, what a change's deltas are written against and archived into, not a second durable home and never a promotion target. Native `openspec` stays a selectable feature backend; a capability fact it changes is promoted into the spec root by that fact's one writer (D46). D45's feature layer moves change artifacts only, never the durable root. Replacing the root needs a new equivalence decision; nothing migrates under this one.
