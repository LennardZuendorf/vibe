---
name: vibe
description: |
  Vibe workflow router — one skill for the whole flow. Routes to the right phase
  for the current state: setup, strategy, feature, quick, verify, compound, amend.
  Trigger on: any vibe flow state, "what should I do next", flow navigation, set up
  vibe, project direction/strategy, build a feature, quick fix, verify/evidence,
  compound/wrap up, amend scope, or user says vibe.
user-invocable: true
argument-hint: "[setup|strategy|feature|quick|verify|compound|amend]"
allowed-tools: Read, Edit, Write, Bash
compatibility: Requires bash + jq. macOS and Linux.
metadata:
  author: lennarddib
  version: "1.0"
---

# vibe — workflow router

One skill for the whole vibe flow. This lean entry carries the routing table and
the machine-extractable per-turn orders; each phase file holds the full procedure.
Read the phase file for your current (or target) state, then follow it.

## Routing

| Argument | Phase file | States |
|---|---|---|
| `setup` | [setup.md](setup.md) | `setup.detect`, `setup.apply` |
| `strategy` | [strategy.md](strategy.md) | `strategy.brainstorm`, `strategy.spec` |
| `feature` | [feature.md](feature.md) | `feature.design`, `feature.plan`, `feature.impl` |
| `quick` | [quick.md](quick.md) | `quick.triage`, `quick.fix` |
| `verify` | [verify.md](verify.md) | `feature.verify`, `quick.verify` |
| `compound` | [compound.md](compound.md) | `strategy.compound`, `feature.compound` |
| `amend` | [amend.md](amend.md) | `amend` (modifier) |

`scripts/` and `reference/` back the setup phase: `merge-agents.sh`, the `AGENTS.md`
template, and `adapters.json`.

## Orders (D12)

Machine-extractable per-state orders. The `UserPromptSubmit` inject hook resolves
the current `<flow>.<phase>`, follows its `skill` link (now always `vibe`), and emits
the matching block verbatim via `.agents/skills/vibe/scripts/orders.sh`. `<feature>` is the
only interpolation; keep each block byte-stable.

<!-- vibe:orders:setup.detect -->
skill=vibe · READ-ONLY audit of repo + harness · report what is missing/present (AGENTS.md health, adapter rows from adapters.json, .agents/skills/vibe scripts, vibe skill, bundled spec) · preflight required plugins · do NOT write yet · caveman=lite · next: setup.apply
<!-- /vibe:orders -->

<!-- vibe:orders:setup.apply -->
skill=vibe · WRITE/MERGE bootstrap: merge-agents.sh AGENTS.md instructions block, .agents/skills/vibe scaffold, baseline .spec/**, optional adapter symlinks (user-driven) · NEVER clobber existing content (diff + ask on divergence) · regen-active-rules.sh after merge · caveman=lite · next: idle
<!-- /vibe:orders -->

<!-- vibe:orders:strategy.brainstorm -->
skill=vibe · delegate superpowers:brainstorming · READ .spec/lessons.md first · scratch only, no source, no spec writes yet · caveman=lite · next: strategy.spec
<!-- /vibe:orders -->

<!-- vibe:orders:strategy.spec -->
skill=vibe · delegate spec · WRITE root .spec/{product,tech,design,plan}.md ONLY · no source, no lessons.md · validate after · caveman=lite · next: strategy.compound (if a durable lesson surfaced) | idle
<!-- /vibe:orders -->

<!-- vibe:orders:strategy.compound -->
skill=vibe · WRITE .spec/lessons.md (tagged entry) · regen-active-rules.sh refreshes CLAUDE/AGENTS active-rules block · receipts caveman=ultra, body=lite · next: idle
<!-- /vibe:orders -->

<!-- vibe:orders:feature.design -->
skill=vibe · spec feature.md flow steps 1–4 (locate→interview WHAT→rigor gate→sketch HOW) · READ lessons.md + root product/tech · WRITE .spec/features/<feature>/{product,tech,design?}.md ONLY · no source · caveman=lite · next: feature.plan
<!-- /vibe:orders -->

<!-- vibe:orders:feature.plan -->
skill=vibe · spec feature.md step 5 (plan units) · WRITE .spec/features/<feature>/plan.md · STABLE unit IDs (<feature>/n) cite R-IDs · verification per unit · no source · caveman=lite · HUMAN GATE before impl · next: feature.impl
<!-- /vibe:orders -->

<!-- vibe:orders:feature.impl -->
skill=vibe · delegate executing-plans + TDD · WRITE src/**, tests/** · do NOT edit .spec/** · cite plan unit IDs (<feature>/n) in tests/commits · caveman=full · next: feature.verify
<!-- /vibe:orders -->

<!-- vibe:orders:feature.verify -->
skill=vibe · delegate verification-before-completion + requesting-code-review + code-reviewer (systematic-debugging on fail) · gather EVIDENCE per plan unit ID · no spec writes · caveman=full · HUMAN GATE before ship · next: feature.compound (pass) | feature.impl (targeted fix) | feature.plan (major drift)
<!-- /vibe:orders -->

<!-- vibe:orders:feature.compound -->
skill=vibe · delegate finishing-a-development-branch · WRITE tagged .spec/lessons.md, promote cross-cutting decisions to root specs, archive .spec/features/<feature> -> .spec/archive/<feature> · regen-active-rules.sh refreshes digest · prompt to delete archive after validation · receipts caveman=ultra, body=lite · next: idle
<!-- /vibe:orders -->

<!-- vibe:orders:quick.triage -->
skill=vibe · READ .spec/lessons.md first · delegate superpowers:systematic-debugging · diagnose, do NOT fix yet · caveman=full (NOT ultra: triage must keep edge cases) · if scope balloons, escalate to feature.design · next: quick.fix
<!-- /vibe:orders -->

<!-- vibe:orders:quick.fix -->
skill=vibe · delegate TDD · WRITE src/** (+ optional .spec/quick/<slug>.md note) · no root spec writes · caveman=full · next: quick.verify
<!-- /vibe:orders -->

<!-- vibe:orders:quick.verify -->
skill=vibe · delegate verification-before-completion + code-reviewer · gather EVIDENCE the fix works and breaks nothing · no spec writes · caveman=full · next: idle
<!-- /vibe:orders -->

<!-- vibe:orders:amend -->
MODIFIER=amend · edit scope for the CURRENT state, then RETURN to it · carry the target state's write rules (do NOT widen them) · caveman=lite · next: <state you came from>
<!-- /vibe:orders -->
