---
type: entrypoint
scope: product
children:
  - features/spec-framework/product.md
  - features/code-flow/product.md
  - features/platform-adapters/product.md
updated: 2026-05-14
---

# shards-code — Product

shards-code is a personal agent workflow framework. It combines a reusable
file-based `spec` framework with a platform-neutral `code` flow harness, then
exposes that workflow to Codex, Claude Code, and future agent runtimes through
thin adapters.

**One-liner:** durable specs plus agent skills plus flow state, composed into a
strict personal coding workflow.

---

## Story

My coding workflow has two separate needs. First, decisions need durable memory:
what the project is, why it exists, how it is built, how it should feel, and what
work remains. Second, agents need a runtime harness: clear phases, skill routing,
workspace setup, and guardrails that work across Codex and Claude Code.

shards-code exists to combine those pieces without blurring them. The `spec`
framework owns planning memory in `.spec/`. The `code` flow owns agent execution
state and skill orchestration in `.agents/`. Platform files such as `AGENTS.md`,
`CLAUDE.md`, and `.claude/*` are adapters over that core, not the source of
truth.

The point is to take the planning load off myself. I should be able to say "I
need X" and have the flow guide the agent through feature spec, planning,
building, and TDD validation — encoding intent and instinct as constraints and
injected resources rather than relying on the agent (or me) to remember the
right next move. It borrows ideas from Compound Engineering — lessons that feed
back, stable plan IDs — while staying KISS and personal, not a second toolchain.

---

## Requirements

At a project level, shards-code must:

1. **Keep planning and runtime state separate.** Durable product, tech, design,
   plan, and lessons docs live in `.spec/`; mutable flow state lives outside
   `.spec/`.
2. **Make `spec` reusable on its own.** The `spec` skill must remain useful even
   without the `code` flow harness.
3. **Make `code` a set of first-class agent skills.** Workflow shims live under
   `.agents/skills/code-*` and delegate to other skills with explicit routing.
4. **Use platform-neutral flow state.** The canonical cursor and state machine
   live under `.agents/flow/`, not `.claude/` or Codex-specific paths.
5. **Treat Codex and Claude Code as adapters.** `AGENTS.md`, `CLAUDE.md`,
   Claude slash commands, and hooks read the same `.agents/flow` core.
6. **Inject output paths when delegating.** A `code-*` skill may call
   `superpowers:*`, `spec`, or subagents, but it must tell them exactly which
   `.spec/` paths to write.
7. **Degrade gracefully.** Missing skills, missing adapters, or corrupt flow
   state produce warnings and recovery paths, not session-ending failures.

---

## Design Principles

1. **Composition over reimplementation.** `code-*` skills route to existing
   skills instead of copying their workflows.
2. **Specs are memory, flow is runtime.** `.spec/` records durable thinking;
   `.agents/flow/` records the current agent state.
3. **Agent skills are the command surface.** The recurring workflow is expressed
   as skills agents can invoke, not as loose markdown snippets.
4. **Adapters stay thin.** Platform-specific files translate runtime events into
   `code-*` skill invocations and `.agents/flow` reads/writes.
5. **Canonical paths beat skill defaults.** Any delegated skill must write into
   the project’s `.spec/` layout, not its own default doc folder.
6. **Small shims, shared machinery.** State transitions and deterministic checks
   belong in `.agents/flow/scripts/`; `SKILL.md` files stay concise.

---

## Target User

Me: one developer shaping a portable personal coding workflow across agent
runtimes. The system should be forkable, but decisions optimize for my working
style rather than a broad marketplace audience.

---

## Product Pieces

| Piece | What It Owns | Feature Spec |
|---|---|---|
| `spec` framework | `.spec/` docs, templates, validation, archive rules | [features/spec-framework/](features/spec-framework/product.md) |
| `code` flow | `.agents/flow` state, `code-*` skills, phase routing | [features/code-flow/](features/code-flow/product.md) |
| Platform adapters | `AGENTS.md`, `CLAUDE.md`, Claude commands/hooks, install/setup glue | [features/platform-adapters/](features/platform-adapters/product.md) |

---

## Workflow Surface

The primary user-facing workflow is a family of agent skills:

| Skill | When | Main Output |
|---|---|---|
| `code-setup` | Installing or repairing the workflow harness in a project | `.agents/flow`, adapter files, baseline `.spec/` |
| `code-strategy` | Bootstrapping or refocusing project direction | Root `.spec/{product,tech,design,plan}.md` |
| `code-feature` | Designing and building a named feature | `.spec/features/<name>/` plus implementation |
| `code-quick` | Small fixes and bounded maintenance | Workspace edits, optional `.spec/quick/<slug>.md` |
| `code-verify` | Evidence before completion | Test/build/review findings |
| `code-compound` | End-of-work consolidation | Lessons, root spec updates, archive moves |
| `code-amend` | Revising active scope | Updated feature or strategy specs |

These are skills, not hidden prompts. Adapters may expose shortcuts, but the
canonical workflow units are `.agents/skills/code-*`.

## Communication Levels

The `code` flow requests a "caveman" communication-density level per state. The
level names (`lite`/`full`/`ultra`) follow the upstream `JuliusBrussee/caveman`
skill; the mapping of *level to workflow phase* is shards-code's own policy.
Caveman is **output compression only** — it never reduces reasoning depth, and
code, paths, and commands stay byte-exact.

| Level | Behaviour | Use When |
|---|---|---|
| `lite` | No filler or hedging; keep full sentences. | Strategy, setup, design, compound, amend — where nuance matters. |
| `full` | Drop articles; fragments OK; short synonyms. The working default. | Implementation, verification, and quick fixes/triage. |
| `ultra` | Abbreviate prose; `X → Y` arrows; one word where one word does. | Compound receipts and high-volume subagent→orchestrator summaries. |

`ultra` is *not* used for triage: it can drop edge cases, and triage is where a
missed edge case is expensive. Regardless of level, security warnings and
irreversible-action confirmations stay in normal prose.

State-machine entries name the expected level, and a single inject owner emits it
so adapters and subagents stay consistent (see features/code-flow).

---

## Non-Goals

- **Not a replacement for `spec`.** The `code` flow uses the spec framework; it
  does not absorb it.
- **Not Claude-only.** Claude Code integration is an adapter, not the core.
- **Not Codex-only.** Codex reads `AGENTS.md`, but the flow state remains under
  `.agents/flow`.
- **Not a new implementation framework.** The repo is markdown, bash scripts,
  and agent skills.
- **Not strict by accident.** Hard blocks must protect real invariants and stay
  understandable.

---

## Open Questions

1. **Git tracking for `.agents/flow/state.json`.** Default recommendation:
   version the static state machine in this repo, gitignore mutable cursor files
   in target projects.
2. **Exact `code-*` skill count.** Start with strategy, feature, quick, verify,
   compound, amend; merge if dogfooding shows extra ceremony.
3. **Adapter installation.** Decide whether `install.sh` creates symlinks,
   copies files, or offers diffs for each platform adapter.
4. **Hook strictness.** Start with warnings and narrow hard blocks; expand only
   after observed failures.

---

## Features

| Feature | Covers |
|---|---|
| **[features/spec-framework/](features/spec-framework/product.md)** | Durable `.spec/` planning model: product, tech, design, plan, lessons, feature folders, validation. |
| **[features/code-flow/](features/code-flow/product.md)** | Agent skill shims, `.agents/flow` state, state machine, phase routing, delegated skill output paths. |
| **[features/platform-adapters/](features/platform-adapters/product.md)** | Codex and Claude Code integration files that expose the same code flow core. |
