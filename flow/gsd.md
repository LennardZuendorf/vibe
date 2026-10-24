# GSD compatibility

GSD aliases are additive entry points into the vibe flow. Claude uses `/gsd-x`;
Codex uses `$gsd-x`. Codex alias derivation is deferred. The alias table is
[reference/gsd-aliases.json](reference/gsd-aliases.json); it is data for F7, not
a second state machine.

The cursor owns sequencing. `.planning/` is read-only context, and `STATE.md` is
never quoted as authority. Gate rules and transitions follow the SKILL.md
Contract.

## Read surface

- `ROADMAP.md`: map a phase number to its slug for feature context.
- `STATE.md`: advisory display only in `status`.
- `VERIFICATION.md`: fold its evidence into the flow verify receipt.
- `PLAN.md`: input while planning that phase.
- All other `.planning/` files are context only; edits go through GSD commands.

Unknown GSD flags are accepted and ignored with a one-line note. They are never
reinterpreted as vibe options.

## No mapping

- `gsd-new-milestone`, `gsd-complete-milestone`, `gsd-milestone-summary`: no
  milestone layer exists in vibe.
- `gsd-ui-phase`, `gsd-ui-review`: UI phase/review has no flow state.
- `gsd-workspace`: workspace lifecycle is outside this flow.
- `gsd-phase`: ambiguous shorthand; use a named alias.
- `gsd-quick-batch`: batched quick work has no single-state mapping.
- `gsd-review`, `gsd-capture`: no standalone state or capture store maps here.
- Namespace routers: resolve aliases directly; no router state is needed.
- `--wave`, `--cross-ai`: orchestration flags have no vibe equivalent.
