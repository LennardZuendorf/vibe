---
type: feature-tech
feature: release-docs
sibling: product.md
parent: ../../tech.md
updated: 2026-07-03
---

# Feature: Release Docs — Architecture

Docs and rails are files; the only moving parts are CI, the combined runner,
and the stranger eval harness. Everything derives from post-split layout and
install-tooling outputs (deps.json renders the dependency table; doctor
features in the trust story).

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Design:** [design.md](design.md)
**Plan:** [plan.md](plan.md)

---

## Files

```
README.md                       # umbrella rewrite (banner, split, quickstarts, honesty tables)
spec/README.md                  # refresh existing standalone quickstart
flow/README.md                  # new — harness deep-dive (states, orders, hooks, deps)
LICENSE                         # MIT, Lennard Zündorf, 2026
CHANGELOG.md                    # keep-a-changelog, 0.1.0
tests/run.sh                    # runs spec+flow+adapters suites, aggregate exit
.github/workflows/ci.yml        # ubuntu: shellcheck **/*.sh, tests/run.sh, spec validate
.github/ISSUE_TEMPLATE/{bug.yml,feature.yml}
docs/img/logo.svg               # picked candidate (banner)
docs/img/candidates/logo-{1..n}.svg
docs/evals/stranger-2026-07-03.md
examples/todo-api/.spec/...     # worked sample: root specs + one feature folder
```

---

## Implementation Detail

**CI.** Single workflow, one ubuntu job: `shellcheck` (gate — repo convention
demands clean), `bash tests/run.sh`, `bash .agents/skills/spec/scripts/validate.sh`.
No matrix, no caching cleverness — total runtime well under a minute.

**Dependency table.** Rendered from `flow/reference/deps.json` (install-tooling
R5) via a tiny jq block in the docs unit — never hand-written twice.

**Platform honesty table.** Rows: Claude Code (full: plugin+hooks+skills),
Codex/other AGENTS.md-readers (specs + instructions, no hooks), bare git
(spec framework only). Columns: what works / what degrades / what's absent.

**Stranger eval harness.** Scripted sandbox (`mktemp -d`, `git init`, copy
nothing) + a fresh subagent whose prompt contains ONLY: "You cloned
github.com/LennardZuendorf/vibe at <path>. Using nothing but its README
files, install it into <sandbox> (first spec-only, then full) and run one
quick.fix arc." Agent reports each command tried + result + friction score.
Report template: finding / severity / fix-or-accept. Findings loop back into
docs units before PR.

**gh metadata.** `gh repo edit --description ... --add-topic` (claude-code,
agents, spec-driven, workflow, ai-tooling…). Graceful skip when `gh` unauthed.

## Open Questions

1. **Badge set** — CI + license + "tests: 190+" static badge; skip coverage
   (bash, no meaningful tool). Decide at README unit.
