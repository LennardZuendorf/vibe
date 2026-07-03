---
type: entrypoint
scope: technical
children:
  - features/vibe-flow/tech.md
  - features/platform-adapters/tech.md
  - features/agent-instructions/tech.md
  - features/install-tooling/tech.md
  - features/release-docs/tech.md
updated: 2026-07-03
---

# vibe — Technical Architecture

Project-level architecture for the combined `spec` framework and `vibe` flow
harness. Feature-level implementation detail lives under `.spec/features/<name>/`.

---

## Design Philosophy

1. **File-based contracts.** Specs, flow state, skills, and adapter instructions
   are ordinary files that agents can inspect and tools can validate.
2. **Separation of durability.** `.spec/` is durable project memory;
   `.agents/skills/vibe/` is runtime workflow state.
3. **One skill, many phases.** `vibe` is a first-class agent skill — a router
   `SKILL.md` plus per-phase files (setup/strategy/feature/quick/verify/compound/
   amend), with shared scripts and references.
4. **Platform-neutral core.** Claude Code and Codex files are adapters that read
   `.agents/skills/vibe` and invoke the `vibe` skill.
5. **Delegation with constraints.** `vibe` phases call `spec`, `superpowers:*`,
   and subagents with explicit path instructions.
6. **Scripts for deterministic machinery.** State reads/writes, validation, and
   adapter installation use bash scripts rather than repeated prose.

---

## Architecture Overview

```mermaid
flowchart TD
  U["User intent"] --> A["Adapter: Codex / Claude Code"]
  A --> K["vibe skill (phase file)"]
  K --> F[".agents/skills/vibe/state.json"]
  K --> M[".agents/skills/vibe/state-machine.json"]
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
| Spec framework | `.agents/skills/spec/` (→ `spec/`), `.spec/**` | Durable project planning and validation. |
| Vibe flow core | `.agents/skills/vibe/**` (→ `flow/`) | Platform-neutral flow state, state machine, transition + health scripts. |
| Vibe skill | `.agents/skills/vibe/` | One agent skill — router `SKILL.md` + per-phase files — that delegates to real skills. |
| Install tooling | `install.sh`, `flow/scripts/doctor.sh`, `flow/reference/deps.json` | Copy/merge provisioning, partial/dry-run/uninstall, health report, dep manifest. |
| Platform adapters | `AGENTS.md`, `CLAUDE.md`, `.claude/**`, `.claude-plugin/` | Runtime-specific integration over the same core, incl. the Claude Code plugin + hooks. |

---

## File Layout

```text
vibe/
├── AGENTS.md
├── CLAUDE.md
├── README.md
├── install.sh
├── spec/                               # symlink → .agents/skills/spec
├── flow/                               # symlink → .agents/skills/vibe
├── .agents/
│   └── skills/
│       ├── spec/                       # spec skill (symlink target)
│       └── vibe/                       # workflow skill (SKILL.md + phase files + state machine + scripts)
├── .claude-plugin/
│   └── plugin.json                     # Claude Code plugin manifest (bundles cmd+skills+hooks)
├── .claude/
│   ├── commands/flow.md                # Claude adapter, reads .agents/skills/vibe
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

Target projects receive the same `.agents/skills` core, plus
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
└── reference/
    ├── product.md, tech.md, plan.md, design.md
    └── templates/          # hard-floor root + feature templates (SF5–SF7)

tests/spec/run.sh           # repo-root behaviour tests (123 cases, SF0–SF19)
tests/run.sh                # combined runner: spec (123) + flow (59) + adapters (66) = 248
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
  read-on-entry; the `compound` phase writes + `regen-active-rules.sh` digest.

Feature specs are ephemeral: design → plan → impl → verify → compound →
`archive/<feature>/`. Cross-cutting decisions promote into root specs; feature-only
detail stays in archive. Promotable tech blocks use `<!-- merge -->` markers.

---

## Vibe Flow Contract

The flow state lives under `.agents/skills/vibe`. States are compound `<flow>.<phase>`
keys; the cursor carries only the moving parts and no turn-varying fields:

```json
{
  "flow": "idle | setup | strategy | feature | quick",
  "phase": "idle | detect | apply | brainstorm | spec | design | plan | impl | verify | compound | triage | fix",
  "feature": null,
  "updated": "2026-06-02T00:00:00Z"
}
```

`state-machine.json` defines each `<flow>.<phase>` state with its linked `vibe`
phase, caveman level, allowed write surfaces, and exit predicate. The `skill` field
**links** the state to the `vibe` skill; under D12 the per-turn orders are sourced
from that linked skill's phase block rather than a hand-written `inject` string. A single inject
owner (the `UserPromptSubmit` hook) pulls the current state's orders from its
linked skill and injects them once per turn (which also sets the caveman level),
keeping the inject byte-stable and prompt-cache-safe. `set-state.sh` is the only
sanctioned writer. Full per-state mapping lives in
[features/vibe-flow/tech.md](features/vibe-flow/tech.md).

---

## Code Skill Contract

`vibe` is one agent skill: a router plus per-phase files and shared machinery.

```text
.agents/skills/vibe/            # → flow/
├── SKILL.md                    # router + D12 orders blocks
├── {setup,strategy,feature,quick,verify,compound,amend}.md   # per-phase guides
├── state-machine.json          # states, links, next, caveman
├── state.example.json          # cursor template (state.json gitignored)
├── reference/deps.json         # external dependency manifest
└── scripts/                    # set-state, validate-state, detect-context,
                                # orders, check-skills, regen-active-rules,
                                # doctor, merge-agents
```

Each phase body must stay small and procedural:

1. Read `.agents/skills/vibe/state.json` and relevant `.spec/` entrypoints.
2. Confirm the current phase or transition through `.agents/skills/vibe/scripts/set-state.sh`.
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

Adapters never own canonical state. They read `.agents/skills/vibe` and invoke
the `vibe` skill.

| Adapter | Owns | Does Not Own |
|---|---|---|
| Codex | `AGENTS.md` instructions, optional desktop/thread affordances | Flow state, spec layout |
| Claude Code | `CLAUDE.md`, `.claude-plugin/plugin.json`, `.claude/commands/*`, `.claude/hooks/*` | Canonical skills, state machine, the allow/warn/block policy (that lives in `detect-context.sh`) |
| Installer | Copy core `.agents` files + Claude adapter, merge `AGENTS.md`, seed+gitignore the cursor; `--only`/`--dry-run`/`--uninstall`; `doctor.sh` health; register the plugin | Project-specific product decisions |

### Claude Code plugin & hooks

The Claude Code adapter is packaged as an installable **plugin**
(`.claude-plugin/plugin.json`) that bundles the `/flow` command and the flow
**hooks** via `${CLAUDE_PLUGIN_ROOT}`. A plugin cannot carry skills outside its
own `skills/` dir, so the `spec` + `vibe` skills ship as project files through
`install.sh`, not the plugin. The hooks are the Stage 2 enforcement layer — what
makes the flow fire every turn rather than only when the agent remembers:

| Hook | Event | Role |
|---|---|---|
| Inject | `UserPromptSubmit` | Pull the current state's orders from its linked `vibe` phase and inject them every turn (D12). |
| Guard | `PreToolUse` (`Edit\|Write\|NotebookEdit`) | Hard-block the three invariants, warn elsewhere, via `detect-context.sh decide`. |
| Gate | `Stop` | Warn-first exit-predicate checks (stuck phase, impl-without-tests, forgotten `set-state.sh`). |

Each hook is a thin shell over `.agents/skills/vibe/scripts/`; the allow/warn/block
policy lives once in `detect-context.sh` and is never duplicated. Hooks are
earned warn-first and degrade gracefully (exit 0 on any missing keystone). Full
wiring is in [features/platform-adapters/tech.md](features/platform-adapters/tech.md).

---

## Build Sequence

| Order | Component | Feature |
|---|---|---|
| 1 | Update `spec` skill for product/tech/design/plan model | `spec` skill bundle (M0 done) |
| 2 | Create `.agents/skills/vibe/state-machine.json` and state scripts | vibe-flow |
| 3 | Create `vibe-strategy`, `vibe-feature`, `vibe-quick` skills | vibe-flow |
| 4 | Create `vibe-verify`, `vibe-compound`, `vibe-amend` skills | vibe-flow |
| 5 | Update `AGENTS.md` and `CLAUDE.md` as thin adapters | platform-adapters |
| 6 | Add the `/flow` command adapter that reads `.agents/skills/vibe` | platform-adapters |
| 7 | Build the Claude Code plugin: `.claude-plugin/plugin.json` + `hooks/hooks.json` with the three hooks (inject / guard / gate), each a thin shell over `.agents/skills/vibe/scripts/` | platform-adapters |
| 8 | Add installer/setup flow for target projects (incl. plugin install) | platform-adapters |

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Dual state systems return | Remove `.spec/.phase` and `.claude/state.json` as canonical concepts; document `.agents/skills/vibe` only. |
| `vibe-feature` becomes a mega-skill | Keep state data in JSON/scripts and split verify/compound/amend into separate skills. |
| Delegated skills write to wrong paths | Every `vibe-*` skill injects explicit `.spec/` paths before delegating. |
| Mutable state creates git noise | Version static definitions; gitignore target-project cursors/caches. |
| Adapter leakage | Root specs name `.agents/skills/vibe` and `.agents/skills` as canonical; `.claude` is adapter-only. |

---

## Features

| Feature | Covers |
|---|---|
| **spec framework (done)** | Spec skill, templates, validation, authoring flow. [`.agents/skills/spec/`](../.agents/skills/spec/SKILL.md) |
| **[features/vibe-flow/](features/vibe-flow/tech.md)** | `.agents/skills/vibe/` state machine, scripts, the one `vibe` skill's contracts. |
| **[features/agent-instructions/](features/agent-instructions/tech.md)** | `AGENTS.md` template + `merge-agents.sh` marker merge + adapter symlinks. |
| **[features/platform-adapters/](features/platform-adapters/tech.md)** | Claude plugin, three hooks, `install.sh` core provisioning. |
| **[features/install-tooling/](features/install-tooling/tech.md)** | `install.sh` flags, `doctor.sh`, `deps.json`. |
| **[features/release-docs/](features/release-docs/tech.md)** | READMEs, rails (LICENSE/CHANGELOG/CI/runner), logo, examples, stranger eval. |
