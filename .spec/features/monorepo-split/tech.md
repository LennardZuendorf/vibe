---
type: feature-tech
feature: monorepo-split
sibling: product.md
parent: ../../tech.md
updated: 2026-07-03
---

# Feature: Monorepo Split — Architecture

`git mv .agents/skills/spec spec` and `git mv .agents/skills/vibe flow`, then
recreate the old paths as relative symlinks. All runtime references stay
`.agents/skills/*`; only storage moves. Installer dereferences symlinks so
targets get real dirs. One sweep pass fixes all stale spec references, then
the three orphan features are compounded.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## Files

```
spec/                                  # moved from .agents/skills/spec (git mv)
flow/                                  # moved from .agents/skills/vibe (git mv)
.agents/skills/spec -> ../../spec      # relative symlink, committed
.agents/skills/vibe -> ../../flow      # relative symlink, committed
.claude/skills/spec -> ../../spec      # repointed (was ../../.agents/skills/spec)
.gitignore                             # cursor line: flow/state.json (old path dies with symlink)
install.sh                             # copy source = spec/ + flow/; cp with dereference into target .agents/skills/
.claude-plugin/plugin.json             # homepage -> LennardZuendorf/vibe
flow/reference/templates/AGENTS.md     # dedupe repo-layout line; paths stay .agents/skills/* (portable)
AGENTS.md                              # regenerated via merge-agents.sh
.spec/{product,tech,plan}.md           # sweep + compound writes
.spec/features/{vibe-flow-collapse,vibe-skill-consolidation,spec-skill-improvements}/  # compound -> archive -> delete
tests/{flow,adapters}/run.sh           # new-layout assertions
```

---

## Implementation Detail

**Symlink direction.** Canonical content at root; `.agents/skills/*` are the
compat/interface links. Rationale: GitHub browsability is the product goal;
runtime interface stays byte-identical for AGENTS.md template portability
(installed targets have real dirs at `.agents/skills/*`, source repo has
symlinks — same reference works in both).

**Script self-location audit (critical).** Any script resolving its own dir
via `pwd -P` / `realpath` / `readlink -f` lands in `flow/scripts/` when
invoked through the symlink, so `../..`-style hops break (repo root vs
`.agents/skills`). Audit all seven `flow/scripts/*.sh` + hooks + `validate.sh`
for physical-path resolution; normalize to logical resolution or root
discovery (`git rev-parse --show-toplevel` fallback upward-search for
`state-machine.json`). Test both invocation paths.

**Installer copy semantics.** Source of truth for copies becomes `spec/` and
`flow/`; write into target `.agents/skills/{spec,vibe}` with `cp -RL`
(dereference). Cursor snapshot/restore logic unchanged (regression test
exists). Target never receives symlinks.

**Gitignore.** `.agents/skills/vibe/state.json` no longer matches once the
dir is a symlink; replace with `flow/state.json`. Runtime writes through
`.agents/skills/vibe/state.json` land in `flow/state.json` — same inode.

**Sweep inventory** (from recon, counts approximate):
`.spec/product.md` (13× `.agents/flow`), `.spec/tech.md` (18× + stale layout
tree + mermaid), `.spec/plan.md` (6× + test counts 44/26/39 → real +
Feature Sequence), `.spec/features/platform-adapters/*` (many),
`.spec/features/agent-instructions/tech.md:176`, `spec/reference/plan.md:225`.
Intentional keeper: `tests/adapters/run.sh:153` (asserts absence).

**Compound of orphans.**
- `vibe-flow-collapse`: plan all-TODO but shipped — mark done with evidence, promote nothing (collapse is layout history), DONE row, archive, delete.
- `vibe-skill-consolidation`: perform deferred root-plan OPEN-2 note update, DONE row, archive, delete.
- `spec-skill-improvements`: already in-repo truth (skill v2.0); handle nonstandard `impl.md` (fold anything durable into tech.md before archive), DONE row, archive, delete.
- First real use of `.spec/archive/` → then prompt-delete per lifecycle.

## Open Questions

1. **Windows checkouts** — committed symlinks need `core.symlinks=true`; document as known limitation in release-docs README, no mitigation here.
