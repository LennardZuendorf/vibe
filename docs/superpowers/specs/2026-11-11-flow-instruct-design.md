# Flow skills and Instruct integration design

**Status:** Superseded by `docs/superpowers/specs/2026-11-11-flow-workflow-design.md`. Broad non-Flow work moved to GitHub issues #83–#86. Do not implement this predecessor independently.
**Date:** 2026-11-11
**Feature:** flow-instruct
**Base:** c2881a73f2872be16971dd864bdfbd607f364215
**Plan:** `docs/superpowers/plans/2026-11-11-flow-instruct.md`
**Supersedes:** The proposed `instruct-api` design and probe-only plan of 2026-11-11. Their enforcement feasibility requirements are retained here.
**Parents:** `docs/spec/parts/flow/flow.md`, `docs/spec/parts/instruct/instruct.md`, `docs/spec/parts/spec/spec.md`, `docs/spec/tech/cli.md`.

## Intent and confirmed requirements

Make Vibe a coherent workflow across Claude Code, OpenCode and Codex. A feature request should select one workflow, use the appropriate skills and agents, preserve its progress, and block coding until its required steps and approval are satisfied. All Vibe components should address one extensible Instruct interface.

The user explicitly requests real blocking, a new implementation plan and feature spec, extensive useful agent-skill support, subagents, and a correctly named workflow skill: `flow` or namespaced `vibe:flow`. Multiple unrelated skills named `vibe` are unacceptable.

This document proposes the remaining engineering choices. It does not claim implementation, host activation, approval authority or enforcement parity already exists. The existing isolated worktree is sufficient for implementation after approval; no new worktree is required by this design.

## Current state and scope

The audited source has Rust Flow state/policy/guard/Stop, Node instruction composition, shell compatibility, Rust Spec, and the global-store CLI. There is no Rust Instruct crate. The current writer checks known state names, not edges or human approvals. Receipt freshness does not prove a passing verdict. `/flow` and runtime readers can select different cursor generations. External executor skills own actions that overlap Flow's verify and finish stages.

In scope: skill identity and packaging; rich, bounded skill procedures; dependency resolution; executable workflow requirements; Rust Instruct and front-door dispatch; shared host translation; scoped approvals and evidence; session isolation; migration; behavioral evaluation and diagnostics.

Out of scope: a general workflow programming language, remote instruction catalogs, TUI, a new daemon, autonomous release authorization, an OS security sandbox, and tamper resistance against the human controlling the machine. Agent-launched processes remain in the enforcement threat model. Their writes cannot be dismissed as out of scope.

Existing F7/F8 and F11–F15 overlap this work. The implementation must reconcile their owning notes rather than introduce a competing roadmap. F13 remote source synchronization remains deferred. D25 hook ownership and D27 approval semantics require explicit amendments; D12 single-source orders, D40 binary independence and D48 global-store ownership remain constraints.

## Acceptance requirements

| ID | Required outcome |
| --- | --- |
| R1 | Canonical skill folder and frontmatter are `flow`; plugin identity remains `vibe`, exposing `vibe:flow` where namespaces are supported. No advertised `vibe` workflow skill remains after a successful owned migration. |
| R2 | Three public capabilities are distinct: `flow`, `spec`, `instruct`. `using-spec` remains a narrow read-only bootstrap helper. Selection, procedures, references and agent briefs work from installed package locations. |
| R3 | Flow owns progression and exactly two human gates for feature implementation. Setup/strategy exploration and planning cannot authorize implementation or release. Workers and external methods cannot advance state, mint approval, commit or finish branches implicitly. |
| R4 | Required step, scope and approval checks run before supported mutating actions. Missing, stale or indeterminate mandatory checks deny with a reason and recovery action. |
| R5 | A transition validates the legal edge, run identity, expected revision, prerequisites and applicable approval atomically. Direct CLI calls cannot bypass this check. |
| R6 | Verification records actual checks and independent review against the full candidate, including uncommitted/untracked files and prepared documentation. A fresh failed receipt is not success. |
| R7 | Flow, Spec and future providers use the same versioned API; adding a provider needs no host-adapter changes. Independent binaries do not import peer crates. |
| R8 | Identical normalized inputs produce equivalent decisions and instruction content on all three hosts. Unsupported mandatory capabilities prevent an enforced-parity claim. |
| R9 | Instructions survive resume, compaction and delegation without cross-session consumption. Rendering a preview does not mark content delivered. |
| R10 | Migration preserves user content, runtime state, unrelated hooks and foreign skills. Source/version selection and duplicate registrations are observable. |
| R11 | Skill behavior is tested with independent realistic scenarios, alongside structural checks and executable integration tests. Passing lint alone is insufficient. |
| R12 | Final ship approval binds the reviewed candidate and a named action. Content changes invalidate readiness. Commit, push, merge and deployment require the authorization appropriate to that action. |

## Skill names, sources and discovery

| Capability | Canonical source | Public invocation | Boundary |
| --- | --- | --- | --- |
| Flow | `flow/SKILL.md`, `name: flow` | `flow`; `vibe:flow` in namespaced hosts | Request routing, lifecycle, bounded delegation and recovery |
| Spec | `spec/SKILL.md`, `name: spec` | `spec`; `vibe:spec` | Spec discovery, artifact ownership, validation and promotion |
| Instruct | New `instruct/SKILL.md`, `name: instruct` | `instruct`; `vibe:instruct` | Explicit instruction inspection, rendering, managed writes, provider and host diagnosis |
| Bootstrap | `spec/using-spec/SKILL.md`, `name: using-spec` | Helper; no competing feature entrypoint | Read governing spec/context, then yield to one owner |

Do not put `vibe:flow` in portable YAML `name`; the host supplies its plugin namespace. Rename directory-based registrations as well as frontmatter. Keep the package brand `vibe`, CLI `vibe`, `.vibe/run/flow`, and frozen oracles unchanged.

New discovery paths use `.agents/skills/flow`, `.claude/skills/flow`, `plugin/skills/flow`, `plugins/vibe/skills/flow` and `factory-plugin/skills/flow`. Legacy data under `.agents/skills/vibe` is migration input, not a second advertised skill. Preserve legacy bytes before changing symlinks. Retain old command redirects only where they invoke the single new implementation without registering another skill body.

A resolver records capability, provider ID, version, source path, content digest, availability and shadowed candidates. Explicit user selection wins. Otherwise select the managed configured provider; same-digest copies may be deduplicated logically. Differing unmanaged providers produce a visible conflict; do not silently delete or rewrite them. Do not modify third-party plugin caches directly. Doctor must distinguish available files from host discovery and successful activation.

Keep `using-spec` non-user-invocable in projections that support it. On other hosts, its name and description must clearly state its bootstrap-only role. Flow invokes Spec for artifact operations; Spec does not recursively launch Flow when already executing a Flow-owned operation. Use a bounded invocation context with owner, run, task and requested operation, not untrusted prose as authorization.

## Rich skills through progressive disclosure

The Flow entrypoint holds selection triggers, current-state lookup, ownership, gate invariants, and short state orders. It links only the procedure needed now. Keep D12: one authored home for state orders. The machine names the procedure section; adapters and generated instructions do not duplicate method prose.

Proposed supporting files, created only with their implemented consumer:

```text
flow/
  SKILL.md
  procedures/{feature,quick,setup,strategy}.md
  reference/{task-contract,review-contract,authorization,compatibility,recovery}.md
  agents/{openai.yaml,code-explorer.md,code-architect.md,implementer.md,code-reviewer.md}
  state-machine.json
spec/
  SKILL.md
  using-spec/SKILL.md
  agents/{openai.yaml,spec-promoter.md,spec-health.md}
instruct/
  SKILL.md
  reference/{providers,channels,diagnostics}.md
  agents/openai.yaml
```

Reuse the existing `reference/` convention and useful templates/scripts. Do not create a skill per phase, copy entire external manuals, or add placeholder resources. Deterministic mechanics belong in Rust; scripts are thin launch/compatibility helpers where required. Commands advertised by a skill must exist in that release.

Each procedure specifies prerequisites, required inputs, meaningful decisions, write surface, completion evidence, failure routes and a concrete example. Feature procedures cover exploration, design alternatives, planning, implementation, per-task review, compound preparation, final review and ship. Quick uses the controlling threshold: at most two files, twenty changed lines, and no interface/schema/contract change. A larger task routes back to feature planning; a new flag cannot be Quick merely because it is short.

Agent briefs carry role, scope, dependencies, exact inputs, allowed outputs and return statuses: `done`, `needs_context`, `blocked`, `failed`. They are portable prompts, not proof of native agent registration. Native host agents may project them; otherwise the controller passes the brief to a generic subagent. Resolve capability separately from model; preserve the user's explicit model override.

Portable skill bodies use supported common fields. Codex UI metadata belongs in `agents/openai.yaml`. Claude forked execution, dynamic context, tool allowlists and skill hook fields are host extensions, used only in a tested projection where they improve the task. OpenCode discovery and metadata behavior need installed-version tests. None of these metadata fields substitutes for enforcement.

## Lifecycle and bounded delegation

Target feature implementation sequence:

`design → plan → [approve design+plan] → impl → compound → verify → [approve candidate+action] → ship → idle`

Here `compound` means reversible candidate preparation, never release. It promotes reviewed durable facts, prepares lessons/rules and removes or archives feature artifacts only after an immutable run snapshot preserves their approval/evidence identity. Final verification covers this complete candidate. The ship gate replaces the current verify→compound gate. Standalone Spec promotion retains its approval behavior; Flow-owned promotion is staged in the isolated candidate and included in the final approval, with no extra human gate.

Feature legal edges:
- design → plan or idle; plan → design, impl or idle.
- impl → compound, plan, design or idle.
- compound → verify, impl, plan or idle.
- verify → ship, compound, impl, plan or idle.
- ship → idle only after the named authorized action succeeds; a failed action remains pending with an explicit recovery path.

Only plan→impl and verify→ship are human gates. All edges also check machine prerequisites. A return to design/plan invalidates affected implementation approval. Defects within approved scope return to impl; final candidate readiness becomes stale whenever content changes. `idle` aborts future workflow permissions; restrictive transitions must handle active actions as described below. Corrupt state is an explicit recovery condition, not implicit idle.

Setup detection and strategy drafting remain planning operations. Non-Quick setup mutations enter `feature.design` with their setup findings, then use the same approved implementation path; add that exit to setup.detect. Strategy outcomes enter feature.design when the user requests implementation; add that exit to strategy.spec. Do not turn a strategy discussion or setup diagnosis into permission to apply changes. The legacy setup.apply state remains readable for migration but cannot provide a new ungated mutation path in v2; resume it through planning. Explicitly requested spec scaffolding remains document preparation, not product implementation.

Flow owns its task executor. It may use Superpowers brainstorming, planning, TDD, debugging and review techniques through a tested compatibility contract. Do not invoke complete external executors that assume automatic commits, finish branches or remove the workspace. Build a native loop using those bounded methods. The user receives the design and plan together at gate one; review during design is dialogue, not an extra mandatory approval stop.

Each TaskBrief contains `task_id`, `run_id`, `plan_digest`, `requirements`, `read_paths`, `write_scope`, `dependencies`, `checks`, `role` and `return_contract`. TaskResult contains `task_id`, `status`, `changed_paths`, `check_results`, `findings`, `artifacts` and `candidate_digest`. Only the controller advances the ledger after checking these outputs. Worker reports are claims until their evidence is checked.

Parallelize independent research and disjoint implementation tasks. Shared-file dependencies serialize. Preserve completed tasks on resume; redispatch only stale or incomplete work. Use risk-based task review and one independent review of the entire final candidate. Self-review may be reported, but cannot satisfy an explicitly mandatory independent-review requirement.

## Runtime ownership and storage

Flow owns machine, runs, task ledger, transition prerequisites and approval validity. Spec owns artifact discovery, validation and promotion. Instruct owns content composition, provider dispatch, policy aggregation and delivery. Adapters normalize events and translate decisions. The front door dispatches binaries. `vibe-core` remains generic; reuse its lock and atomic-file primitives.

Keep `.vibe/run/flow/state.json` as the single authoritative atomic cursor. Preserve `flow`, `phase`, `feature`, `updated`; add `v:2`, `run_id`, `revision`, `plan_digest`, `scope_digest`, `task_state` and `evidence_refs`. Store immutable artifacts/results under `.vibe/run/flow/runs/<run_id>/`; publish and sync them before atomically committing cursor references. A crash may leave unreferenced artifacts, never an approved cursor pointing to unpublished evidence. Do not use an independently mutable revision sidecar. Upgrade every writer before enabling enforcement; legacy writers must refuse a v2 runtime. Do not add new security meaning to old receipts.

Instruct delivery state lives under `.vibe/run/instruct/`, keyed by project/worktree/host/session/agent/context epoch. Existing global store paths remain authoritative for global configuration. Approval trust records and verified evidence must use a demonstrated boundary unavailable to agent modification; a project JSON file and a checksum alone do not provide that boundary. Its physical location and native authority mechanism are fixed by the feasibility checkpoint, not guessed here.

## Instruct API v1

Add independently installable `vibe-instruct` and `vibe instruct` forwarding. Also add `vibe flow` forwarding so wrappers use one supported entrypoint. No peer-crate imports or persistent service is required for context composition.

Human operations: `render <channel> [--json]`, `write [--check] [--target <target>]`, `lint`, `explain <id> --json`, `providers list --json`, `doctor --harness <id> --json`.

Machine operation: `api`, one JSON request on stdin and one JSON response on stdout. Envelope: `{v:1, request_id, operation, context, data}`. Context: `{project_id, worktree_id, run_id, harness, session_id, agent_id, parent_agent_id, turn_id, context_epoch}`; inapplicable values are null. Identity is checked against authoritative physical-root/registry/session resolution, never trusted solely from input.

Response: `{v:1, request_id, status, result, diagnostics}`; `status=ok|refused|error`.

| Operation | Effect |
| --- | --- |
| publish | Namespaced context snapshot or event from a registered provider; cannot change policy/approval |
| render | Budgeted ordered fragments with provenance and delivery ID; no consumption |
| evaluate | Evaluate a proposed action using authoritative domain state; does not execute it |
| record | Delivery acknowledgment or observed outcome; cannot mint approval or trusted test evidence |
| handle | Trusted adapter lifecycle ingress; compose/evaluate/record as applicable |
| write | Preview/check/apply managed document regions with explicit ownership |

Decision result: `{decision, reasons, requirements, policy_revision}`; decision `allow|warn|deny`. Reason: `{owner, rule_id, message, recovery}`. Providers additionally return `abstain|error`. Render fragment: `{id, owner, channel, text, source, revision}`. Required deny/error/missing capability dominates; optional context failures may degrade visibly. Host permissions still apply after an Instruct allow.

ProviderManifest: `{v, id, contract_major, capabilities, context_sources, evaluator_argv, timeout_ms, trust_ref}`. Execution requires trusted registration; a repository manifest or fetched text cannot authorize a command. Use argv arrays, no shell interpolation. Resource limits and deterministic ordering apply. Unknown optional fields are additive; unknown versions, operations, mandatory capabilities or decision enums refuse. Add contracts to the existing validator, not a parallel schema engine.

API exit 0 means a valid response, including a structured refusal; 1 means operational failure without a valid response; 2 means usage failure. Native hook statuses are translated separately. This deliberately preserves the distinction from existing Flow CLI usage status 1; wrappers must not confuse usage errors with a policy decision.

Context composition preserves existing tiers, budgets, channels, marker ownership, block precedence and source provenance. Only approved repo-shared content reaches committed AGENTS.md/CLAUDE.md projections; do not leak global, local or session content into them. Resolve shared symlink targets once and preserve user-owned prose. Frozen Node/bash oracles establish compatible behavior; security changes use explicit new expectations rather than rewriting the oracle.

## Approvals, evidence and host action boundaries

Normalized Action: `{action_id, kind, tool_name, targets, effect, command, cwd, raw_input_digest}`. Kinds include file read/write/delete/move, process start/input, workflow transition, external mutation and other. Effect is `read_only|mutating|unknown`. Normalize every patch target and both sides of moves; resolve physical paths and escapes.

A gate request binds `request_id`, project/worktree/run, gate, artifact digest, scope digest and expected revision. An authenticated human response creates a grant for that binding. Agent text, a `--confirm` flag, a synthetic continuation, an ordinary API call or a worker report cannot grant it. No agent-accessible approval command is accepted as proof of user origin. The native trusted boundary is a checkpoint outcome.

TransitionRequest: `{v:1, run_id, expected_revision, target, feature, approval_ref, candidate_digest, action_ref}`; nullable fields are required only for relevant edges. TransitionResult: `{v:1, status, revision, cursor, reasons}`. Lock before reading; validate one consistent snapshot; reject stale revision; publish immutable records before committing the cursor; recover deterministically. No `--force` bypass.

EvidenceRecord: `{v:1, evidence_id, run_id, task_ids, candidate_digest, source, checks, review, verdict}`. CheckResult: `{id, argv, cwd, exit_code, output_digest, output_path, started_at, finished_at}`. Verdict is `passed|gaps_found|human_needed`. Independent review records reviewer identity, role and candidate digest. Trusted collectors distinguish executed observations from agent-authored claims. Reports bind to the tested bytes, not file timestamps or HEAD alone.

At a closed implementation gate, deny implementation mutations and unknown effects; do not prohibit the work needed to reach approval. The current state's narrow artifact scope permits normalized writes to the active design/plan and explicitly requested spec scaffolding. Reads and exploration remain available. Planning permission never includes product source, arbitrary configuration, grants or trusted evidence. A trusted runtime operation may update its own cursor/records through validated commands; it is not a general file-write exemption. Raw agent writes to runtime authority files remain denied. Evidence collection uses the separately trusted collector; agent-authored notes stay untrusted.

Re-entry to design/plan invalidates the affected approval before artifact edits are permitted. During impl the approved design/plan are frozen; new requirements require that re-entry. Add tests for permitted plan revision and denied neighboring source/runtime edits, both directly and through process tools. Approved process execution still needs demonstrated confinement to the current phase's permitted write scope. Shell regexes are diagnostics, not proof of confinement. Interactive input, subprocesses, subagents and in-flight commands are explicit probe cases. Revocation blocks new authorizations; restrictive transitions wait for active mutations to finish or cancellation to be confirmed. Action dispatch/completion must be bound to leases or an equivalent proven host primitive so state cannot race pending dispatch.

Native approval UI, stop hooks and generic permission prompts are not assumed equivalent. User interruption always remains available. Completion prompting is bounded and never treated as authenticated user input. Required context should be restored on startup/resume/compaction and delegation; previews do not consume events and no exactly-once claim is made without acknowledgment support.

## Feasibility checkpoint and delivery sequence

Safe skill naming, method improvements and composition can be implemented without claiming full enforcement. Mandatory three-host enforcement remains conditional on isolated live probes for human-origin approval, protected records, pre-action denial, scoped shell writes, interactive input, subagent behavior, completion/cancellation and fail-closed handling.

Probe reports separate `fixture` and `live`, with result `passed|failed|unsupported|unverified`. Record host version, binary/config digests, exact isolated launch settings and side-effect evidence. A log saying deny is insufficient if the forbidden file exists. Refuse tests that cannot isolate the user's normal profile. Do not install into live profiles, fake human responses, or bypass native trust prompts.

If a mandatory host primitive is absent, stop only dependent enforcement work. Continue independently approved skill/composition work, clearly labeled. Return the concrete gap for a design decision; do not silently add a broker, weaken the guarantee or exclude a host. All three passing live reports are required before supported parity.

Delivery checkpoints: A skill identity and methods; B contracts/composition and isolated capability proofs; C Flow/Spec enforcement using proven authority; D adapters, migration and real-host acceptance. Later production enforcement tasks are conditional until checkpoint B fixes the authority mechanism and scope boundary.

## Migration, diagnostics and release

Freeze current implementations before replacing them. Inventory old/new skills, commands, hooks, store projections, cursor/evidence locations and ownership markers. Rename owned discovery entries transactionally; resolve collisions without overwriting user files. Preserve legacy runtime data and recover interrupted migrations. Test old-only/new-only/both/invalid-current layouts. A missing stronger runtime must not fall back to a writer that bypasses mandatory gates.

Avoid dual injection: one installed coordinator per native event, routing to standalone domain evaluators. Preserve unrelated hooks; deduplicate only owned entries. Preview changes before applying. A rollback cannot restore unsafe enforcement while reporting supported status. Stale third-party packages require normal package-management remediation, not cache surgery.

Doctor reports provider source/version/digest, duplicate skills, resolved methods, installed/trusted/active hooks, supported events, last behavioral probe and unsupported guarantees. Required method failure is visible; optional method fallback is explicit. Dependency declarations separate a technique capability from its provider and model. Use tested version ranges and reject unknown incompatible methods for required stages.

## Verification and publication criteria

Trace R1–R12 to implementation tasks and fixtures. Test structural discovery, installed relative resources, method pressure scenarios, schema boundaries, runtime transitions, actual forbidden side effects, concurrency/recovery and full-candidate review. Keep model evaluations separate from deterministic test results; record model, inputs, output and observed behavior. Missing live-host results remain unverified.

Final candidate includes owning documentation changes, generated artifacts and amended decisions. Run affected tests, the Rust workspace suite, umbrella suite, strict Spec validation and drift checks. Obtain independent review of exact candidate bytes, then present the candidate plus named ship action at gate two. No commit, publication or host rollout is authorized by writing this design.
