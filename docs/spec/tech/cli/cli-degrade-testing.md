---
type: tech
applies:
  - "crates/vibe/**"
  - "vibe/**"
updated: 2026-10-19
---

# Degrade rules, contracts, testing

- No peers installed → dispatch subcommands print one advice line, exit 0.
- Not a git repo → `worktree` subcommands exit 1 with one line.
- No registry / empty registry → `space list` prints "no repos registered",
  exit 0.
- Global library absent → merged answers equal repo-only answers; absence is
  a clean state, not a warning.
- Registry entry points at a moved/deleted path → one warning line, entry
  kept; repair is re-init or `--uninstall`, never a destructive guess.

## Contracts touched

- **Spec JSON v1** gains an additive `global: true` flag on `lessons-for`
  (additive-only within major, per [tech-contracts.md](../contracts.md)).
- **Payload v1, marker grammar v1, provider manifest v1** — unchanged;
  `space` and `worktree` inject through existing channels only.

## Testing posture

New code, not a port — no oracles. The house rules apply: every check
asserts its own population (registry floor, worktree floor ≥1 before "none
found" can pass); refusal paths get discriminating tests (the mid-flow
`remove` refusal must fail if replaced by a naive `rm -rf`); `worktree
create` is tested from a fresh non-git target and a bare `mktemp -d` clone
(the stranger-eval rule); every path this doc names is asserted from both
the main checkout and a worktree.
