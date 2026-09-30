---
type: tech-topic
parent: tech.md
scope: rust workspace and distribution
covers: crates, MSRV, dependency allowlist, shim discovery, oracles and parity, TUI, signals, distribution, CI matrix
updated: 2026-09-04
---

# Rust Workspace — Tech

[tech.md](tech.md)

Branch doc for the Rust implementation: workspace layout, the port from the
Node engine and bash scripts, the hook shim, the TUI, and distribution.
**Shipped today:** `flow/engine/` (Node/ESM) plus bash oracles drive
everything this doc describes as a target; none of it exists as Rust yet.

## Workspace

```text
crates/
├── vibe-core       # lib, publish = false — internal only
├── vibe-spec       # bin: vibe-spec
├── vibe-flow       # bin: vibe-flow (feature `tui` default on)
├── vibe-instruct   # bin: vibe-instruct
├── vibe-signals    # lib — Signal, SignalProvider; flow depends on it, core does not
└── vibe-parity     # dev-only differential harness
```

Each tool binary path-depends on `vibe-core` only; no tool crate depends on
another (`cargo metadata` CI test). MSRV 1.88 (the `ratatui` floor), pinned
in `rust-toolchain.toml` and `rust-version`, bumped only at a tagged
release, built at MSRV in CI. Dependency allowlist via `cargo-deny`: `clap`
(derive), `serde`, `serde_json`; `ratatui` + `crossterm` in `vibe-flow`
only, behind feature `tui`. No `toml`, `regex`, `tokio`, or `tempfile` —
JSON is the only config format; glob matching and atomic writes (tmp +
rename, 0600 on unix) are hand-written in core and parity-proven against
the bash oracles.

Errors: core returns `Result<T, vibe_core::Error>`; no panics on input
(clippy `-D unwrap_used`). Bins map errors to the exit codes in
[tech-contracts.md](tech-contracts.md).

## Shim and discovery

`hooks.json` invokes one shim per event:
`"${CLAUDE_PLUGIN_ROOT}/bin/hook.sh" <event>`. `bin/hook.sh` is POSIX sh,
shellcheck-clean, ≤40 lines — the only shell left in the runtime path.
Discovery order:

1. `$VIBE_<TOOL>_BIN` (explicit override)
2. `$CLAUDE_PLUGIN_ROOT/bin/<os>-<arch>/vibe-<tool>` (bundled, optional)
3. `command -v vibe-<tool>` (PATH)
4. flow guard/stop only: `bin/fallback/<event>.sh` (frozen bash, kept
   permanently)
5. `exit 0` with one stderr line — never downloads

Each tier is tested with the higher tiers' variables **deleted**, not
merely unset, from the child environment.

## Oracles and parity

`oracles/js/` freezes `flow/engine/*.mjs` verbatim; `oracles/bash/` freezes
every current script plus both fallbacks; `oracles/MANIFEST.sha256` is
checked in CI so an oracle can only change by an explicit, reviewed commit.
JS bugs found during parity work become pinned `expected-divergence` cases,
fixed in Rust only — the oracle never changes to match a bug.

`crates/vibe-parity` runs the oracle and the candidate over a **generated**
corpus (`tests/parity/corpus/`) and diffs exit code, stdout bytes, and
every resulting file's bytes (cursor, AGENTS.md, `settings.json`). Corpus
classes derive from `state-machine.json` and `policy.json`: cursors ×
{feature, none, malformed, missing}, policies × one mutation per rule,
tool inputs × path class × state. An empty corpus class fails the run — a
floor is asserted, not assumed.

Port order, blast-radius first: guard → stop → cursor/`state set` → policy
→ root → machine/orders/doctrine → payload/compose → doctor →
merge-settings → merge-agents/render (instruct) → spec scripts. The JS
engine is deleted in one commit, only when every unit is parity-green;
oracles stay in the tree permanently. CI job `port-order` fails the build
if `crates/vibe-flow/src/tui/` or `crates/vibe-signals/` exists while
`tests/parity/GREEN.hard` is absent — the TUI and signals cannot get ahead
of the two hard blocks.

## machine-teeth

`vibe-flow state set <flow.phase> [feature] [--confirm]`: the target state
must exist and be in the current state's `next` (`idle` is always legal);
an edge listed in `gates` requires `--confirm` or the call refuses (exit 1,
listing the legal `next` set). No `--force` — repair is `state set idle`
then walking forward. Every write appends a ledger event. `/flow` becomes a
thin wrapper (`confirm` token → `--confirm`); the guard hook and the TUI
call the same `check_transition` function core exposes — one enforcement
point, not three.

## Ledger and signals

`.vibe/run/flow/ledger.jsonl`, `O_APPEND`, rotated at 5 MB, kinds
`transition | hook | signal`. `vibe-signals` defines `Signal{ts, source,
kind: progress|blocked|done|drift, confidence, evidence, state}` — no
verdict field — and `trait SignalProvider { fn observe(&Observation) ->
Vec<Signal> }`. `vibe-core` cannot depend on `vibe-signals` (cycle-free,
asserted by a `cargo metadata` test), so the guard and the gate are
structurally unable to read a signal. Ships now: `RulesProvider`
(fixed-confidence keyword rules) built from the `Stop`/`SubagentStop`
transcript tail. Later, opt-in: `ModelProvider` behind feature `model`.
Surfaced in the TUI's signals pane and as one advisory line in the inject
relay.

## TUI v1

`vibe-flow tui` (ratatui, feature `tui`): a live pane (cursor, legal `next`
with gate marks, orders, last 20 ledger events, one doctor line; 500 ms
mtime poll, no `notify` crate); transitions (pick `next`, gated edges
prompt `y/N`, `a` aborts — same `check_transition`, same teeth); recording
(`r` starts, `n` adds a note, `s` saves to
`~/.config/vibe/flows/<name>.json`, `%APPDATA%` on Windows). A recorded
flow is a linear subset of `state-machine.json`; a repo selects one via
`.vibe/flow.json` `{"machine": "<path>"}`, falling back to the repo's own
machine.

## Distribution

cargo-dist targets: `{x86_64,aarch64}-unknown-linux-musl`,
`{x86_64,aarch64}-apple-darwin`, `x86_64-pc-windows-msvc`; each artifact
ships SHA256 + attestation. Independent tags per tool: `spec-v*`,
`flow-v*`, `instruct-v*`; first Rust releases `flow-v0.4.0`,
`spec-v0.4.0`, `instruct-v0.1.0`. `install.sh` steps: detect triple →
fetch `vibe-<tool>-<ver>-<triple>.tar.gz` + `SHA256SUMS` from GitHub
Releases → verify → install to `--bin-dir` (default `~/.local/bin`) → on
failure, `cargo install --locked` → register the plugin (Claude:
marketplace add + `claude plugin install <tool>@vibe --scope user`;
OpenCode: link the shim into `~/.config/opencode/plugins/`).
`~/.vibe/installed.json` records every action taken; `--uninstall`
reverses only what it recorded — never a guess.

## CI matrix

| Job | Runs |
|---|---|
| `check` | fmt, clippy `-D warnings`, `cargo-deny` |
| `test` | unit + integration, 3 OS × stable + MSRV |
| `parity` | `crates/vibe-parity` vs. both oracle sets (ubuntu + node, dev-only) |
| `shellcheck` | `bin/hook.sh`, `bin/fallback/*.sh`, `install.sh` |
| `cross-smoke` | build all 5 targets; `--version` under qemu for aarch64-linux |
| `port-order` | fails while TUI/signals code exists ahead of `GREEN.hard` |
| `release` | cargo-dist on tag, per tool |
