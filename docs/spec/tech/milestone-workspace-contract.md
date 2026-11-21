---
type: tech
applies: ["crates/vibe/**", "crates/vibe-spec/**"]
updated: 2026-11-21
---

# Multi-repo workspace contract

A workspace maps one feature to several Git worktrees. Repository and feature
artifact ownership remain explicit. Each member is a separate checkout and
retains its own project context and Git lifecycle.

## Contracts

- Record workspace identity and each member's registered repo, canonical
  checkout, base and branch, selected feature context and creation state.
  Resolve members from explicit registry entries. Detect duplicate or mismatched
  members before mutating a destination.
- Keep each member's runtime state local. Reuse registered Spec context and its
  repo-first or global-first semantics. The selected backend and authority
  determine where feature documents live and where other members link to them.
- Reuse existing config projection ownership and conflict handling. Resolve
  user-level store separately from worktree-local overrides and generated
  project files. Repeating bootstrap leaves already-correct state intact.
- Make create/resume recoverable after a partial failure. Verify current Git
  registration and filesystem ownership before removing anything. Preserve
  dirty, active or unowned checkouts and explain the refusal.
- Verify the ordered set of member identities and candidate revisions. A change
  in one member invalidates its evidence and the aggregate ship result. Any
  per-member failure remains visible in the group result.
- Exercise two members, two simultaneous workspaces, partial bootstrap,
  independent commits and a stale member candidate before dogfood acceptance.

## Open design

[#109](https://github.com/LennardZuendorf/vibe/issues/109) chooses the manifest
layout and primary feature ownership. Keep the
[single-member lifecycle](cli/cli-worktree.md) and
[Spec storage constraints](spec/spec-projects-mcp.md) as inputs. The
[dogfood issue](https://github.com/LennardZuendorf/vibe/issues/111) records proof.

[Milestones](../concepts/product-milestones.md) · [Product requirements](../concepts/milestone-feature-workspaces.md)
