# The spec framework

> **For humans.** This README is the standalone guide to the spec half of vibe kit.
> Agents read [SKILL.md](SKILL.md) instead — it is the router they route through.

The **spec framework** is a durable planning layer for a codebase. It teaches an
agent (or a human) to keep design docs that are *current*, not a backlog: what you
are building (product), how it is built (tech), the visual / interaction language
(design), and the order of work (plan). It ships as the `spec` skill, the
`using-spec` session router, two agents, and hooks, all backed by the `vibe-spec`
binary. The binary validates and queries the spec tree. It works with **any** agent,
or none.

It is one of two halves. The other is [the vibe flow](https://github.com/LennardZuendorf/vibe/blob/main/flow/README.md); the
[root README](https://github.com/LennardZuendorf/vibe#readme) explains the split and
holds the [Install section](https://github.com/LennardZuendorf/vibe#install). This half stands entirely alone.

## Quickstart

```bash
/spec setup            # write the config and seed a spec root (vibe-spec init)
/spec strategy         # author the root layer: product / tech / design / plan
/spec feature <name>   # route and scope one named feature
/spec validate         # check structural consistency
```

`/spec …` are Claude Code skill commands. Everywhere else, run `vibe-spec`
directly from anywhere inside the project; it walks up to the git root to find the
spec root:

```bash
vibe-spec root                # print the spec root and its layout
vibe-spec init                # seed a layered spec root
vibe-spec validate            # check consistency
vibe-spec validate --strict   # promote every warning to an error (CI)
vibe-spec check-drift         # compare the plan rows with the feature folders
```

## The two-layer model

The **spec root** is the directory `vibe-spec root` prints (`docs/spec/` by default).
It has two layouts. `vibe-spec init` creates the **layered** one.

```
<root>/
├── product.md, tech.md, design.md, plan.md   ← hubs: short, link only
├── lessons.md                                 ← tagged lessons, append-only
├── areas/ flows/ concepts/ platform/ tech/    ← typed notes the hubs link to
│   decisions/ questions/ parts/<part>/
└── features/<feature>/   fallback feature layer only; see below
```

- **Root** answers project-level questions and carries no backlog or archaeology.
  The layered guide is `reference/layered.md`.
- **Feature layer**, branch-scoped: by default the superpowers artifacts under
  `docs/superpowers/specs/` and `docs/superpowers/plans/`. The fallback folder
  `<root>/features/<feature>/` applies only when superpowers is absent, your human
  partner says *self*, `.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec
  interchange is required. Wrap-up promotes the cross-cutting parts to the root and
  deletes the feature artifacts **before the branch merges**. Code is truth.
- **Flat (legacy)** roots keep `product.md`, `tech.md`, `design.md`, `plan.md`,
  `lessons.md` and `{product,tech}-{topic}.md` topic docs side by side. The
  templates in [reference/templates/](reference/templates/) are flat-layout only;
  the `scripts/` validator reads a flat legacy root only.

## Day-to-day authoring

**Bootstrap (strategy).** Write the root layer in order:
`product.md → tech.md → design.md → plan.md`. The root plan holds the feature map
and a Feature Sequence with binary whole-feature gates — never unit-level detail.
The row for a fallback feature exists, with status `IN PROGRESS`, before its design.

**A new feature** routes through [SKILL.md](SKILL.md) § Routing. The default is
`superpowers:brainstorming` then `superpowers:writing-plans`. The fallback follows
[feature.md](feature.md): step 0 sizes the work, then Locate and name, Interview for
WHAT, Rigor gate, Sketch HOW, Plan units. Cite unit IDs in commits and tests during
implementation. Bump each edited spec's `updated:` date, then run `/spec validate`.

**Wrap-up** has one sequence: [SKILL.md](SKILL.md) § Wrap-up.

**Strict separation** is the core discipline: product holds *what & why* (no code
or paths), tech holds *how* (no UX opinions), plan holds *units & order* (no code
or requirement prose), design holds tokens and interaction patterns. Read before
you write; keep parent ↔ child links alive both ways.

**Superpowers are optional.** Each authoring step names a delegate the skill
suggests unless `.vibe/spec.json` sets `suggest` to false. Decline any and the skill
does the step itself from its constraint documents. They enhance; they never gate.

## With or without the flow

The spec skill is self-sufficient. Drive it directly with `/spec …` on any host,
or let [the vibe flow](https://github.com/LennardZuendorf/vibe/blob/main/flow/README.md) drive its authoring states
(`strategy.spec`, `feature.design`, `feature.plan`, `feature.compound`). Either way
the docs, templates, and validator are identical.

## Degrade behavior

- **No `vibe-spec` binary:** the skill says so and points to the install section.
  On a flat legacy root it may run the legacy validator in `scripts/`; on any other root
  it reports "validation unavailable" and never claims the tree validated.
- **Other `AGENTS.md` readers:** agents follow the written authoring flow manually.
- **Missing superpowers:** the fallback feature layer applies.

## File map

Everything below is the `spec` skill, addressed at runtime under
`.agents/skills/spec/` (and `.agents/skills/using-spec/`).

| Path | What it is |
|---|---|
| [SKILL.md](SKILL.md) | Skill router: routing table, layers, wrap-up, roles |
| [using-spec/](using-spec/SKILL.md) | Session-start router that sends work to `spec` |
| [strategy.md](strategy.md) | Root-layer authoring guide |
| [feature.md](feature.md) | Fallback feature-layer authoring flow |
| [reference/](reference/) | Per-doc writing guides (`product`, `tech`, `design`, `plan`) and [templates/](reference/templates/) |
| [agents/](agents/) | Claude Code agents: `spec-promoter`, `spec-health` |
| [hooks/](hooks/) | Fail-open shim that runs `vibe-spec hook <event>` |
| `scripts/` | Bash scripts, legacy (flat root only) |

## More

- [the root README](https://github.com/LennardZuendorf/vibe#readme) — the umbrella: the spec/flow split and install.
- [`flow/README.md`](https://github.com/LennardZuendorf/vibe/blob/main/flow/README.md) — the other half: the state-machine flow.
- [SKILL.md](SKILL.md) — the canonical rules agents follow.
