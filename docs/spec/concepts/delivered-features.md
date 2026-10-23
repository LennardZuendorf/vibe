---
type: concept
updated: 2026-10-23
---

# Delivered features

**Delivered features** — all compounded into the root docs; branch spec folders removed (history in the Delivered section below). Truth is the code + tests:

| Feature | Owns | Tests |
|---|---|---|
| spec | Spec tree + templates + `validate.sh` | `spec/tests/run.sh` |
| vibe-flow | State machine + `vibe` skill + D12 orders | `flow/tests/run.sh` |
| agent-instructions | `AGENTS.md` template + marker merge + adapter symlinks | `flow/tests/adapters/run.sh` |
| platform-adapters | Three Claude hooks + `/flow` + `install.sh` settings wiring | `flow/tests/adapters/run.sh` |
| dogfood | Hook/merge/install behaviours exercised end-to-end | scripted + lessons |
| monorepo-split | `spec/`+`flow/` canonical split, compat symlinks, truth sweep | suites + validate + grep evidence |
| install-tooling | `--only`/`--dry-run`/`--uninstall`, `doctor.sh`, `deps.json` | `flow/tests/adapters/run.sh` + `flow/tests/run.sh` |
| release-docs | READMEs, trust rails, logo, stranger eval | CI + eval report |
| flow-mvp | Precedence + delegation contracts, hybrid plan grammar, two edge-keyed gates, evidence-receipt verify tooth | `flow/tests/run.sh` |
| flow-legibility | Self-carrying orders, SessionStart doctrine hook, loop edges, drift-first nudges | `flow/tests/run.sh` |
| install-distribution | One-command `install.sh`, `--with-plugins`, self-hosting plugin + marketplace | `flow/tests/adapters/run.sh` + `spec/tests/run.sh` |
| js-core | `engine/` + `vibe` CLI dispatch; one cursor reader, root resolver, marker grammar | `engine/tests/` + existing suites |
| content-layer | `content/blocks/**`, typed placeholders, `vibe.json` override layer, `vibe render` | `engine/tests/` |
| inject-triggers | Level/edge/event trigger classes, `content/policy.json`, `Stop` predicate cut | `engine/tests/` + `flow/tests/adapters/run.sh` |
| host-support | Four-host native skill/command packages (`flow/reference/host-surfaces.json` matrix) + `install.sh` `--platform` selection, CLI detection, ownership manifest, surgical uninstall | `flow/tests/adapters/run.sh` |

## Delivered (history)

Cleansed notes for shipped work — detail lives in live surfaces, not this plan.

- **spec** — four-layer model, warn-first `validate.sh`, feature-authoring flow. DONE.
- **vibe-flow** — 13-state machine, six scripts, D12 orders via `orders.sh`. DONE.
- **agent-instructions** — `AGENTS.md` template, marker merge, adapter symlinks. DONE.
- **platform-adapters** — three hooks as shims + `install.sh` settings wiring. DONE.
- **dogfood** — hook/merge/install behaviours scripted end-to-end. DONE.
- **monorepo-split** — canonical halves at `spec/`+`flow/`, compat symlinks, truth sweep. DONE (2026-07-03).
- **install-tooling** — `--only`/`--dry-run`/`--uninstall`, `doctor.sh`, `deps.json`. DONE (2026-07-03).
- **release-docs** — READMEs, trust rails, logo, stranger eval. DONE (2026-07-03).
- **flow-mvp** — precedence contract, two edge-keyed gates, evidence-receipt tooth. DONE (2026-07-08).
- **flow-legibility** — self-carrying orders, SessionStart doctrine hook, drift-first nudges. DONE (2026-07-18).
- **install-distribution** — one-command install, self-hosting plugin + marketplace. DONE.
- **js-core** — `engine/` + `vibe` CLI dispatch, one cursor reader, byte-identical port. DONE.
- **content-layer** — `content/blocks/**`, typed placeholders, `vibe render` with budgets. DONE.
- **inject-triggers** — level/edge/event trigger classes, `policy.json`, `Stop` predicate cut. DONE.
- **host-support** — four-host packages, `--platform` selection, ownership manifest, surgical uninstall. DONE (2026-10-02).
