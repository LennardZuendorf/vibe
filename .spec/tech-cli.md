---
type: tech-topic
parent: tech.md
scope: vibe cli front door and cross-tool capabilities
covers: dispatcher, repo registry, worktree lifecycle, global spec space, global blocks, degrade rules
updated: 2026-09-26
---

# vibe CLI — Tech

[tech.md](tech.md)

Branch doc for the v0.5 front door: the `vibe` binary, the worktree
lifecycle, and the global spec space. **Not started** — the whole arc is
spec-ahead-of-code and gates on v0.4's F22 (installer/release surface).

## The vibe binary

`crates/vibe`, bin `vibe`, depends on `vibe-core` only (same `cargo metadata`
no-tool-depends-on-tool rule). Subcommands:

| Subcommand | Behaviour |
|---|---|
| `vibe spec\|flow\|instruct …` | Exec the peer binary through the same five-tier discovery as `bin/hook.sh`; a missing peer prints one "not installed" advice line, exit 0 — advice, never failure. |
| `vibe worktree …` | Native — worktree lifecycle (below). |
| `vibe space …` | Native — global spec space (below). |
| `vibe doctor` | Peer checks + registry + global-tier consistency. |

The three tool binaries remain independently installable with their own
plugins and release trains; `vibe` is a convenience front door, never a
requirement (D40). The installer's default target becomes `vibe` (which pulls
the three tools along); `--only` subsets are unchanged. The `vibe` plugin
carries the front-door command surface only — no hooks of its own; spec,
flow, and instruct keep their hook ownership (D25).

## Repo registry

`~/.config/vibe/repos.json` (`%APPDATA%\vibe\` on Windows):

```json
{ "repos": [ { "path": "/abs/path", "registered": "2026-09-26T00:00:00Z" } ] }
```

Registered by `vibe` at install/init and by `vibe worktree create`;
deregistered by `vibe worktree remove` when a worktree's repo has no other
entry, and by `--uninstall` for the main checkout. The registry is the only
cross-repo discovery mechanism — `vibe space list` never scans the disk.

## Worktree lifecycle

`vibe worktree` — one git worktree per feature/branch, runtime state isolated
per worktree. The v0.4 cursor already resolves per project root, so isolation
needs no new mechanism — each worktree is its own root with its own
gitignored `.vibe/run/`:

- `create <name> [--from <branch>]` — `git worktree add`, then bootstrap
  inside the new worktree: `.vibe/spec.json` pointing at the repo's spec root
  (worktrees share committed memory via git), each installed tool's `init`
  for config and AGENTS.md blocks, `.vibe/run/` seeded empty. Refuses when a
  worktree of that name already exists.
- `list` — worktrees with their cursor (flow/phase/feature), uncommitted-file
  count, days since last ledger event. Staleness is advisory, never a gate.
- `remove <name>` — refuses while the cursor is mid-flow (suggest compound or
  `state set idle`), then removes the worktree and its run dir; deregisters.
  This is the only hard refusal in the lifecycle.
- `archive <name>` — compound-time: moves uncommitted `features/<name>/`
  spec residue to the main checkout, then removes. Committed spec edits
  travel by branch merge; `archive` exists for the residue git does not
  carry.

## Global spec space

`vibe space` — the personal layer above per-repo memory:

- **Global lessons library** at `~/.config/vibe/spec/lessons.md`. `vibe-spec
  lessons-for` gains an additive global source: answers merge repo lessons
  with global lessons; on tag conflict the repo entry wins (more specific
  memory beats general doctrine). Flow's orders and session start read the
  merged answer through the existing payload/budget machinery — no new
  channel, no new budget.
- `vibe space lessons edit` — open the global library. `vibe space lessons
  promote <tag>` — copy a repo lesson's rule into the global library
  (verbatim rule + tags; the repo-specific pattern prose stays behind).
- **Cross-repo visibility** — `vibe space list [--json]` shows each
  registered repo: path, flow/phase, feature, last ledger event, spec drift
  flag. `vibe space status <repo>` for one. Reads cursor, ledger, and spec
  JSON answers only — never another tool's internals.

## Global blocks

`vibe space blocks` — list/edit the instruct global tier
(`~/.config/vibe/instruct/`) and pin a personal git source for it. Sync is
`vibe-instruct sync` over the registered source (D32 unchanged); `vibe space`
only registers and edits — it never grows a second sync engine, asserted by
a `cargo metadata` test that `crates/vibe` pulls no network or fetch crate.

## Degrade rules

- No peers installed → dispatch subcommands print one advice line, exit 0.
- Not a git repo → `worktree` subcommands exit 1 with one line.
- No registry / empty registry → `space list` prints "no repos registered",
  exit 0.
- Global library absent → merged answers equal repo-only answers; absence is
  a clean state, not a warning.
- Registry entry points at a moved/deleted path → one warning line, entry
  kept; repair is re-init or `--uninstall`, never a destructive guess.

## Contracts touched

- **Spec JSON v1** gains an additive `global: true` flag on `lessons-for`
  (additive-only within major, per [tech-contracts.md](tech-contracts.md)).
- **Payload v1, marker grammar v1, provider manifest v1** — unchanged;
  `space` and `worktree` inject through existing channels only.

## Testing posture

New code, not a port — no oracles. The house rules apply: every check
asserts its own population (registry floor, worktree floor ≥1 before "none
found" can pass); refusal paths get discriminating tests (the mid-flow
`remove` refusal must fail if replaced by a naive `rm -rf`); `worktree
create` is tested from a fresh non-git target and a bare `mktemp -d` clone
(the stranger-eval rule); every path this doc names is asserted from both
the main checkout and a worktree.
