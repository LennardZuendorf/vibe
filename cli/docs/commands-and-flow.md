# vibe CLI — commands & flow (sketch)

Two console scripts ship from the `vibe-flow` package: **`vibe`** (the rich human
app) and **`vibe-hook`** (a stdlib-only fast entry the settings.json hooks call
every turn). Together they *are* the 15-state flow machine.

## Command surface

```
vibe
├── init [PATH]              provision a project in one command:
│                              skills → .claude/skills, hooks → .claude/settings.json,
│                              AGENTS.md block, seed+gitignore cursor, install dep plugins
│                              flags: --yes  --only spec  --dry-run
├── update [PATH]            re-provision managed files; preserve live cursor + user prose
├── uninstall [PATH]         surgical removal (per-file inverse); --yes also drops the cursor
├── doctor                   health report (skills/hooks/cursor/AGENTS/deps) + fix hints
│                              --exit-code → nonzero on any degraded check (CI gate)
│
├── status                   where am I: flow / phase / feature + legal next states
├── next                     list the legal next states from the current cursor
├── go <state> [-f FEATURE]  transition the cursor — refused unless it is a legal `next`
│
├── check <path>             write-policy verdict for a path (allow / warn:… / block:…)
├── orders                   print the current state's per-turn orders (D12)
├── rules                    regenerate the active-rules digest into CLAUDE.md/AGENTS.md
│
├── spec
│   ├── validate             wrap the spec framework's validate.sh (no flow runtime needed)
│   └── setup                wrap setup.sh (bootstrap a bare .spec/)
│
└── plugins
    ├── list                 list the dependency plugins declared in deps.json
    ├── install              claude plugin marketplace add + install, per dep
    └── add                  claude plugin marketplace add, per dep

vibe-hook                    (wired into .claude/settings.json — automatic, never run by hand)
├── inject                   UserPromptSubmit → emit the current state's orders
├── guard                    PreToolUse(Edit|Write|NotebookEdit) → EXIT 2 on a hard-block
└── gate                     Stop → warn-only end-of-turn smell checks (always exit 0)
```

## The flow, and which command drives it

`vibe go <state>` is the only cursor mover, and it is legality-checked against the
state machine's `next`. `vibe status`/`next` orient you; the lifecycle commands
(`init`/`update`/`uninstall`/`doctor`) are state-independent.

```
  lifecycle (any time):  vibe init · vibe update · vibe uninstall · vibe doctor

  idle
   │
   ├─ go setup.detect ──▶ setup.detect ──▶ setup.apply ──▶ idle
   │
   ├─ go strategy.brainstorm ──▶ strategy.brainstorm ──▶ strategy.spec ──▶ strategy.compound ──▶ idle
   │                                                             └──────────────────────────▶ idle
   │
   ├─ go feature.design ──▶ feature.design ──▶ feature.plan ──▶ feature.impl ──▶ feature.verify ──▶ feature.compound ──▶ idle
   │                                                                  ▲                  │
   │                                                                  └── fix / drift ───┘   (verify → impl or plan)
   │
   └─ go quick.triage ──▶ quick.triage ──▶ quick.fix ──▶ quick.verify ──▶ idle
                                 └── escalate ──▶ feature.design

  amend  =  a scope edit within the CURRENT state's write rules; the cursor does
            NOT change (no `vibe go` — it is a modifier, not a state).
```

Every turn, the three `vibe-hook` entries run automatically:

- **inject** surfaces the current state's orders into the prompt (D12).
- **guard** enforces the write policy on each edit — `block` → exit 2 (the 3 hard
  blocks: `lessons.md` outside `*.compound`, root specs outside `strategy.spec`/
  `feature.compound`/`setup.apply`, direct `state.json` edits); `warn`/`allow` → exit 0.
- **gate** runs warn-only end-of-turn smells (src-without-tests in impl, verify
  evidence reminder, advance nudge).

## Shell → CLI mapping (what each command replaces)

| bash origin | CLI |
|---|---|
| `./install.sh <t>` | `vibe init <t>` |
| `./install.sh <t>` (re-run) | `vibe update <t>` |
| `./install.sh <t> --uninstall` | `vibe uninstall <t>` |
| manual `/plugin` registration | *(none — hooks fire from settings.json)* |
| `set-state.sh <s>` | `vibe go <s>` |
| `detect-context.sh decide <p>` | `vibe check <p>` |
| `orders.sh` | `vibe orders` / `vibe-hook inject` |
| `detect-context.sh` (guard) | `vibe-hook guard` |
| `stop-gate.sh` | `vibe-hook gate` |
| `doctor.sh` | `vibe doctor` |
| `regen-active-rules.sh` | `vibe rules` |
| `validate.sh` (spec) | `vibe spec validate` (wraps it) |

## A day in the life

```bash
uv tool install vibe-flow          # once, persistent PATH
vibe init                          # provision this project (hooks fire next session)
vibe status                        # → idle, next: setup.detect / strategy.brainstorm / feature.design / quick.triage
vibe go feature.design -f auth     # start a feature
#   … design → plan → impl → verify → compound, each `vibe go <next>` …
vibe doctor --exit-code            # gate the install in CI
```

## Parity & tests

- `cli/tests/` — 459 unit + byte-parity tests (`test_parity_{policy,orders,rules}` vs the bash origins).
- `tests/cli/run.sh` — system suite (pytest gate + real install→lifecycle E2E); wired into `tests/run.sh`.
- `tests/cli/parity-shell-vs-cli.sh` — one-time system sweep comparing every bash-origin verdict/orders to the CLI (195/195 byte-identical). Retires with the bash scripts.
