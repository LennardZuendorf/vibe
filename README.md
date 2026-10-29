<div align="center">

<img src="docs/img/logo.png" alt="vibe" width="360">

**Spec-first structure for coding with agents.**

[![CI](https://github.com/LennardZuendorf/vibe/actions/workflows/ci.yml/badge.svg)](https://github.com/LennardZuendorf/vibe/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![built with](https://img.shields.io/badge/built%20with-rust%20%2B%20bash%20%2B%20markdown%20%2B%20node-informational)

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
| **What it is** | Durable `docs/spec/` planning layer — product / tech / design / plan / lessons — with templates and a validator. | State-machine workflow: routes each phase to the right skills, injects per-turn orders, guards its own write invariants with hooks. | ⏳ Planned — layered, budgeted instruction injection (global → repo → local → session tiers). |
| **Works with** | Any coding agent, or none. | Claude Code, OpenCode, and Factory Droid via `/flow`; Codex via skill invocation. Other agents read the same rules as prose. | Claude Code + OpenCode, both ⏳ planned. |
| **Needs** | `bash`; the `vibe-spec` binary (dev build) for validation | `bash`, plus Node ≥18 for today's engine — a static Rust binary in v0.4. | Nothing today (unbuilt) — a static Rust binary in v0.4. |
| **Install** | `./install.sh <repo> --only spec` | `./install.sh <repo> --only flow` | ⏳ planned: `./install.sh <repo> --only instruct` |
| **Deep dive** | [`spec/README.md`](spec/README.md) | [`flow/README.md`](flow/README.md) | [`docs/spec/tech/instruct.md`](docs/spec/tech/instruct.md) |

This repo builds itself with its own harness, so what you install is what is
dogfooded here.

## Shipped today (v0.3.x, plus the vibe-spec dev build)

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

A bare run searches upward for the enclosing repo (a `docs/spec`/`.git` marker)
and, on a terminal, asks which mode you want.

**Where it installs.** Run from anywhere inside a repository or Git worktree —
the nearest repo/worktree root is the target, not the directory you ran from. A
plain directory with no Git ancestor uses the current directory.

| Mode | Command | What you get |
|---|---|---|
| **local** (default) | `./install.sh <repo>` | The full **stateful** kit at the resolved root: both skills, the engine, and the native surface of every configured host. For Claude Code that is `/flow` plus the hooks wired via `.claude/settings.json`. |
| **global** | `./install.sh --global` | The **per-user plugin** (`vibe@vibe`): the portable, stateless surface only — both skills plus the doctrine hook — in **every** repo. Needs the `claude` CLI. |

```bash
./install.sh <repo> --only spec       # the spec framework alone
./install.sh <repo> --only flow       # the flow harness alone
./install.sh <repo> --adapters claude # symlink CLAUDE.md -> AGENTS.md (opt-in)
./install.sh <repo> --with-plugins    # also install companion plugins (superpowers)
./install.sh <repo> --dry-run         # print the plan, write nothing
```

**Host selection.** With no flag, the installer detects the `claude`,
`opencode`, `codex`, and `droid` CLIs on `PATH` and configures only the hosts it
finds. If it finds none, the host-neutral core still installs and the summary
prints the explicit selection syntax. `--platform LIST` overrides detection in
both directions — the listed hosts are configured whether or not their CLIs are
present:

```bash
./install.sh <repo> --platform claude,codex   # only these two hosts
./install.sh <repo> --platform none           # host-neutral files only
```

`--platform` takes a comma-separated list of `claude`, `opencode`, `codex`,
`droid`, or `none`; the **last** `--platform` flag wins, and `none` cannot be
combined with a host. What each host gets is in
[Platform support](#platform-support).

The repo is its own plugin marketplace, if you would rather skip the
installer:

```bash
claude plugin marketplace add LennardZuendorf/vibe
claude plugin install vibe@vibe
```

Uninstall removes only what the installer created — `docs/spec/**`, `AGENTS.md`
prose, unrelated hooks, and the flow cursor (unless `--yes`) survive:

```bash
./install.sh <repo> --uninstall             # cursor kept; --dry-run to preview
./install.sh <repo> --uninstall --yes       # also remove the flow cursor
./install.sh <repo> --uninstall --only flow # remove one half, leave the other
```

Every install records the artifacts it created in `.vibe/install-manifest.json`
(deliberately not gitignored), which is what makes re-install idempotent and
`--uninstall` safe: it removes only the recorded vibe-owned paths and preserves
user files in every shared directory. A repeat run leaves the managed files
byte-stable and keeps your live cursor and evidence receipts. `--uninstall`
inverts every host, regardless of `--platform`.

#### The `vibe-spec` binary

`/spec validate` and the spec hooks run `vibe-spec`. `install.sh` does not ship
the binary yet; build it from a clone:

```bash
cargo install --locked --path crates/vibe-spec
vibe-spec --version
```

### spec — the planning layer

Every vibe project gets a `docs/spec/` tree, the single source of truth for what
you are building, why, and how:

```text
docs/spec/
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
/spec setup            # initialise docs/spec/ with templates
/spec strategy         # write root product/tech/design/plan
/spec feature <name>   # scope and design a named feature
/spec validate         # check structural consistency
```

The `/spec` skill is Markdown and bash. Validation (`/spec validate`) runs the
`vibe-spec` binary, which is a Rust dev build today (see [Install](#install)).

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
| `PreToolUse` | Guards the write invariants on file-tool calls (Edit/Write/MultiEdit/NotebookEdit/Bash); on `Bash` it only warns. | **still blocks**, via `flow/hooks-fallback/` |
| `Stop` | Warn-first exit checks; blocks in `*.verify` without a fresh evidence receipt. | **still blocks**, via `flow/hooks-fallback/` |

The flow runs on a zero-dependency Node engine ([`flow/engine/`](flow/engine/)).
In an installed repo it is `node .agents/skills/vibe/engine/cli.mjs <command>`
— shortened to `vibe` below:

Orders arrive once, on the turn after the cursor moves. If they are lost (a
compaction, a missed turn), `vibe orders` or `bash .agents/skills/vibe/scripts/orders.sh`
recovers them.

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
├── .claude/                   # Claude adapter: /flow + hooks + settings.json
├── .opencode/                 # OpenCode: /flow project command
├── plugins/vibe/              # Codex: skill package (+ .agents/plugins/marketplace.json)
├── factory-plugin/            # Factory Droid: /flow + skills (+ .factory-plugin/marketplace.json)
├── .vibe/install-manifest.json # what this install created; --uninstall reads it
├── docs/spec/                 # your durable project memory
└── AGENTS.md                  # merged instructions (CLAUDE.md may symlink here)
```

Each host surface appears only when that host was configured — a Claude-only
install writes `.claude/` and nothing under `.opencode/`, `plugins/`, or
`factory-plugin/`. In **this** repo the canonical halves live at
[`spec/`](spec/) and [`flow/`](flow/); `.agents/skills/{spec,vibe}` are
compatibility symlinks.

## Tests

```bash
bash tests/run.sh   # spec + flow + adapters + engine suites
```

CI runs `shellcheck` on every tracked `*.sh`, the combined suite on Linux and
macOS `bash` 3.2, the engine suite jq-less and from a space-containing path, a
`CLAUDE_PROJECT_DIR` hermeticity leg, `vibe-spec validate --strict`,
`vibe-spec check-drift`, and `vibe-spec migrate --to docs/spec --check`.

## v0.4 target

The re-architecture in progress. Nothing in this section is runnable from an
install today — every command and path below is ⏳ **planned**. The exceptions
are the `vibe-spec` commands that already exist in a dev build (`cargo run -p
vibe-spec -- <command>`), among them `check-drift`, `promote`, `export|import
--openspec`, `hook {session-start,guard,validate}`, `setup`, `init` and `migrate`; skills, CI and the flow scripts still call the bash scripts and do
not use them yet. Detail lives in
[`docs/spec/tech.md`](docs/spec/tech.md) and its branch docs, including
[`docs/spec/tech/instruct.md`](docs/spec/tech/instruct.md) for the instruct tool.

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

Default `docs/spec/`, legacy `.spec/` still resolved. `vibe-spec migrate --to docs/spec` <!-- literal -->
rewrites this repo in one commit. spec also adopts **OpenSpec
grammar** (`### Requirement: <ID>`, `#### Scenario:` GIVEN/WHEN/THEN,
`## ADDED|MODIFIED|REMOVED Requirements`) inside vibe's own document model —
grammar-compatible, not layout-adoption — plus `vibe-spec export|import --openspec <dir>` (dev build only; not yet wired into skills or CI scripts).

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
├── docs/spec/   # this repo's memory
└── .agents/skills/{spec,flow,instruct} → ../../<tool>/skills/<tool>  (compat)
```

## Roadmap

Five phases, each a binary gate. Parallel tracks open once
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
[`docs/spec/plan.md`](docs/spec/plan.md). The per-feature issue drafts are under
[`docs/roadmap/`](docs/roadmap/).

## Platform support

One project install lays down a host-neutral core plus a native surface for each
configured host. Paths below are relative to the installed root.

| Host | Start the flow workflow | Skills live at | Wired per host |
|---|---|---|---|
| **Claude Code** | `/flow` — native project command (`.claude/commands/flow.md`) | `.agents/skills/{spec,vibe}` + `.claude/skills/{spec,vibe}` | All hooks, via `.claude/settings.json`. |
| **OpenCode** | `/flow` — native project command (`.opencode/commands/flow.md`) | `.agents/skills/{spec,vibe}` | No hook integration. |
| **Codex** | Skill invocation only — `$vibe` (or the skill picker `/skills`); there is no synthetic `/flow` command | `.agents/skills/{spec,vibe}` + `plugins/vibe/skills` | No hook integration; no native agent registration. |
| **Factory Droid** | `/flow` — native project command (`factory-plugin/commands/flow.md`) | `.agents/skills/{spec,vibe}` + `factory-plugin/skills` | No hook integration; no native droid registration. |
| **Other `AGENTS.md` readers** | Manual — the agent follows the `AGENTS.md` rules as prose | `.agents/skills/{spec,vibe}` (plain files) | No hooks, no native command. |
| **Bare git / any editor** | — (spec framework only) | — | `docs/spec/` docs, templates, `vibe-spec`. |

The **spec** workflow is skill-invoked on every host (the skill is
user-invocable, so hosts that list skills as commands offer it as `/spec`). The
flow's per-turn inject, guard and stop-gate hooks are wired for Claude Code only
in this slice: OpenCode, Codex, and Factory Droid get skills and a workflow entry
point — and no host gets native agent/droid registration. (The `vibe` store hook
below is separate and covers all four harnesses.)
The install summary reports a host's hooks only when their wiring actually
installed. Budgeted instruction injection (`instruct`) remains ⏳ planned.

## Global store

`vibe` (crate `crates/vibe`) keeps one user-level store and projects it into Claude Code, Codex, OpenCode and Factory droid. Decision: [D48](docs/spec/decisions/d48-global-store-and-projection.md); tech note: [`cli-store.md`](docs/spec/tech/cli/cli-store.md).

```bash
vibe install [--harness claude,codex] [--dry-run]   # store init, one SessionStart hook per detected harness, OpenCode plugin, then sync
vibe store import-mcp ~/old/mcp.json                # merge servers; secrets become ${VIBE_<SERVER>_<FIELD>}
vibe sync [--dry-run] [--json]                      # full projection, including Claude MCP via `claude mcp add-json`
vibe store status                                   # per-harness drift, skips, conflicts
vibe uninstall [--dry-run]                          # reverse exactly what vibe created; the store stays
```

Store layout (`~/.config/vibe/`):

```text
store.json   # harness switches, extra skill sources
AGENTS.md    # root instructions, the single source
mcp.json     # {"mcpServers":{...}}
skills/      # one dir per skill, each with SKILL.md
```

`install` wires the harnesses it detects (configuration directory present, or its CLI `claude`, `codex`, `opencode`, `droid` on `PATH`); an undetected one gets a note, and `--harness <id>` forces it. Without the `claude` CLI, `install` and `sync` skip the Claude MCP step with one note and exit 0; after you install the CLI, run `vibe sync` once to project MCP.

After install, every harness session start re-projects the store. When something changed, the hook prints one notice line: a `{"systemMessage":"vibe: …"}` JSON line for Claude, Codex and Factory (shown to you), plain text for OpenCode. The hook never injects instruction content and always exits 0. Claude MCP changes need `vibe sync`; until you run it, every session start repeats the notice, and `vibe store status` shows `claude mcp: out of date`. vibe changes only its own marker blocks and the entries it recorded in `state.json`; everything else survives install, sync and uninstall.

**Secrets.** vibe never writes a secret it detects on import. Keep API keys in your environment and reference them as `${NAME}` placeholders in `mcp.json`; `store import-mcp` does the replacement and prints variable names only (`--keep-literals` opts out). An env secret of key `K` becomes `${K}` (so Codex can forward it) unless that name has another use in the store; headers and URLs get `${VIBE_<SERVER>_<FIELD>}`. Literals you write into `mcp.json` yourself are projected as written, with a `literal secret-looking value projected` note. URL userinfo is never rewritten and always warns.

| Variable | Effect |
|---|---|
| `VIBE_STORE` | the whole global root (default `~/.config/vibe`): the store files and what later CLI slices keep beside them |
| `VIBE_STATE` | state dir (default `~/.local/state/vibe`) |
| `VIBE_TOOLKIT` | directory holding vibe's own skills, added as a skill source by `install` |
| `VIBE_SPEC_BIN` | `vibe-spec` binary used by `vibe spec …` |

Known limits: Windows skills links are skipped; a Codex `config.toml` with a root inline `mcp_servers = {…}` gets no vibe MCP block.

## Documentation

- [`spec/README.md`](spec/README.md) · [`flow/README.md`](flow/README.md) — each half, standalone.
- [`docs/spec/product.md`](docs/spec/product.md) · [`docs/spec/tech.md`](docs/spec/tech.md) · [`docs/spec/plan.md`](docs/spec/plan.md) — the harness's own specs, a living worked example.
- [`CHANGELOG.md`](CHANGELOG.md) — release notes.

## License

[MIT](LICENSE) © 2026 Lennard Zündorf
