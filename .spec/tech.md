---
type: entrypoint
scope: technical
children:
  - features/vibe-flow/tech.md
  - features/platform-adapters/tech.md
updated: 2026-06-06
---

# vibe — Technical Architecture

Project-level architecture for the combined `spec` framework and `vibe` flow
harness. Feature-level implementation detail lives under `.spec/features/<name>/`.

---

## Design Philosophy

1. **File-based contracts.** Specs, flow state, skills, and adapter instructions
   are ordinary files that agents can inspect and tools can validate.
2. **Separation of durability.** `.spec/` is durable project memory;
   `.agents/flow/` is runtime workflow state.
3. **Skills as orchestration units.** `vibe-*` skills are first-class agent
   skills with `SKILL.md` frontmatter, concise instructions, and optional
   scripts/references.
4. **Platform-neutral core.** Claude Code and Codex files are adapters that read
   `.agents/flow` and invoke `.agents/skills/vibe-*`.
5. **Delegation with constraints.** `vibe-*` skills call `spec`,
   `superpowers:*`, and subagents with explicit path instructions.
6. **Scripts for deterministic machinery.** State reads/writes, validation, and
   adapter installation use bash scripts rather than repeated prose.

---

## Architecture Overview

```mermaid
flowchart TD
  U["User intent"] --> A["Adapter: Codex / Claude Code"]
  A --> K["vibe-* agent skill"]
  K --> F[".agents/flow/state.json"]
  K --> M[".agents/flow/state-machine.json"]
  K --> D["Delegated skills with injected paths"]
  D --> S["spec skill"]
  D --> P["superpowers:*"]
  D --> G["subagents / review agents"]
  S --> SPEC[".spec/**"]
  P --> SPEC
  G --> SRC["workspace edits"]
  K --> V["verification / compound"]
```

---

## Layers

| Layer | Files | Role |
|---|---|---|
| Spec framework | `.agents/skills/spec/`, `.spec/**` | Durable project planning and validation. |
| Vibe flow core | `.agents/flow/**` | Platform-neutral flow state, state machine, transition scripts. |
| Vibe skills | `.agents/skills/vibe-*/SKILL.md` | Agent-facing workflow shims that delegate to real skills. |
| Platform adapters | `AGENTS.md`, `CLAUDE.md`, `.claude/**`, `.claude-plugin/` | Runtime-specific integration over the same core, incl. the Claude Code plugin + hooks. |

---

## File Layout

```text
vibe/
├── AGENTS.md
├── CLAUDE.md
├── README.md
├── install.sh
├── .agents/
│   ├── flow/
│   │   ├── state-machine.json          # static flow definition
│   │   ├── state.example.json          # neutral cursor template
│   │   └── scripts/
│   │       ├── detect-context.sh       # read flow + repo state, emit JSON / decide
│   │       ├── set-state.sh            # validated state writer
│   │       ├── validate-state.sh       # state-machine consistency checks
│   │       └── regen-active-rules.sh   # project lessons digest into adapter blocks
│   └── skills/
│       ├── spec/
│       ├── vibe-strategy/
│       ├── vibe-feature/
│       ├── vibe-quick/
│       ├── vibe-verify/
│       ├── vibe-compound/
│       └── vibe-amend/
├── .claude-plugin/
│   └── plugin.json                     # Claude Code plugin manifest (bundles cmd+skills+hooks)
├── .claude/
│   ├── commands/flow.md                # Claude adapter, reads .agents/flow
│   └── hooks/
│       ├── hooks.json                  # event → script wiring
│       ├── user-prompt-submit-inject.sh  # inject linked skill's orders each turn
│       ├── pre-tool-use-guard.sh         # allow/warn/block via detect-context.sh
│       └── stop-gate.sh                  # end-of-turn exit-predicate checks
└── .spec/
    ├── product.md
    ├── tech.md
    ├── design.md
    ├── plan.md
    ├── lessons.md
    ├── features/<name>/
    └── archive/<name>/
```

Target projects receive the same `.agents/skills` and `.agents/flow` core, plus
adapter files for the agent runtimes they use.

---

## Spec Framework Contract

The spec framework owns only durable planning artifacts. It does **not** own flow
state, agent instruction files, or platform hooks (see feature boundaries in root
[plan.md](plan.md)).

```text
.spec/
├── product.md
├── tech.md
├── design.md
├── plan.md
├── lessons.md
├── product-<topic>.md
├── tech-<topic>.md
├── plan-<topic>.md
├── features/<feature>/     # ephemeral; archive after compound
│   ├── product.md          # required — WHAT: Requirement+Scenario format
│   ├── tech.md             # required — HOW: files, contracts
│   ├── design.md           # optional (full-rigor / UI)
│   ├── plan.md             # recommended — ### U1. units, Requirements Trace
│   └── research.md         # optional
└── archive/<feature>/      # post-merge history
```

No mutable cursor, phase file, turn counter, hook cache, or runtime lock belongs
under `.spec/`.

### Bundled skill layout

```text
.agents/skills/spec/
├── SKILL.md
├── strategy.md
├── feature.md              # 6-step feature authoring interview flow (SF16)
├── scripts/
│   ├── setup.sh            # bootstrap root entrypoints + lessons.md (**Tags:**)
│   ├── validate.sh         # warn-first structural checks (SF8–SF12)
│   └── list-specs.sh       # root + feature doc inventory
├── reference/
│   ├── product.md, tech.md, plan.md, design.md
│   └── templates/          # hard-floor root + feature templates (SF5–SF7)
└── tests/spec/run.sh       # behaviour tests (17 cases)
```

`setup.sh` resolves templates relative to its script directory (vendored or
`~/.agents/skills/spec`).

### Validation

- Root entrypoints: `product.md`, `tech.md`, `design.md`, `plan.md`.
- Feature folders: require `product.md` + `tech.md`; optional docs need frontmatter.
- **Warn-first** structural checks (SF8–SF12): Scope table, frontmatter, Requirement+
  Scenario (RFC-2119 + Given/When/Then), plan units, ID traceability. Promote to
  errors after live specs migrate.
- **Design tokens (SF3):** local empty-group check (offline floor).
- **External linter (SF4, OPEN-5):** opt-in `VIBE_DESIGN_LINT=1` →
  `npx @google/design.md lint`; skips when offline or unset.
- **Lessons (D8):** format owned here (`**Tags:**` per entry); vibe-flow owns
  read-on-entry; vibe-compound writes + `regen-active-rules.sh` digest.

Feature specs are ephemeral: design → plan → impl → verify → compound →
`archive/<feature>/`. Cross-cutting decisions promote into root specs; feature-only
detail stays in archive. Promotable tech blocks use `<!-- merge -->` markers.

---

## Vibe Flow Contract

The flow state lives under `.agents/flow`. States are compound `<flow>.<phase>`
keys; the cursor carries only the moving parts and no turn-varying fields:

```json
{
  "flow": "idle | setup | strategy | feature | quick",
  "phase": "idle | detect | apply | brainstorm | spec | design | plan | impl | verify | compound | triage | fix",
  "feature": null,
  "updated": "2026-06-02T00:00:00Z"
}
```

`state-machine.json` defines each `<flow>.<phase>` state with its required `vibe-*`
skill, caveman level, allowed write surfaces, and exit predicate. The `skill` field
**links** the state to its owning shim; under D12 the per-turn orders are sourced
from that linked skill rather than a hand-written `inject` string. A single inject
owner (the `UserPromptSubmit` hook) pulls the current state's orders from its
linked skill and injects them once per turn (which also sets the caveman level),
keeping the inject byte-stable and prompt-cache-safe. `set-state.sh` is the only
sanctioned writer. Full per-state mapping lives in
[features/vibe-flow/tech.md](features/vibe-flow/tech.md).

---

## Code Skill Contract

Each `vibe-*` skill is a normal agent skill:

```text
.agents/skills/vibe-strategy/
└── SKILL.md
```

The body must stay small and procedural:

1. Read `.agents/flow/state.json` and relevant `.spec/` entrypoints.
2. Confirm the current phase or transition through `.agents/flow/scripts/set-state.sh`.
3. Delegate to the correct external skill with explicit output paths.
4. Validate the expected files or verification evidence.
5. Report the next legal transition.

Example delegation:

```text
Use superpowers:brainstorming to clarify strategy. Then use the spec skill to
write only .spec/product.md, .spec/tech.md, .spec/design.md, and .spec/plan.md.
Do not use the delegated skill's default documentation path.
```

---

## Adapter Contract

Adapters never own canonical state. They read `.agents/flow` and invoke
`.agents/skills/vibe-*`.

| Adapter | Owns | Does Not Own |
|---|---|---|
| Codex | `AGENTS.md` instructions, optional desktop/thread affordances | Flow state, spec layout |
| Claude Code | `CLAUDE.md`, `.claude-plugin/plugin.json`, `.claude/commands/*`, `.claude/hooks/*` | Canonical skills, state machine, the allow/warn/block policy (that lives in `detect-context.sh`) |
| Installer | Copy/symlink adapter files and core `.agents` files; register the Claude Code plugin | Project-specific product decisions |

### Claude Code plugin & hooks

The Claude Code adapter is packaged as an installable **plugin**
(`.claude-plugin/plugin.json`) that bundles the `/flow` command, the `vibe-*`
skills, and the flow **hooks**. The hooks are the Stage 2 enforcement layer and a
first-class part of building the flow — they are what make the flow fire every
turn rather than only when the agent remembers:

| Hook | Event | Role |
|---|---|---|
| Inject | `UserPromptSubmit` | Pull the current state's orders from its linked `vibe-*` skill and inject them every turn (D12). |
| Guard | `PreToolUse` (`Edit\|Write\|NotebookEdit`) | Hard-block the three invariants, warn elsewhere, via `detect-context.sh decide`. |
| Gate | `Stop` | Warn-first exit-predicate checks (stuck phase, impl-without-tests, forgotten `set-state.sh`). |

Each hook is a thin shell over `.agents/flow/scripts/`; the allow/warn/block
policy lives once in `detect-context.sh` and is never duplicated. Hooks are
earned warn-first and degrade gracefully (exit 0 on any missing keystone). Full
wiring is in [features/platform-adapters/tech.md](features/platform-adapters/tech.md).

---

## Build Sequence

| Order | Component | Feature |
|---|---|---|
| 1 | Update `spec` skill for product/tech/design/plan model | spec-framework (M0 done) |
| 2 | Create `.agents/flow/state-machine.json` and state scripts | vibe-flow |
| 3 | Create `vibe-strategy`, `vibe-feature`, `vibe-quick` skills | vibe-flow |
| 4 | Create `vibe-verify`, `vibe-compound`, `vibe-amend` skills | vibe-flow |
| 5 | Update `AGENTS.md` and `CLAUDE.md` as thin adapters | platform-adapters |
| 6 | Add the `/flow` command adapter that reads `.agents/flow` | platform-adapters |
| 7 | Build the Claude Code plugin: `.claude-plugin/plugin.json` + `hooks/hooks.json` with the three hooks (inject / guard / gate), each a thin shell over `.agents/flow/scripts/` | platform-adapters |
| 8 | Add installer/setup flow for target projects (incl. plugin install) | platform-adapters |

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Dual state systems return | Remove `.spec/.phase` and `.claude/state.json` as canonical concepts; document `.agents/flow` only. |
| `vibe-feature` becomes a mega-skill | Keep state data in JSON/scripts and split verify/compound/amend into separate skills. |
| Delegated skills write to wrong paths | Every `vibe-*` skill injects explicit `.spec/` paths before delegating. |
| Mutable state creates git noise | Version static definitions; gitignore target-project cursors/caches. |
| Adapter leakage | Root specs name `.agents/flow` and `.agents/skills` as canonical; `.claude` is adapter-only. |

---

## Features

| Feature | Covers |
|---|---|
| **spec framework (M0 done)** | Spec skill, templates, validation, authoring flow. [archive/spec-framework/](archive/spec-framework/tech.md) |
| **[features/vibe-flow/](features/vibe-flow/tech.md)** | `.agents/flow` state machine, scripts, `vibe-*` skill contracts. |
| **[features/platform-adapters/](features/platform-adapters/tech.md)** | Codex/Claude adapter files and installer behavior. |
