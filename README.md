<div align="center">

<img src="docs/img/logo.png" alt="vibe" width="360">

**Spec-first structure for coding with agents.**

[![CI](https://github.com/LennardZuendorf/vibe/actions/workflows/ci.yml/badge.svg)](https://github.com/LennardZuendorf/vibe/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![built with](https://img.shields.io/badge/built%20with-bash%20%2B%20markdown%20%2B%20json%20%2B%20node-informational)

</div>

---

**Durable specs, a flow with teeth, and budgeted instructions — three
binaries, no bundle.**

*Three tools, one language, no bundle.*

vibe is three small, independent tools for coding with agents: **spec**
(durable planning memory), **flow** (a state-machine workflow with real
teeth), and **instruct** (layered, budgeted instruction injection). Each
works alone; installed together they compose through file-based contracts,
never through code. One repo, one installer, three release trains.

|  | **spec** | **flow** | **instruct** |
|---|---|---|---|
| **What it is** | Durable `.spec/` planning layer — product / tech / design / plan / lessons — with templates and a validator. | State-machine workflow: routes each phase to the right skills, injects per-turn orders, guards its own write invariants with hooks. | ⏳ Planned — layered, budgeted instruction injection (global → repo → local → session tiers). |
| **Works with** | Any coding agent, or none. | Claude Code, for the hooks and `/flow`. Other agents read the same rules as prose. | Claude Code + OpenCode, both ⏳ planned. |
| **Needs** | `bash` | `bash`, plus Node ≥18 for today's engine — a static Rust binary in v0.4. | Nothing today (unbuilt) — a static Rust binary in v0.4. |
| **Install** | `./install.sh <repo> --only spec` | `./install.sh <repo> --only flow` | ⏳ planned: `./install.sh <repo> --only instruct` |
| **Deep dive** | [`spec/README.md`](spec/README.md) | [`flow/README.md`](flow/README.md) | [`.spec/tech-instruct.md`](.spec/tech-instruct.md) |

This repo builds itself with its own harness, so what you install is what is
dogfooded here.

## Shipped today (v0.3)

Everything in this section is real, installable, and tested right now.
Commands are byte-exact — copy them as written.

### Install

**Prerequisites:** `bash` and `curl` (or `wget`), plus `git` for the target repo.
[`jq`](https://jqlang.github.io/jq/) and [Node](https://nodejs.org) ≥18 are
recommended — both halves degrade without them, but with no Node the flow's
hooks fall back to bash (guard, Stop gate) or go quiet. `claude` CLI is needed only for `--global`.

No clone required — run from inside the repo you want vibe in:

```bash
curl -fsSL https://raw.githubusercontent.com/LennardZuendorf/vibe/main/install.sh | bash
```

It fetches a snapshot (override with `VIBE_REF=<branch|tag>`) and installs
into the enclosing repo. Pass flags after `bash -s --`. Or, from a clone:

```bash
git clone https://github.com/LennardZuendorf/vibe.git && cd vibe
./install.sh <target-repo>
```

A bare run searches upward for the enclosing repo (a `.spec`/`.git` marker)
and, on a terminal, asks which mode you want.

| Mode | Command | What you get |
|---|---|---|
| **local** (default) | `./install.sh <repo>` | The full **stateful** kit in that repo: both skills, the `/flow` command, the engine, and the four Claude Code hooks wired via `.claude/settings.json`. |
| **global** | `./install.sh --global` | The **per-user plugin** (`vibe@vibe`): the portable, stateless surface only — both skills plus the doctrine hook — in **every** repo. Needs the `claude` CLI. |

```bash
./install.sh <repo> --only spec       # the spec framework alone
./install.sh <repo> --only flow       # the flow harness alone
./install.sh <repo> --adapters claude # symlink CLAUDE.md -> AGENTS.md (opt-in)
./install.sh <repo> --with-plugins    # also install companion plugins (superpowers)
./install.sh <repo> --dry-run         # print the plan, write nothing
```

The repo is its own plugin marketplace, if you would rather skip the
installer:

```bash
claude plugin marketplace add LennardZuendorf/vibe
claude plugin install vibe@vibe
```

Uninstall removes only what the installer created — `.spec/**`, `AGENTS.md`
prose, unrelated hooks, and the flow cursor (unless `--yes`) survive:

```bash
./install.sh <repo> --uninstall             # cursor kept; --dry-run to preview
./install.sh <repo> --uninstall --yes       # also remove the flow cursor
./install.sh <repo> --uninstall --only flow # remove one half, leave the other
```

### spec — the planning layer

Every vibe project gets a `.spec/` tree, the single source of truth for what
you are building, why, and how:

```text
.spec/
├── product.md, tech.md, design.md, plan.md, lessons.md   ← ROOT: persistent role, current content
└── features/<name>/
    ├── product.md    required     what this feature does
    ├── tech.md       required     how it is built
    ├── plan.md       recommended  stable <name>/n unit IDs; verification per unit
    ├── design.md     optional     UI/UX or design-system fragment
    └── research.md   optional     findings from spikes / investigations
```

Root files carry no backlog. Feature folders are branch-scoped: written at
design, consumed at impl, merged at compound, then deleted. **Code is truth.**

```bash
/spec setup            # initialise .spec/ with templates
/spec strategy         # write root product/tech/design/plan
/spec feature <name>   # scope and design a named feature
/spec validate         # check structural consistency
```

Pure bash and Markdown — no runtime, no build step.

### flow — the workflow harness

Everything starts at `idle`. The cursor `.agents/skills/vibe/state.json` —
`{flow, phase, feature}` — points at one of 13 states in `state-machine.json`,
the source of truth for each state's skill, delegates, write surface, and
legal `next`.

```bash
/flow feature.design my-feature   # start a feature; names the feature
/flow feature.plan                # advance to planning
/flow idle                        # abort — always legal
```

Most edges auto-advance; `feature.plan → feature.impl` and `feature.verify →
feature.compound` are **gated** and need an explicit confirm token.

| Hook | Does | Without Node |
|---|---|---|
| `SessionStart` | Re-injects the working-model doctrine each session (and on `compact`). | quiet |
| `UserPromptSubmit` | Every turn, two byte-stable lines naming the state and its transition command; full orders on the turn after the cursor moves. | quiet |
| `PreToolUse` | Guards the write invariants. | **still blocks**, via `flow/hooks-fallback/` |
| `Stop` | Warn-first exit checks; blocks in `*.verify` without a fresh evidence receipt. | **still blocks**, via `flow/hooks-fallback/` |

The flow runs on a zero-dependency Node engine ([`flow/engine/`](flow/engine/)).
In an installed repo it is `node .agents/skills/vibe/engine/cli.mjs <command>`
— shortened to `vibe` below:

```bash
vibe state get | vibe state set <target>   # read / move the cursor
vibe orders                                # the current state's orders
vibe doctor                                # install health report (warn-only, always exits 0)
vibe render --list                         # what each injection channel composes, and from where
vibe policy decide <path>                  # the write-invariant verdict for a path
```

A root `vibe.json` (never rewritten by install) turns injected content
blocks on or off and adds your own — see [`flow/README.md` § Injection
config](flow/README.md#injection-config-vibejson). Flow also *delegates* to
external skills declared in [`flow/reference/deps.json`](flow/reference/deps.json)
(superpowers, feature-dev, reported by `vibe doctor`); a missing one degrades,
never hard-fails.

### Today's layout

```text
your-repo/                     # after install
├── .agents/skills/
│   ├── spec/                  # the spec framework
│   └── vibe/                  # the flow: router, phase files, state machine, engine
├── .claude/                   # Claude adapter: /flow + four hooks + settings.json (flow half)
├── .spec/                     # your durable project memory
└── AGENTS.md                  # merged instructions (CLAUDE.md may symlink here)
```

In **this** repo the canonical halves live at [`spec/`](spec/) and
[`flow/`](flow/); `.agents/skills/{spec,vibe}` are compatibility symlinks.

## Tests

```bash
bash tests/run.sh   # spec + flow + adapters + engine suites
```

CI runs `shellcheck` on every tracked `*.sh`, the combined suite on Linux and
macOS `bash` 3.2, the engine suite jq-less and from a space-containing path, a
`CLAUDE_PROJECT_DIR` hermeticity leg, `validate.sh`, and `check-drift.sh`.

## v0.4 target

The re-architecture in progress. Nothing in this section is runnable today —
every command and path below is ⏳ **planned**. Detail lives in
[`.spec/tech.md`](.spec/tech.md) and its branch docs, including
[`.spec/tech-instruct.md`](.spec/tech-instruct.md) for the instruct tool.

### Three binaries, one contract layer

`vibe-spec`, `vibe-flow`, `vibe-instruct` — a Cargo workspace, static, zero
runtime deps. No tool crate depends on another, and a tool never imports a
peer: cross-tool traffic is files under `.vibe/` plus one soft exec
(`vibe-spec … --json`, probed, absence = degrade). `contracts/` holds
versioned schemas + golden fixtures, additive-only within a major; readers
ignore unknown fields.

### `.vibe/` — the per-repo vibe directory (⏳ planned)

```text
.vibe/
├── spec.json, flow.json, instruct.json   # per-tool config; each tool reads only its own file
├── blocks/*.md                           # repo-shared instruction blocks
├── providers/*.json                      # provider manifests
├── local/                                # personal, git-excluded (.git/info/exclude)
└── run/                                  # runtime, gitignored
    ├── flow/state.json, ledger.jsonl, evidence/
    ├── inject/<provider>.json
    ├── instruct/sessions/<id>.json
    └── agents-md/<NN>-<owner>.md
```

The cursor moves from `.agents/skills/vibe/state.json` to
`.vibe/run/flow/state.json` (`vibe-flow migrate`).

### spec root moves to `docs/spec/`

Default `docs/spec/`, legacy `.spec/` still resolved. `vibe-spec migrate --to
docs/spec` rewrites this repo in one commit. spec also adopts **OpenSpec
grammar** (`### Requirement: <ID>`, `#### Scenario:` GIVEN/WHEN/THEN,
`## ADDED|MODIFIED|REMOVED Requirements`) inside vibe's own document model —
grammar-compatible, not layout-adoption — plus `vibe-spec export|import --openspec <dir>`.

### flow TUI

`vibe-flow tui` (ratatui): a live pane (cursor, legal next, orders, ledger
tail), transitions with the same teeth as `state set`, and workflow
recording (`r`/`n`/`s`) to `~/.config/vibe/flows/<name>.json`.

### Installer (planned interface)

```bash
# ⏳ planned — not runnable today
./install.sh --only spec,flow,instruct --platform claude,opencode,none --dry-run
./install.sh --uninstall
```

`install.sh` becomes a POSIX-sh `curl | sh` bootstrapper: detect the target
triple, fetch prebuilt binaries from GitHub Releases, verify checksums, fall
back to `cargo install --locked`, and register each platform's plugins.
`~/.vibe/installed.json` records actions so `--uninstall` reverses only that.

### Degrade matrix

Any subset can be installed; each still does something useful alone.

| Installed | You get |
|---|---|
| spec only | docs, validate, agents, block, pre-commit; no cursor, no injection |
| flow only | full machine, guard, tooth, TUI; adherence via block only; orders cite spec paths if the dir exists, no lessons |
| instruct only | tiers, rules block, its own injection; no providers → only rules |
| spec + flow | orders carry lessons-by-tag and plan rows; delegates resolve; no injection |
| flow + instruct | state line, edge orders, guard events injected; no spec content |
| spec + instruct | blocks composed; spec drift events injected |
| all three | today's behaviour, decoupled |

### Target repo layout

```text
vibe/
├── crates/{vibe-core,vibe-spec,vibe-flow,vibe-instruct,vibe-signals,vibe-parity}/
├── spec/        # plugin: .claude-plugin/ skills/spec/ commands/ agents/ hooks/ bin/ opencode/ reference/ README.md
├── flow/        # plugin: … + state-machine.json policy.json bin/fallback/
├── instruct/    # plugin: … + blocks/ (shipped defaults)
├── contracts/   # schemas + goldens + run.sh
├── oracles/{js,bash}/ + MANIFEST.sha256
├── tests/parity/ (corpus, GREEN.hard)   tests/run.sh
├── .claude-plugin/marketplace.json
├── install.sh
├── .vibe/       # this repo's own config + blocks (+ run/, local/ ignored)
├── docs/spec/   # this repo's memory (after migration; .spec/ until then)
└── .agents/skills/{spec,flow,instruct} → ../../<tool>/skills/<tool>  (compat)
```

## Roadmap

Five phases, F1–F24, each a binary gate. Parallel tracks open once
foundations land: flow (F5→F10), instruct (F11→F15), spec (F16→F20).

| Phase | Focus | Features |
|---:|---|---|
| 0 | Foundations — contracts, workspace core, oracle parity harness | F1–F4 |
| 1 | flow — cursor, teeth hooks, machine-teeth, orders/payload, signals, TUI | F5–F10 |
| 2 | instruct — tiers/blocks, providers, sources/sync, adapters, migration | F11–F15 |
| 3 | spec — root discovery, validators, delta engine, hooks/agents, migrate | F16–F20 |
| 4 | Distribution + retirement — packaging, release, legacy removal, doc truth | F21–F24 |

F1 (the spec path-resolution hotfix) is a same-week fix on the current bash
tree, ahead of the phases above. Full sequence and evidence criteria:
[`.spec/plan.md`](.spec/plan.md). The issue set — one tracking epic plus one
issue per feature — is authored under [`docs/roadmap/`](docs/roadmap/) and
posts to GitHub with `bash docs/roadmap/create-issues.sh` once Issues are
enabled on the repository.

## Platform support

| Host | What works | What is absent |
|---|---|---|
| **Claude Code** | spec + flow today: both skills, `/flow`, per-turn inject, guard + gate hooks. instruct ⏳ planned. | Budgeted instruction injection beyond the flow's own hooks, until instruct ships. |
| **OpenCode** | ⏳ planned — spec, flow, and instruct all ship an OpenCode plugin in v0.4. | Nothing today. |
| **Other `AGENTS.md` readers** (Codex, etc.) | spec framework + committed instruction blocks; agents follow the written flow manually. | Hooks (no per-turn inject / guard / gate). |
| **Bare git / any editor** | spec framework: `.spec/` docs, templates, `validate.sh`. | Flow automation, hooks, instruct. |

## Documentation

- [`spec/README.md`](spec/README.md) · [`flow/README.md`](flow/README.md) — each half, standalone.
- [`.spec/product.md`](.spec/product.md) · [`.spec/tech.md`](.spec/tech.md) · [`.spec/plan.md`](.spec/plan.md) — the harness's own specs, a living worked example.
- [`CHANGELOG.md`](CHANGELOG.md) — release notes.

## License

[MIT](LICENSE) © 2026 Lennard Zündorf
