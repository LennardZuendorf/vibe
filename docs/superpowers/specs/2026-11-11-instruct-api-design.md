# Instruct API and cross-harness enforcement design

**Status:** Superseded. Active Flow scope: `docs/superpowers/specs/2026-11-11-flow-workflow-design.md`; deferred Instruct/Spec scope: GitHub issues #83–#86. Retained as prior context; do not implement independently.
**Date:** 2026-11-11
**Feature:** instruct-api
**Base:** c2881a73f2872be16971dd864bdfbd607f364215
**Parents:** docs/spec/parts/instruct/instruct.md; docs/spec/tech/instruct.md; docs/spec/concepts/cross-tool-contracts.md
**Related decisions:** D25 hook ownership; D27 writer is the gate; D28 advisory signals; D40 CLI front door; D48 global store.

## Intent and confirmed requirement

Expose Instruct through the existing Rust CLI and give Claude Code, OpenCode and Codex one instruction and enforcement contract. Flow, Spec and future components must use this API without implementing their own host integrations.

The user explicitly requires blocking: an agent may propose coding, but the action must pass the required step and approval checks before execution. A warning and a log entry do not satisfy this requirement.

The design below contains proposed engineering choices. The user has confirmed the blocking requirement, not every choice below.

## Scope and boundaries

In scope: Rust composition, versioned context and policy interfaces, lifecycle routing, managed instruction documents, three adapters, approval/evidence checks, migration and observable compatibility.

Keep Flow authoritative for workflow state, legal transitions and gate requirements. Keep Spec authoritative for spec validation and artifact readiness. Instruct owns context composition, policy dispatch, decision aggregation and delivery. Adapters own host protocol translation only.

One front door: `vibe instruct ...`, dispatching to independently installable `vibe-instruct`. Retain existing peer-binary independence; do not introduce tool-to-tool Rust dependencies. Internal Instruct modules use typed Rust interfaces. External components use versioned JSON. No persistent general-purpose daemon or network server is required for composition/evaluation.

This feature changes D25 by centralizing installed host routing. Domain policy ownership remains separate. Changes to D25 and the Instruct/Codex target documents ship with implementation, not as silently accepted decisions during planning.

Out of scope: remote source catalogs/synchronization from F13, a policy scripting language, automatic semantic judgment that prose meets a spec, a universal OS sandbox, a web management UI, and tamper-proof enforcement against a user or privileged process controlling the host.

## Guarantees

R1. Every intercepted action subject to an active mandatory rule is evaluated synchronously before execution. Denial prevents execution and names the requirement and recovery action.

R2. A required policy failure, missing approval, stale approval, unsupported required capability or indeterminate action never becomes allow. Optional context failures may omit context with a diagnostic.

R3. Approvals bind to a specific project, worktree, workflow run, gate, artifact digest and scope. An agent-supplied `confirm` flag, payload, message or receipt is not approval.

R4. Flow enforces approval and legal edges in its own state writer, even when invoked directly. A host hook is an additional check, not the sole transition boundary.

R5. Decision and instruction content agree across supported adapters for the same normalized inputs. Transport wrappers, prompt role placement and host UI may differ.

R6. Required enforcement is enabled only for tested host capabilities. Installation, trust, runtime activation and demonstrated enforcement are distinct states. Doctor reports each one.

R7. Every agent/subagent has explicit session identity. Approvals are shared only through the bound workflow run and scope. Delivery state is isolated by project/worktree, host, session, agent and context epoch.

R8. Previews never consume events. Resume/compaction re-establish current required context. Delivery uses stable IDs with safe replay; do not promise exactly-once delivery where the host supplies no acknowledgment.

R9. Preserve user-owned prose, configuration, skills, existing store instructions and unrelated hooks. Migration must not create duplicate injections or a temporary loss of mandatory checks.

R10. Policy permission never bypasses native host permissions. An Instruct allow result means that Vibe has no remaining objection; the host still applies its own policy.

## Architecture choice

Considered: composition-only Instruct (duplicates enforcement adapters); shared coordinator (recommended); a monolithic CLI importing every component (breaks independent tool boundaries).

Use the shared coordinator. One host event enters one adapter, which normalizes the event and calls Instruct. Instruct collects context and domain decisions, aggregates them, and returns a typed result. The adapter delivers context and applies the host-native action. Flow and Spec retain standalone entry points backed by the same domain logic.

`vibe-core` may hold generic JSON, atomic files, locks, identity and markers. It must not acquire Flow state names or Spec artifact rules. New Instruct vocabulary lives in `vibe-instruct` and versioned contract files.

## Public API

Human CLI:
- `vibe instruct render <channel> [--json]`
- `vibe instruct write [--check] [--target <target>]`
- `vibe instruct lint`
- `vibe instruct explain <id> --json`
- `vibe instruct doctor --harness <claude|opencode|codex> --json`
- `vibe instruct providers list --json`

Machine CLI:
- `vibe instruct api`: one versioned JSON request on stdin, one JSON response on stdout; no shell expansion.
- `vibe instruct hook --harness <id> --event <native-event>`: adapter ingress, not an agent-facing approval API.
- `vibe-instruct` offers the same operations without the front door.

API v1 operations:
| Operation | Input and ownership | Result / effect |
| --- | --- | --- |
| publish | Namespaced context snapshot/events from a registered provider | Atomic provider snapshot; cannot alter executable policy or approvals |
| render | Channel, normalized context and optional explicit source snapshot | Ordered fragments, provenance, diagnostics and delivery ID; no consumption |
| evaluate | Proposed action and authoritative context | Decision and required recovery; does not execute action |
| record | Observed operation outcome or delivery acknowledgment | Runtime observation; ordinary callers cannot mint approval or trusted test evidence |
| handle | Trusted normalized lifecycle event | Compose/evaluate/record as appropriate |
| write | Explicit target selection and check/apply mode | Managed-document preview/check or atomic update |

Envelope:
`{v:1, request_id, operation, context, data}`.
Context:
`{project_id, worktree_id, run_id?, harness, session_id, agent_id?, parent_agent_id?, turn_id?, context_epoch}`.
Project/worktree identity is resolved from the physical root and registry, never accepted solely from an agent-supplied path. Callers may propose IDs; authoritative resolution checks them.

Response:
`{v:1, request_id, status, result, diagnostics}`, with status `ok|refused|error`.
Policy result:
`{decision, reasons, requirements, policy_revision}`.
Decision:
`allow|warn|deny`; individual providers additionally return `abstain|error`.
Reason:
`{owner, rule_id, message, recovery}`.
Render fragment:
`{id, owner, channel, text, source, revision}`.
A host continuation request is separate from an action denial.

A valid negative decision is still a successfully evaluated JSON response. Process exit 0 means a valid API response, 1 means an operational failure without a valid response, 2 means CLI usage failure. Native hook exit codes are emitted only by adapter translation. Text CLI check commands may exit 1 for findings. This prevents confusing a clap usage exit with a policy block.

Unknown optional fields are ignored within v1; unknown operation, enum, mandatory capability or incompatible major is refused. Resource bounds apply before parsing, provider execution and rendering.

## Providers and policy

Provider manifests declare id, contract major, capabilities, context sources, evaluator argv, timeout, and trust provenance. Commands use argv arrays, never shell interpolation. Repo manifests do not authorize execution; registration requires user trust or an existing allowlist. Fetched content cannot register commands.

Two distinct provider outputs:
- Context: preserve level/edge/session/events and sequence semantics from the existing planned payload contract.
- Policy: structured decision over normalized action and authoritative domain state.

Namespace IDs as `flow.*`, `spec.*`, or a registered component namespace. Publish cannot overwrite another namespace. Provider order is explicit and deterministic. Registering a new component must require only a manifest and contract implementation, with no adapter changes.

Mandatory denies win. Mandatory evaluator errors deny. Missing mandatory evaluators deny. Warnings cannot cancel denies. An optional context provider failure produces a bounded diagnostic. No lower-trust layer may weaken a mandatory rule; instruction text precedence does not imply policy override authority.

Revocation applies to new action authorizations; it cannot undo an action already running. Workflow transitions that narrow permissions must wait for authorized mutating actions to finish or for cancellation to be confirmed. Pending dispatch must be covered by an action lease before the hook returns; a state change cannot race that lease. If action completion/cancellation cannot be established, keep the restrictive transition pending and report recovery. Checkpoint 1 must prove the adapter can identify dispatch/completion and interactive process lifetime sufficiently to support this boundary.

Evaluation consumes a consistent policy/state snapshot. State mutations use exclusive locks before reads, compare expected revision, and invalidate decisions on concurrent changes. Never use a cached context payload as authorization.

## Actions and lifecycle

Normalized events: session.start, session.resume, context.reset, prompt.submit, action.before, action.after, agent.start, agent.finish, turn.finish, session.end.

Normalized action:
`{action_id, kind, tool_name, targets, effect, command?, cwd?, raw_input_digest}`.
Kinds: file.read, file.write, file.delete, file.move, process.start, process.input, workflow.transition, external.mutate, other.
Effect: read_only, mutating, unknown. Targets include all patch edits, deletes and move source/destination paths, resolved against the actual worktree.

When a gate is closed, mutating or unknown actions that can bypass it are denied. Read-only work and the explicit recovery path remain available. Read-only process allowances are exact executable/argument policies with tests, not keyword guesses. A shell string mentioning no guarded filename is not proof of safety. Existing interactive processes cannot remain a write bypass: strict mode must restrict or stop their input/lifetime across a gate change, or mark this capability unsupported.

The first release enforces workflow-controlled workspace changes and transitions. External operations are normalized and available to components, but are not automatically granted business authorization by this feature.

Scope approvals cover classes of intended work so implementation does not ask permission for every edit. Direct file actions receive step/scope/revision checks. A process action may execute only when its write capability is confined to the approved scope by a demonstrated host boundary, or when it matches a demonstrated read-only allowance. This applies after approval too. Hook text scanning does not prove confinement of writes performed inside a process; unknown effects are denied.

## Approval authority

Create a pending gate request from Flow with a random request ID, gate, run identity, exact plan/evidence digest, permitted scope and expiration. Instruct delivers the request through a supported host interaction.

Accept approval only from a verified user-origin response to that pending request. Adapter-maintained trusted ingress supplies origin metadata. Normal API publish/record/evaluate, model text, generated Stop continuations, tool output, shell argv and a plain `--confirm` cannot approve.

Approval storage is runtime state outside committed documents. It is writable through the approval handler, not through general provider recording. Approval references are opaque IDs; the domain writer rechecks the stored grant and current artifact digest under its lock.

Plan or scope changes revoke the affected grant. Repeated requests are idempotent; a replay for another run/worktree/digest fails. Subagents inherit only the parent run's approved scope, never a wider permission.

Host approval UI and Vibe workflow approval remain distinct. A generic tool permission grant, automatic reviewer decision or programmatic plugin reply is not automatically user approval of a plan.

**Mandatory feasibility checkpoint:** before implementing approval enforcement, each adapter must demonstrate a stable way to distinguish an actual user response from synthetic/model-origin input, and that agent tools cannot invoke the privileged approval handler as ordinary API traffic. If a host does not expose this, strict approval cannot be declared supported. Bring the concrete missing capability back for a product decision; do not silently parse arbitrary transcript prose or add a new approval service. This is a go/no-go dependency, not deferred hardening.

Threat model: prevent an agent from skipping workflow through its normal supported tools, including processes it launches with the host user's identity. Those agent-launched processes MUST NOT be excluded as privileged same-user activity. Approval grants, trusted evidence and policy configuration must be protected from those processes through a demonstrated execution boundary. Manual host-owner changes outside the agent, deliberately disabled integration and independently privileged attackers are outside the guarantee. Doctor must not label this tamper-proof.

## Evidence and completion

Flow owns artifact readiness and verification requirements. Spec supplies deterministic validation results. Store observed command identity, exit status, relevant revision/digest, scope and capture origin. A text file with a fresh timestamp is insufficient proof of success.

Trusted capture records what actually ran; arbitrary `record` calls remain untrusted observations. Verification must cover the current changes, and the ship gate remains independent of the plan gate.

Before completion, the coordinator checks required evidence and returns a bounded continuation request with actionable gaps. Stop/idle handling must prevent infinite loops and respect user interruption. A host that cannot veto or continue completion cannot be reported as supporting that capability. Ship/export/transition checks remain enforceable at their actual action boundaries even when a host ends a conversation.

## Composition and durable targets

Port the existing tested composer first: shipped defaults, repo blocks, root vibe.json, summary/body rendering, placeholders, configured channel order and managed writes. Preserve old summary markers; support frontmatter summary only through explicit versioned migration.

Then introduce global, repo-shared, repo-local and runtime context tiers, with provenance. Reuse the existing global store root; do not create a competing root AGENTS.md source. Only repo-shared content goes into committed documents by default. Global/local context must not leak into commits.

Support explicit managed targets AGENTS.md and CLAUDE.md. Preserve symlinks that resolve inside the target root, refuse dangling/outside links, malformed marker pairs and unmanaged collisions. Use the existing shared marker implementation and current contract, not stale hash examples.

Use existing channel defaults: level 2 lines, edge 15, event 10, session 15, agents-md 80; combined per-prompt level+edge+event 20 lines. Legacy standing user-prompt content participates in the total after migration. Optional context may be omitted with diagnostics to meet budget; mandatory state/recovery information must not be silently truncated. Reject impossible configuration at lint/setup. Enforcement never depends on the model receiving every instruction fragment.

Render is pure. handle obtains a delivery plan under lock; adapters acknowledge supported delivery. Without acknowledgment, replay a stable delivery ID rather than losing an event. Deduplication must not suppress level updates, state transitions or required context after compaction. Ledger includes source revisions and context epoch.

## Adapter contract and support

Claude: SessionStart/UserPromptSubmit for context, PreToolUse for checks, PostToolUse for observations, Stop/SubagentStop for bounded continuation. Test command-failure blocking on supported releases; documented onFailure:block requires 2.1.295+. Do not silently rely on the default failure behavior.

Codex: hooks for session/resume/compact, prompts, tools and completion. Trust review is a required installation state. Normalize apply_patch, exec_command-as-Bash, MCP and subagents. Later write_stdin input lacks a fresh native PreToolUse check; strict support must close that route through host restrictions or refuse that capability. Unsupported hook fields can fail open, so output schema fixtures are mandatory.

OpenCode: V2 @opencode/plugin adapter using prompt/context/compaction, tool execute.before/after and permission evaluation. Use experimental.idle only behind a tested capability for bounded continuation. It is not a universal finish veto. Preserve native deny precedence; Instruct allow must not grant a permission.

All three named hosts must pass the mandatory matrix before releasing or claiming parity for this feature. A partial host release or reduced guarantee requires a new user decision. Optional capabilities may differ only when they do not weaken the mandatory checks.

Do not choose minimum versions from documentation alone. Check actual released host behavior in checkpoint 1; pin a support matrix and test fixture versions. Unknown versions begin unverified. Doctor distinguishes available, installed, trusted, active, enforced, degraded and unsupported. Unsupported mandatory capability blocks enabling the protected workflow and provides a recovery path.

## Migration

Inventory legacy Node composition, repository hooks, Spec plugin hooks, global store hooks and bespoke host plugins. Adopt only exact known owned entries. Preview replacements; preserve unknown user plugins and report conflicts.

One coordinator per host event. Built-in Flow/Spec registrations reuse existing domain evaluators. Keep standalone commands operational; when co-installed, disable only owned duplicate routing, not domain checks.

Migration is resumable and locked. Read all persisted markers when resuming. Keep old enforcement active until replacement wiring has passed its activation probe, then switch owned entries atomically where possible. If a host cannot provide an atomic switch, use an overlapping safety phase with deduplicated enforcement; never a gap. Rollback restores only Vibe-owned entries.

No user host configuration is changed during planning.

## Planning readiness

Independent builder review identified four prerequisites: user-origin approval authority, protection from agent-launched shell processes, all-host parity, and dispatch/revocation timing. The defaults above make the intended guarantees explicit. Their implementation feasibility is not yet proven. The accompanying implementation plan therefore details checkpoint 1 only; later checkpoints remain a conditional roadmap. Do not execute the later roadmap as if it were an approved implementation plan.

## Verification and implementation checkpoints

1. Released-host capability and approval-origin proof. Stop if mandatory parity is infeasible.
2. API schemas, fixtures, Rust CLI dispatch and provider boundaries.
3. Composer parity, tiers, budgets, durable targets and delivery ledger.
4. Flow/Spec evaluator integration, approvals and evidence.
5. Three adapters and strict capability checks.
6. Migration, standalone compatibility and complete host parity suite.

Acceptance scenarios:
- Coding before approval: file tools, patches, shell writes and child agents are denied.
- Agent calls confirm or record approval: no grant is created.
- Valid user approval: scoped coding succeeds; unrelated scope remains denied.
- Plan changes or concurrent transition: previous grant/decision is rejected.
- Mandatory provider crashes/times out/returns malformed data: action is denied.
- Optional context provider fails: action policy still runs; a diagnostic is visible.
- Patch contains multiple targets, moves or a symlink escape: every target is checked.
- Existing interactive shell receives later input: no closed-gate bypass.
- Resume/compact/fork/concurrent sessions: current orders return with isolated delivery state.
- Stale or agent-authored verification receipt: completion/ship checks reject it.
- Migration interrupted at each persisted step: no user content loss or enforcement gap.
- External fixture provider registers through the public API and obtains identical normalized results across all adapters without host-specific provider code.

Tests must assert the intended action did not execute, not just that a decision said deny. Run fixture tests plus real host smoke tests on pinned supported versions. Inject fake binaries, missing trust, timeouts and malformed output to verify the failure boundary.

## Sources checked 2026-11-11

- Repository base and contracts listed above; flow/engine/content.mjs and commands/{render,hook}.mjs.
- https://code.claude.com/docs/en/hooks
- https://opencode.ai/docs/build/plugins/server
- https://opencode.ai/docs/permissions/
- https://github.com/anomalyco/opencode/blob/dev/packages/plugin/src/README.md
- https://learn.chatgpt.com/docs/hooks
- https://developers.openai.com/plugins/build/plugins

Development-branch examples are evidence for investigation, not proof of released support.
