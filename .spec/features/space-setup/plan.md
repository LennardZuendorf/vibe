---
type: feature-plan
feature: space-setup
sibling: tech.md
parent: ../../plan.md
updated: 2026-09-28
---

# Feature: space-setup — Implementation Plan

A `space/` half extracted from today's hand-built, unversioned cross-runtime setup:
data layer first, then the apply engine, then one adapter per runtime (opencode,
droid, codex, claude), then the front door (install flag, doctor, uninstall) with a
stranger eval as the release gate.

> For agentic workers: execute units in Seq order via superpowers:executing-plans or superpowers:subagent-driven-development; each unit's Steps are your task checklist.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** No upstream feature — independent of the Rust arc (F-issues, v0.4
#22) and reads the seam decision in [tech.md](tech.md) as its boundary. Does not
depend on any other feature's units.

---

## Problem Frame

All cross-runtime setup today lives unversioned in home dirs (opencode plugins, the
agent-stack MCP instance, symlinks) and one runtime (claude) has all its wiring in
the repo while two (codex, droid) are drift-broken or unwired. The units below build
inwards-out: the data exists in the repo before any adapter can consume it; the
apply engine exists before any adapter can be idempotent; the front door comes last
so it delegates to a tested half.

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | [Single-source prompt distribution](product.md#requirement-single-source-prompt-distribution-r1) | space-setup/1, space-setup/2, space-setup/4, space-setup/5 |
| R2 | [Canonical MCP declaration, no committed secrets](product.md#requirement-canonical-mcp-declaration-no-committed-secrets-r2) | space-setup/1, space-setup/2, space-setup/5 |
| R3 | [Per-runtime adapters, teeth included](product.md#requirement-per-runtime-adapters-teeth-included-r3) | space-setup/3, space-setup/4, space-setup/5, space-setup/6 |
| R4 | [Idempotent apply, surgical uninstall](product.md#requirement-idempotent-apply-surgical-uninstall-r4) | space-setup/2, space-setup/7 |
| R5 | [One-command fresh-machine bootstrap](product.md#requirement-one-command-fresh-machine-bootstrap-r5) | space-setup/7 |
| R6 | [Doctor asserts its own population](product.md#requirement-doctor-asserts-its-own-population-r6) | space-setup/7 |
| R7 | [omp-parity coexistence](product.md#requirement-omp-parity-coexistence-r7) | space-setup/3 |

---

## Key Technical Decisions

1. **Repo holds templates; home holds instances.** Credential-bearing outputs are
   materialized with env fill at apply time (R2); the repo is scannable-clean.
2. **One fan-out engine, per-runtime adapters as data + scripts.** No second sync
   engine: nothing in space fetches; source sync stays instruct's (seam in tech.md).
3. **Manifest-driven surgical uninstall.** Apply records every created path;
   uninstall is its exact inverse, pinned by discriminating tests.
4. **Hooks are thin shells over the shared flow scripts** — droid adapters call the
   same `.agents/skills/vibe` resolvers the Claude adapter calls; no policy copy.
5. **Fresh-target testing is a unit requirement, not a nice-to-have** — the stranger
   eval from a bare temporary home is the release gate (dogfood-target lesson).

---

## Global Constraints

<!-- Executors: read this section before starting any unit. -->

- Bash: `set -euo pipefail`, shellcheck-clean, graceful-degrade (warn, never hard-fail on absent runtimes).
- Every check that can pass vacuously must assert its own population (file-count or finding floors) — a green that examined nothing is not a green.
- Tests run from a representative fresh target (isolated `$HOME` via `mktemp -d`), never only from the dogfood repo.
- Committed files under `space/` never contain literal credentials; the secrets scan runs in every suite invocation.
- Half suite: `bash space/tests/run.sh`; adapter wiring of `install.sh` additionally proves itself in `bash flow/tests/adapters/run.sh`.

---

## Unit IDs

Units are `space-setup/n`, assigned once and never renumbered. Cite IDs in commits
and tests during impl (`feat(space): space-setup/1 …`).

---

### space-setup/1 — Data layer in repo

**Goal:** The prompt, MCP template, and plugin declarations live in `space/`, secret-free.

**Requirements:** R2, R1

**Dependencies:** —

**Files:**

```
space/AGENTS.md        # moved verbatim from ~/.config/opencode/AGENTS.md
space/mcp.json         # canonical template with ${ENV} placeholders (schema per tech.md)
space/plugins.json     # per-runtime plugin declarations
space/tests/run.sh     # suite seed: secrets scan + JSON validation
```

**Interfaces:**

- Produces: the template schema all later emitters read; the secrets scan all later units extend.

**Test scenarios:**

- Secrets scan over `space/**` finds zero token-shaped literals AND asserts a file-count floor (a scan that examined nothing must fail).
- Both JSON files parse and satisfy the schema in tech.md (server types valid; every credential field is a `${VAR}` reference; serena carries `"optional": true`).
- `space/AGENTS.md` is byte-identical to the current `~/.config/opencode/AGENTS.md` at extraction time.

**Verification:** `bash space/tests/run.sh` green with the scan's population line printed.

---

### space-setup/2 — Apply engine core

**Goal:** `apply.sh` materializes instances from templates, links the prompt, records a manifest; `--dry-run` and re-run stability hold.

**Requirements:** R1, R2, R4

**Dependencies:** space-setup/1

**Files:**

```
space/apply.sh                 # materialize + link + record; --dry-run
space/tests/run.sh             # + materialization, idempotency, dry-run legs
```

**Interfaces:**

- Consumes: `space/mcp.json`, `space/AGENTS.md` (from /1).
- Produces: `apply()` engine, manifest at `$HOME/.config/vibe/space/manifest.json`, materializer used by every adapter unit.

**Test scenarios:**

- Materialize with tokens in env: instance file mode `0600`, real values present, template untouched.
- Missing token: one warning naming the variable, that server skipped, exit 0.
- Re-run with unchanged inputs: byte-identical outputs, no backups, no warnings.
- `--dry-run` on an unprovisioned isolated `$HOME`: plan output names every action, zero writes.
- Differing regular prompt file at a target: backed up, one warning (R1 adoption scenario).

**Verification:** `bash space/tests/run.sh` green; all legs under an isolated `$HOME` from `mktemp -d`.

---

### space-setup/3 — opencode adapter

**Goal:** The hand-built opencode plugins move into the repo and keep working; MCP reach flows through the shared instance; omp-parity keys untouched.

**Requirements:** R3, R7

**Dependencies:** space-setup/2

**Files:**

```
space/adapters/opencode/vibe.ts            # moved from ~/.config/opencode/plugins/vibe.ts
space/adapters/opencode/shared-mcp.ts      # moved; unchanged reading of the materialized instance
space/adapters/opencode/omp-parity/mcp.ts  # moved format library
space/apply.sh                             # + opencode fan-out: plugin symlinks, prompt symlink
```

**Interfaces:**

- Consumes: materializer + manifest (from /2).
- Produces: `~/.config/opencode/plugins/vibe.ts` + `shared-mcp.ts` symlinks into the checkout; prompt symlink at `~/.config/opencode/AGENTS.md`.

**Test scenarios:**

- Post-apply, the plugin files in `~/.config/opencode/plugins/` resolve into the repo checkout; `opencode.json` is byte-unchanged by apply (omp-parity ownership).
- Materialized shared instance equals the golden transformation of the template via the moved format lib (parity leg).
- A fixture repo with `.agents/skills/vibe` present: orders injection path exercised (the moved `vibe.ts` contract held: guarded write blocked, orders emitted).
- If opencode does not auto-load symlinked plugins: fall back to copy + doctor drift check — the test pins whichever mechanism holds, with the fallback documented.

**Verification:** `bash space/tests/run.sh` green; omp-parity byte-parity leg explicit in output.

---

### space-setup/4 — droid adapter

**Goal:** Droid gets prompt, MCP, and teeth: user-scope hooks merged safely, project-scope hooks written by `install.sh --local`, guard and orders fire in droid sessions.

**Requirements:** R3, R1

**Dependencies:** space-setup/2

**Files:**

```
space/adapters/droid/hooks.json.template     # user-scope entries: guard, gate, orders
space/adapters/droid/project-hooks.json      # project-scope template
space/apply.sh                               # + droid fan-out: ~/.factory/hooks.json merge, ~/.factory/mcp.json merge
install.sh                                   # --local gains the .factory/hooks.json step
space/tests/run.sh                           # + merge safety, teeth, discriminating-uninstall legs
```

**Interfaces:**

- Consumes: materializer + manifest (from /2); the existing `.agents/skills/vibe` scripts (read-only).
- Produces: merged `~/.factory/hooks.json` entries (marker-guarded), `.factory/hooks.json` per repo, `~/.factory/mcp.json` server entries from the canonical template.

**Test scenarios:**

- Reversed/overlapping markers in a fixture `hooks.json` leave the file byte-untouched and refuse with an error (marker-pairing lesson applied).
- User-owned entries co-located in `hooks.json` survive apply and uninstall (discriminating leg: blanket removal must fail the test).
- Fixture hook run against a fixture repo: a policy-blocked write is refused with the policy's reason (droid `PreToolUse` deny path); orders context is produced on prompt submit.
- Prompt half: whichever global-prompt mechanism the Factory docs verify (see product.md Open Questions) — or the documented plugin fallback — resolves the shared prompt in a droid session.

**Verification:** `bash space/tests/run.sh` green; `bash flow/tests/adapters/run.sh` green for the install.sh step.

---

### space-setup/5 — codex adapter

**Goal:** Codex reads the shared prompt via a repaired import, and its MCP blocks are emitted from the canonical template.

**Requirements:** R3, R2, R1

**Dependencies:** space-setup/2

**Files:**

```
space/adapters/codex/agents-import.md   # repaired import content (replaces broken @RTK.md-only file)
space/apply.sh                          # + codex fan-out: prompt import, additive TOML emission
space/tests/run.sh                      # + TOML golden, import-resolution legs
```

**Interfaces:**

- Consumes: materializer (from /2).
- Produces: `~/.codex/AGENTS.md` import, `[mcp_servers.<name>]` blocks in `~/.codex/config.toml`.

**Test scenarios:**

- TOML emission from the template is golden-compared; foreign blocks in the fixture config are byte-unchanged; tokens are baked from env at apply time.
- The repaired import resolves the shared prompt from a codex session's perspective (path resolution test; symlink in `~/.codex` if at-imports cannot reach outside it — pinned by test, not assumption).

**Verification:** `bash space/tests/run.sh` green.

---

### space-setup/6 — claude alignment

**Goal:** The existing claude wiring points at the repo sources; user-scope MCP and plugin state become verified facts, not manual residue.

**Requirements:** R3

**Dependencies:** space-setup/2

**Files:**

```
space/apply.sh        # + claude fan-out: AGENTS.md symlink repoint, user-scope MCP add, plugin verify
```

**Interfaces:**

- Consumes: materializer + manifest (from /2); existing `.claude` adapter (read-only).

**Test scenarios:**

- Post-apply, `~/.claude/AGENTS.md` resolves into the repo checkout; the import chain in `~/.claude/CLAUDE.md` still resolves.
- User-scope MCP add is idempotent (second run: already-present → no-op, no error).
- Plugin presence check warns per missing plugin; never attempts a marketplace install without consent.

**Verification:** `bash space/tests/run.sh` green.

---

### space-setup/7 — Front door: install flag, doctor, uninstall, stranger eval

**Goal:** `install.sh --space` delegates to the half; doctor verifies with population assertions; uninstall is the manifest's surgical inverse; a stranger eval from a bare home is the release gate.

**Requirements:** R4, R5, R6

**Dependencies:** space-setup/3, space-setup/4, space-setup/5, space-setup/6

**Files:**

```
space/doctor.sh                       # per-runtime verification, population-asserted
space/apply.sh                        # + --uninstall (manifest-driven)
install.sh                            # --space flag + delegation + uninstall plumbing
space/tests/run.sh                    # + doctor fixtures, uninstall legs, stranger eval
flow/tests/adapters/run.sh            # + install.sh --space wiring test
```

**Interfaces:**

- Consumes: manifest (from /2), all adapter fan-outs (from /3–/6).
- Produces: the one-command bootstrap path; `space/doctor.sh` verdict lines.

**Test scenarios:**

- Doctor on a fully-applied fixture home: exit 0, one verdict line per runtime (population proven); broken-prompt fixture: named non-zero finding; empty registry: finding, never a clean pass.
- Uninstall with planted user files in every shared target: user files survive, shipped entries removed; the test fails if removal is replaced by `rm -rf`.
- Stranger eval: from a bare `mktemp -d` `$HOME`, `curl`-style invocation with `--space` wires every present runtime, advises once per absent runtime, exit 0 — executed, not narrated.

**Verification:** `bash space/tests/run.sh` green; `bash flow/tests/adapters/run.sh` green; stranger eval transcript captured as the release evidence.

---

## Dependencies

| Unit | Blocks | Blocked by |
|---|---|---|
| space-setup/2 | /3, /4, /5, /6, /7 | space-setup/1 |
| space-setup/3 | /7 | space-setup/2 |
| space-setup/4 | /7 | space-setup/2 |
| space-setup/5 | /7 | space-setup/2 |
| space-setup/6 | /7 | space-setup/2 |
| space-setup/7 | — | space-setup/3, /4, /5, /6 |

Same-feature dependencies only. Cross-feature order is a whole-feature gate in the root [plan.md](../../plan.md) Feature Sequence.

---

## Spec vs Implementation

| Gap | Tracked in | Notes |
|---|---|---|
| Droid global-prompt mechanism unverified | space-setup/4 | Open question in [product.md](product.md); hooks/MCP halves proceed regardless |
| opencode symlinked-plugin autoloading unverified | space-setup/3 | Fallback copy + drift check documented in tech.md |

---

## Progress

| Unit | Status |
|---|---|
| space-setup/1 | NOT STARTED |
| space-setup/2 | NOT STARTED |
| space-setup/3 | NOT STARTED |
| space-setup/4 | NOT STARTED |
| space-setup/5 | NOT STARTED |
| space-setup/6 | NOT STARTED |
| space-setup/7 | NOT STARTED |
