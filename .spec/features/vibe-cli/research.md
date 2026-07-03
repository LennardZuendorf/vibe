---
name: vibe-cli
type: research
status: draft
date: 2026-07-03
---

# vibe-cli — Research & Design: port the flow into a Python (typer + rich) CLI

**One-liner.** Replace the bash flow scripts + `install.sh` + manual `/plugin` dance with a
single installable CLI (`vibe`) that *is* the state machine and makes project setup — skills,
hooks, `AGENTS.md`, and dependency plugins — a one-command operation. Reference architecture:
`LennardZuendorf/indexed` (typer + rich, `uv tool install`).

> Scope note: this is a research/design doc produced during exploration. It precedes the formal
> `product.md` / `tech.md` / `plan.md` for this feature, which follow after the flagged decisions
> below are confirmed.

---

## 1. Why

The current harness works but its **user surface is bash scripts + manual GUI steps**. From the
user-flow audit (23 distinct flows), the sharpest friction is all in *setup and orientation*:

- **Three-mechanism setup.** `./install.sh` (file copy) → manual `/plugin` registration (hooks) →
  in-agent "set up vibe" (`setup.detect`/`setup.apply`). None reference the others; a user who does
  one doesn't know the others exist.
- **Hooks silently don't fire** until the plugin is registered by hand — the "flow fires every turn"
  value prop is dark until an undiscoverable GUI step happens.
- **No `vibe status`.** To answer "where am I / what's my legal next move" you read `state.json` by
  hand or run `validate-state.sh`. `set-state.sh` needs the exact `<flow>.<phase>` key and offers no
  `--list`.
- **No one-liner install.** A stranger must `git clone` before `./install.sh`.
- **Installing the dependency plugins** (superpowers, feature-dev, caveman) is entirely on the user.

The goal is to fold all of that into a CLI built "around user needs," and to **back the state
machine into the CLI itself** rather than a pile of scripts.

---

## 2. What exists today (condensed)

Full maps live in the exploration transcripts; the load-bearing facts:

- **State machine** — `flow/state-machine.json` (`version:"1"`): 15 states, compound keys
  `<flow>.<phase>` (bare `idle` when flow==phase), vocabularies `flows`/`phases`/`modifiers`,
  `caveman_levels`, `safety_carveouts`, and per-state `{skill, delegates, caveman, reads, writes,
  inject, next, exit}`. **This JSON is the source of truth and should stay data** (repo convention:
  "state machine is data — edit the JSON, not prose").
- **8 flow scripts** (`flow/scripts/`): `set-state` (only cursor writer, atomic), `validate-state`,
  `detect-context` (policy oracle: allow/warn/block + the **3 hard blocks** — lessons.md, root
  specs, state.json), `orders` (D12 per-turn inject resolution via `SKILL.md` marker blocks),
  `check-skills`, `regen-active-rules` (top-5 lessons digest into managed markers), `doctor`
  (warn-only health, always exit 0), `merge-agents` (AGENTS.md marker-merge + adapter symlinks +
  unmerge, 5 cases).
- **Installer** — `install.sh`: 7 steps (copy skills, copy adapter, seed cursor, gitignore cursor,
  merge AGENTS.md, adapter symlinks, print registration hint); `--only spec|flow`, `--dry-run`,
  `--uninstall` (surgical: preserves `.spec/`, user prose, cursor), `--yes`, `--adapters`.
- **3 hooks** — `.claude/hooks/*` wired via `.claude-plugin/plugin.json`:
  `UserPromptSubmit→inject` (thin shell over `orders.sh`), `PreToolUse:Edit|Write|NotebookEdit→guard`
  (`detect-context decide`, block→exit 2), `Stop→gate` (warn-only smell checks). Today they rely on
  `${CLAUDE_PLUGIN_ROOT}` and require the plugin to be registered.
- **Data contracts** — cursor `{flow,phase,feature,updated}`; marker pairs `vibe:orders:<state>`,
  `vibe:instructions`, `vibe:active-rules`; `deps.json`, `adapters.json`.
- **Spec half** — `spec/` skill + `validate.sh`, `setup.sh`, `promote.sh`, etc. **Explicitly the
  standalone, zero-runtime, any-agent half.**

---

## 3. Verification (two probes that de-risk the design)

### 3a. Hot-path startup cost — the guard fires per-Edit

Measured (M-series Mac, Python 3.14, warm cache; avg of 8 process spawns):

| path | ms/run |
|---|---|
| bash + jq (current guard) | ~10 |
| python cold start | ~19 |
| python + **stdlib only** (json/argparse) | ~24 |
| python + typer/rich/pydantic | ~66 |

The **guard is the only per-Edit hook** (dozens of invocations in an impl turn). Routing it through
the full typer app (~66 ms, and multiples of that on cold cache / CI / slow disks) would add real
lag exactly during implementation. **Mitigation: a second stdlib-only console_script for the hook
hot-path** (~24 ms, near bash parity) — mirrors `indexed`'s `indexed` + `indexed-mcp` dual entry.
Both entries share one cheap-import policy module; the rich `vibe` app never loads on the hot path.

### 3b. What a CLI can actually automate in Claude Code (verified against docs)

- **Hooks in `.claude/settings.json` fire immediately — no plugin, no `/plugin` UI step.** This
  removes the single worst friction. `vibe init` writes the three hooks to settings.json and they
  work on next session.
- **`claude plugin install <p>@<marketplace> --scope project` is fully non-interactive** (writes
  settings.json). Also `claude plugin marketplace add`, `enable`, `disable`, `list --json`. So
  `vibe init` can install superpowers / feature-dev / caveman for the user.
- **Skills auto-discover from `.claude/skills/<name>/SKILL.md`** (project) or `~/.claude/skills/`;
  drop files, no restart. (Repo already ships to `.claude/skills/` today.)
- **Constraint:** settings.json hooks get `CLAUDE_PROJECT_DIR` but **not** `CLAUDE_PLUGIN_ROOT` →
  hook scripts must self-locate. Solved by the CLI being globally installed: hooks call the
  `vibe-hook` binary on `PATH`; project is found via `CLAUDE_PROJECT_DIR` / marker search.
- **Unavoidably manual:** the one-time "trust this folder?" workspace dialog. Everything else
  automates.

**Consequence:** setup can genuinely become *one command*. The plugin (`plugin.json`) is now
*optional* — settings.json hooks are the primary, zero-friction path.

---

## 4. Approaches

**A — Python CLI owns the flow; bash becomes vestigial. (recommended)**
New single-package CLI, dual entry (`vibe` rich + `vibe-hook` stdlib-fast). `state-machine.json`
stays canonical, loaded into pydantic models. All flow logic (transitions, policy oracle + 3 hard
blocks, orders resolution, active-rules regen, doctor, AGENTS.md merge, install/uninstall) ported to
Python. Hooks move to `.claude/settings.json → vibe-hook`. The **spec half stays bash** and the CLI
*wraps* it (`vibe spec validate` shells to `validate.sh`), preserving its standalone/zero-runtime
promise. `flow/scripts/*` and `install.sh` retire (optionally kept one release as deprecated shims).

*Why:* directly satisfies "supersede the implementation and scripts" and "back the state machine
into its own CLI." Perf de-risked by the fast entry. Setup unified into `vibe init`.

**B — Hybrid: CLI as front-door, bash as guts.**
CLI does human-facing commands (init/status/doctor); hooks keep calling bash orders/detect/gate.
*Rejected:* leaves two sources of truth — the exact friction we're removing — and doesn't move the
state machine into the CLI.

**C — Thin wrapper over bash.**
`vibe` just shells out to existing scripts with prettier output. *Rejected:* logic stays in bash;
contradicts "back the state machine into its own CLI."

---

## 5. Target architecture (Approach A)

### 5a. Package (mirror `indexed`, skip its monorepo machinery)

```
cli/
├── pyproject.toml          # name: vibe-flow (PyPI); scripts: vibe, vibe-hook; hatchling; src-layout
└── src/vibe/
    ├── app.py              # root Typer app + @callback + main(); registers sub-apps
    ├── hook.py             # SECOND entry (vibe-hook): stdlib-only argparse; inject|guard|gate
    ├── machine.py          # pydantic load of state-machine.json (StateMachine, State) — data stays canonical
    ├── cursor.py           # read/atomic-write state.json (ports set-state semantics)
    ├── policy.py           # allow/warn/block oracle + the 3 hard blocks (cheap-import; shared by hook.py)
    ├── orders.py           # D12 resolution: SKILL.md marker block → machine.inject → fallback
    ├── provision/          # init/uninstall/update: skills copy, settings.json hooks, AGENTS.md merge, cursor
    │   ├── settings.py     #   read/merge .claude/settings.json hook entries (idempotent)
    │   ├── agents_md.py    #   port merge-agents.sh marker logic (5 cases + reversal guard + unmerge)
    │   └── plugins.py      #   orchestrate `claude plugin install/add` for deps.json entries
    ├── rules.py            # regen active-rules digest (ports regen-active-rules.sh)
    ├── doctor.py           # rich health report (ports doctor.sh) + fix hints
    ├── ui/                 # console singleton, theme accessors, alerts/cards/panels (indexed pattern)
    └── errors.py           # VibeError base; typer.Exit(1) at call sites
tests/                      # CliRunner + Path.home sandbox; parity tests vs current bash outputs
```

Distribution: publish `vibe-flow` to PyPI → `uv tool install vibe-flow` (persistent PATH) → `vibe`.
`uvx vibe-flow init` works to try it, but init **verifies `vibe`/`vibe-hook` are on PATH** and warns
if ephemeral (else hooks silently no-op — the bootstrapping trap).

### 5b. Command surface (grouped; no `indexed` dual-registration baroqueness)

Human-facing (`vibe`, rich):

| command | replaces / new | purpose |
|---|---|---|
| `vibe init [PATH]` | install.sh + /plugin + setup.apply | **headline**: one-command project setup (below) |
| `vibe status` | *new* (fixes #6) | current flow/phase/feature + legal next, as a panel |
| `vibe go <state> [--feature F]` | set-state + /flow guard | legality-checked transition |
| `vibe next` | *new* | list legal next states from current cursor |
| `vibe doctor` | doctor.sh | rich health report + fix hints; `--exit-code` for CI |
| `vibe uninstall [PATH]` | install.sh --uninstall | surgical removal (preserve .spec/, prose, cursor) |
| `vibe update [PATH]` | install.sh re-run | re-provision managed files, preserve cursor/prose |
| `vibe check <path>` | detect-context decide | human-inspectable write-policy verdict |
| `vibe orders` | orders.sh | print current orders (inspection/debug) |
| `vibe rules` | regen-active-rules.sh | regen active-rules digest |
| `vibe plugins [list\|install\|add]` | *new* | orchestrate `claude plugin …` for deps |
| `vibe spec [validate\|setup]` | wraps validate.sh/setup.sh | spec half stays bash; CLI fronts it |

Hook hot-path (`vibe-hook`, stdlib-only, ~24 ms):

| entry | replaces | wired in settings.json as |
|---|---|---|
| `vibe-hook inject` | user-prompt-submit-inject.sh | `UserPromptSubmit` |
| `vibe-hook guard` | pre-tool-use-guard.sh | `PreToolUse` matcher `Edit\|Write\|NotebookEdit` (block→exit 2) |
| `vibe-hook gate` | stop-gate.sh | `Stop` (warn-only) |

### 5c. The headline: `vibe init`

One interactive (rich) command; `--yes` + flags for non-interactive/CI. Steps:

1. **Prereq check** — confirm `vibe`/`vibe-hook` resolvable on PATH (persistent install); warn if not.
2. **Skills** — copy spec + flow skill files into the target's Claude-discovered skills dir
   (`.claude/skills/{spec,vibe}/`), keeping `.agents/skills/` as the portable canonical.
   *(exact dir resolved in tech.md — both exist in the repo today.)*
3. **Hooks** — merge the three hook entries into `.claude/settings.json` (idempotent, marker/keyed) →
   **fire immediately, no `/plugin`**. Scripts call `vibe-hook …`; project found via `CLAUDE_PROJECT_DIR`.
4. **AGENTS.md** — merge the managed `vibe:instructions` block (ports merge-agents.sh; never clobbers
   user prose); offer adapter symlinks (CLAUDE.md/WARP.md).
5. **Cursor** — seed `state.json` if absent; add to `.gitignore`.
6. **Dependency plugins** — offer to `claude plugin install superpowers@… feature-dev@… caveman@…`
   from `deps.json` (confirm each; `--yes` installs all). Register marketplace via
   `claude plugin marketplace add` if needed.
7. **Summary** — rich "what happened / your next step" panel (fixes the "no post-install guidance"
   gap). `--only spec` still supported (spec-only, no hooks/cursor/plugins).

### 5d. Invariants preserved (must not regress)

- `state-machine.json` stays the canonical data contract (pydantic *loads*, never hardcodes states).
- The **3 hard write-blocks** port verbatim into `policy.py`; guard still exits 2 on block.
- Cursor is writer-only through the CLI (`vibe go`), atomic write; never hand-edited.
- Re-install/update **preserves the live cursor and user prose** (regression-tested — this is a
  standing lesson).
- Uninstall is **surgical** (per-file inverse; preserve shared dirs, `.spec/`, prose) with a
  discriminating test.
- Hook scripts self-locate by marker search (no `CLAUDE_PLUGIN_ROOT` reliance).
- Spec half remains bash + zero-runtime + standalone.

---

## 6. MVP sequencing (first plan)

1. **Skeleton + machine + cursor** — package, pydantic `state-machine.json` loader, cursor I/O →
   `vibe status`, `vibe next`, `vibe go` (with legality check). Parity: `vibe go` matches
   `set-state.sh` behavior.
2. **`vibe-hook` fast entry** — `inject`/`guard`/`gate` + `policy.py` (3 hard blocks) + `orders.py`.
   Parity tests assert byte-identical verdicts/orders vs the current bash scripts.
3. **`vibe init`** — settings.json hook merge, skills copy, AGENTS.md merge, cursor seed/gitignore,
   PATH check, summary panel. This is the user-visible win.
4. **`vibe doctor` / `uninstall` / `update`** — health, surgical removal, re-provision (with the
   cursor-preservation + discriminating-uninstall regression tests ported).
5. **Later** — `vibe plugins` (claude-plugin orchestration), `vibe spec` wrappers, `vibe rules`,
   `vibe check`/`orders` inspection, dry-run/preview, adapters.

---

## 7. Flagged decisions (confirm before formal spec + plan)

- **D1 — Scope: Approach A (full flow port; spec half stays bash, wrapped).** vs B (hybrid) / C
  (thin wrapper). *Recommended: A.*
- **D2 — Hooks via `.claude/settings.json` → `vibe-hook`** (fire immediately, no `/plugin`); keep
  `plugin.json` only as an optional secondary channel. *Recommended: settings.json primary.*
- **D3 — Runtime shift accepted.** The flow half now requires Python + `uv tool install vibe-flow`,
  losing the "pure bash / zero-runtime" property for *flow*. Mitigated: spec half stays bash and
  standalone; `vibe-hook` is stdlib-only. *Confirm this trade is acceptable.*
- **D4 — Fate of legacy `flow/scripts/*` + `install.sh`.** Retire outright, or keep one release as
  deprecated shims that call the CLI? *Lean: retire, with a migration note.*
- **D5 — Names.** Command `vibe` (keep); PyPI `vibe-flow` (or `vibe-harness`). *Minor.*

## 8. Open questions

- Exact skills install dir: `.claude/skills/` (confirmed discovery) vs keeping `.agents/skills/`
  canonical + copying/symlinking — resolve in tech.md.
- Whether `vibe init` should also write a project `.mcp.json` for any MCP deps (out of scope for v1).
- Windows support (bash hooks assume POSIX; a Python `vibe-hook` improves portability — possible
  bonus, not a v1 goal).
