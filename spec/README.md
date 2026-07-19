# The spec framework

> **For humans.** This README is the standalone guide to the spec half of vibe kit.
> Agents read [SKILL.md](SKILL.md) instead — it is the router they route through.

The **spec framework** is a durable planning layer for a codebase. It teaches an
agent (or a human) to keep design docs in `.spec/` that are *current*, not a
backlog: what you are building (product), how it is built (tech), the visual /
interaction language (design), and the order of work (plan). It ships as one
bundled skill, `spec`, plus a validator. It needs only `bash` — no runtime, no
build step — and works with **any** agent, or none.

It is one of two halves. The other is [the vibe flow](../flow/README.md); the
[root README](../README.md) explains the split. This half stands entirely alone.

## Quickstart

```bash
/spec setup            # write .spec/ entrypoints from templates (+ .config.yaml)
/spec strategy         # author the root layer: product / tech / design / plan
/spec feature <name>   # scope and design one named feature
/spec validate         # check structural consistency
```

`/spec …` are Claude Code skill commands. Everywhere else, run the scripts
directly **from your project root** — they resolve `.spec/` relative to the
current directory, so cwd must be the project you want to act on:

```bash
bash .agents/skills/spec/scripts/setup.sh      # create .spec/ entrypoints from templates
bash .agents/skills/spec/scripts/validate.sh   # check consistency (run from project root)
bash ~/.agents/skills/spec/scripts/validate.sh # global install
```

## The two-layer model

```
.spec/
├── product.md, tech.md, design.md, plan.md, lessons.md   ← ROOT (persistent, high-level)
├── product-{topic}.md, tech-{topic}.md                    ← ROOT branch docs (cross-cutting, rare)
├── features/<name>/
│   ├── product.md   required     what this feature does (requirements + Scope)
│   ├── tech.md      required     how it is built (paths, contracts, layout)
│   ├── plan.md      recommended  stable <name>/n unit IDs; verification per unit
│   ├── design.md    optional     UI/UX or interaction detail
│   └── research.md  optional     discovery artifacts
└── archive/<name>/  transient    post-wrapup safety net (deleted before merge)
```

- **Root** answers project-level questions and carries no backlog or archaeology.
- **Features** are branch-scoped: written at design, consumed at impl, merged
  (cross-cutting parts only) into root at compound, archived transiently, then
  **deleted before the branch merges**. Code is truth; the archive is never read
  for active work.
- **Branch docs** cover only concerns that span *every* feature. Default to a
  feature folder; extract a branch doc when the same concern keeps recurring.

## Day-to-day authoring

**Bootstrap (strategy).** Write the root layer in order:
`product.md → tech.md → design.md → plan.md`. The root plan holds the feature map
and a Feature Sequence with binary whole-feature gates — never unit-level detail.

**A new feature** follows a six-step ladder (full steps in [feature.md](feature.md)):

```
1. Locate & name    confirm name; read root product/tech + lessons.md
2. Interview WHAT    Scope, SHALL/MUST requirements, GWT scenarios → product.md
3. Rigor gate        lite vs full — does it need design.md?
4. Sketch HOW        trace the codebase → tech.md (+ design.md if full)
5. Plan units        stable <name>/n IDs, verification per unit → plan.md
6. Skip check        atomic, no decisions? → use the flow's quick arc instead
```

Add the feature to the root `plan.md` Feature Sequence. Cite unit IDs in commits
and tests during implementation. Bump each edited spec's `updated:` date, then run
`/spec validate`.

**Strict separation** is the core discipline: product holds *what & why* (no code
or paths), tech holds *how* (no UX opinions), plan holds *units & order* (no code
or requirement prose), design holds tokens and interaction patterns. Read before
you write; keep parent ↔ child links alive both ways.

**Superpowers, offered not required.** Each authoring step names an executor the
skill offers proactively — `superpowers:brainstorming` for the WHAT interview,
`code-explorer` + `code-architect` for the HOW trace, `superpowers:writing-plans`
for plan units. Decline any and the skill self-executes from its own constraint
documents. They enhance; they never gate.

## With or without the flow

The spec skill is self-sufficient. Drive it directly with `/spec …` on any host,
or let [the vibe flow](../flow/README.md) drive its authoring phases
(`strategy.spec`, `feature.design`, `feature.plan`, the compound promotions). When
the flow is present, `.spec/.config.yaml` can set `vibe-flow: true` so output
density follows the flow's single `style` note; absent that, the skill uses its own
documented style. Either way the docs, templates, and validator are identical.

## Degrade behavior

- **Bare git / any editor:** the whole framework works — `.spec/` docs, the
  templates in [reference/templates/](reference/templates/), and `validate.sh`
  (pure `bash`, no `jq` required). You lose only the `/spec` command sugar.
- **Other `AGENTS.md` readers:** agents follow the written authoring flow manually.
- **Missing superpowers:** every step self-executes from its constraint document.

Nothing here hard-fails on a missing dependency.

## File map

Everything below is the `spec` skill, addressed at runtime under
`.agents/skills/spec/`.

| Path | What it is |
|---|---|
| [SKILL.md](SKILL.md) | Skill router — routing table, two-layer rules, roles |
| [strategy.md](strategy.md) | Root-layer authoring guide |
| [feature.md](feature.md) | Feature-layer authoring flow (the six-step ladder) |
| [reference/product.md](reference/product.md), [reference/tech.md](reference/tech.md), [reference/design.md](reference/design.md), [reference/plan.md](reference/plan.md) | Per-doc writing guides |
| [reference/templates/](reference/templates/) | Copy-in templates for every root, feature, and branch doc |
| [scripts/validate.sh](scripts/validate.sh) | Structural consistency check (frontmatter, links, folders) |
| [scripts/setup.sh](scripts/setup.sh) | Create entrypoint templates in `.spec/` |
| [scripts/list-specs.sh](scripts/list-specs.sh) | Current-state summary of the `.spec/` tree |
| [scripts/promote.sh](scripts/promote.sh), [scripts/scan-merges.sh](scripts/scan-merges.sh) | Compound: merge feature blocks into root, preview pending merges |
| [scripts/lessons-for.sh](scripts/lessons-for.sh) | Extract lessons matching a tag |
| [agents/](agents/) | Optional subagents: `spec-tracer`, `spec-interviewer`, `spec-promoter`, `spec-health` |

## More

- [`../README.md`](../README.md) — the umbrella: the spec/flow split and install.
- [`../flow/README.md`](../flow/README.md) — the other half: the state-machine flow.
- [SKILL.md](SKILL.md) — the canonical rules agents follow.
