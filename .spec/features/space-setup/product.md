---
type: feature-product
feature: space-setup
sibling: tech.md
parent: ../../product.md
updated: 2026-09-28
---

# Feature: space-setup — Product

A `space/` half in vibe: the versioned, cross-runtime personal environment. One
idempotent apply distributes the shared global prompt, the canonical MCP set, and
the per-runtime plugin set to opencode, droid, Claude Code, codex, and omp, so a
new coding space (fresh machine or fresh repo) reaches full working parity in one
command. The hand-built, unversioned adapters that exist today (the opencode vibe
plugin, the shared MCP file and its format library) move into the repo.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | `space/` half: prompt source, canonical MCP template, plugin declarations, per-runtime adapters (opencode, droid, claude, codex), `apply.sh`, `doctor.sh`; the `install.sh --space` flag; the materialized per-user instances (prompt symlinks, the shared MCP instance, hook registrations, codex TOML blocks); drift fixes (codex prompt import, droid prompt wiring) |
| **Does not own** | Instruction content authoring and source sync (instruct half: F11–F15, C4, C5); flow machinery internals (`.agents/skills/vibe` scripts — space only wires hooks that call them); vibe's own plugin packaging (F21); secret storage (tokens stay in the environment, never in the repo); model/provider routing (omp-parity keeps owning model roles); installing the runtimes themselves |

---

## Requirements

### Requirement: Single-source prompt distribution (R1)

The system SHALL hold the global prompt once in the repo and distribute it to every
runtime through per-runtime indirection (symlink or import). The apply step MUST NOT
write divergent copies of the prompt into any runtime's config.

#### Scenario: apply wires all present runtimes

- **Given** a machine with opencode, claude, and codex configured, and no space apply yet run
- **When** the space apply runs
- **Then** every present runtime resolves the same prompt source, and no runtime holds an unlinked copy

#### Scenario: re-apply is content-stable

- **Given** a previous apply, and the prompt edited in the repo
- **When** the apply runs again
- **Then** each runtime's indirection resolves the edited prompt with no stale copies left behind

#### Scenario: existing unlinked prompt is adopted, not silently destroyed

- **Given** a runtime whose prompt file is a regular file with content that differs from the repo source
- **When** the apply runs
- **Then** the existing file is backed up beside the new indirection and one warning names the backup

### Requirement: Canonical MCP declaration, no committed secrets (R2)

The canonical MCP template SHALL declare the default server set (exa, context7, serena
flagged optional) with environment-variable placeholders for credentials. Committed
files MUST contain no literal credential values. The apply step MUST materialize the
per-runtime formats (shared instance, codex TOML, claude, droid) from this one
template, filling placeholders from the environment at apply time.

#### Scenario: materialization with tokens present

- **Given** the template and the required token variables set in the environment
- **When** the apply runs
- **Then** materialized credential-bearing files are created with owner-only permissions and contain the real values, while repo files still contain only placeholders

#### Scenario: missing token degrades, never fails

- **Given** one required token variable absent from the environment
- **When** the apply runs
- **Then** that server is skipped with exactly one warning line naming the missing variable, and the apply exits 0

#### Scenario: optional server stays opt-in

- **Given** the template with serena flagged optional
- **When** the apply runs without an explicit request for serena
- **Then** serena is materialized disabled, and an explicit request materializes it enabled

### Requirement: Per-runtime adapters, teeth included (R3)

The feature MUST ship adapters for opencode, droid, claude, and codex covering prompt,
MCP, plugins, and hook wiring where the runtime supports it. The existing hand-built
opencode adapter (hook-trio port, shared-MCP plugin, format library) MUST move into
the repo and keep working. Droid MUST gain the same teeth the Claude adapter has: the
flow guard and stop gate fire in droid sessions against the same runtime-neutral flow
scripts.

#### Scenario: droid session has teeth

- **Given** a repo with a vibe local install and the droid hooks wired
- **When** a droid session attempts a write the flow policy blocks
- **Then** the write is refused with the policy's stated reason

#### Scenario: opencode adapter survives the move

- **Given** the opencode plugins previously maintained by hand, now in the repo
- **When** the apply runs on the machine that had the hand-built versions
- **Then** opencode loads the moved plugins and per-turn orders injection still fires

#### Scenario: absent runtime is advice, not error

- **Given** a machine without one of the supported runtimes
- **When** the apply runs
- **Then** the absent runtime gets one advice line, and every present runtime is still wired; exit 0

### Requirement: Idempotent apply, surgical uninstall (R4)

The apply MUST be re-runnable with no effect on an already-provisioned machine.
The uninstall MUST remove only what the apply created and MUST preserve all
neighbor-owned content (other config keys, co-located user files, other tools' hook
entries).

#### Scenario: re-run changes nothing

- **Given** a completed apply
- **When** the apply runs a second time with unchanged inputs
- **Then** the machine state is byte-identical and no backup or warning is produced

#### Scenario: uninstall spares neighbors

- **Given** an applied machine where hook files and config directories also hold user-owned and other-tool entries
- **When** the uninstall runs
- **Then** only the paths the apply created are removed; user-owned entries survive — and the test must fail if the uninstall is replaced by a blanket directory removal

#### Scenario: dry-run describes without touching

- **Given** an unprovisioned machine
- **When** the apply runs with `--dry-run`
- **Then** every action is described in the plan output and nothing is written

### Requirement: One-command fresh-machine bootstrap (R5)

The space apply MUST be reachable through the existing one-command installer
bootstrap, and MUST be tested from a representative fresh target (an isolated
temporary home with no prior config), not only from a machine that already ran
the setup by hand.

#### Scenario: stranger bootstrap

- **Given** a bare environment with only the installer reachable and tokens in the environment
- **When** the one-command install runs with the space flag
- **Then** every runtime present in that environment is fully wired, and each absent runtime yields one advice line; exit 0

### Requirement: Doctor asserts its own population (R6)

The space doctor MUST verify per runtime that the prompt indirection resolves, the
MCP servers are materialized, the plugins are present, and the teeth are wired. It
MUST fail loudly when it examined nothing: an empty or absent runtime registry is a
finding, never a clean pass.

#### Scenario: clean tree proves it checked

- **Given** a fully applied machine
- **When** the doctor runs
- **Then** it exits 0 with one per-runtime verdict line, proving at least one runtime was examined

#### Scenario: broken wiring is a named finding

- **Given** a machine with a broken prompt symlink
- **When** the doctor runs
- **Then** it exits non-zero with a finding that names the runtime and the broken artifact

### Requirement: omp-parity coexistence (R7)

The apply MUST NOT modify opencode config keys owned by omp-parity (model roles,
providers, its plugin set). MCP reach into opencode flows through the shared-MCP
plugin reading the materialized shared MCP instance, so the two tools never write
the same keys.

#### Scenario: both tools keep their keys

- **Given** a machine where omp-parity manages opencode config
- **When** the space apply runs
- **Then** omp-parity-managed keys are byte-unchanged and the shared MCP instance carries the space servers

---

## Outputs

- The `space/` half in the repo: prompt source, MCP template, plugin declarations, adapters, apply, doctor, uninstall.
- Materialized per-user instances on each machine: prompt indirections, the shared MCP instance (owner-only permissions), hook registrations, per-runtime MCP blocks.
- An apply manifest recording every created path, consumed by uninstall and doctor.

## Non-Goals

- No secret management beyond environment fill: no vaults, no rotation logic; token rotation means edit environment and re-run apply.
- No runtime installation: the runtimes (opencode, droid, claude, codex, omp) must already exist; the apply advises, never installs them.
- No instruction-content features: block authoring, tiers, budgets, and source sync stay with the instruct half.

## Open Questions

1. **Droid global-prompt mechanism** — droid reads project-level instruction files, but the user-scope mechanism for a global prompt is unverified (candidates: a user instruction file, or instructions carried by the droid plugin). Hooks are unaffected — this blocks only the prompt half of the droid adapter. Verify against Factory docs during impl; the hook and MCP halves of unit `space-setup/4` proceed regardless.
