# Instruct API — enforcement feasibility implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to execute this plan task-by-task. Steps use checkbox syntax. Implementation requires explicit user approval of this plan.

**Goal:** Prove the mandatory cross-host enforcement boundary before committing to the full Rust Instruct implementation.
**Architecture:** Build disposable, isolated host probes that capture the same normalized evidence from Claude Code, OpenCode and Codex. Do not replace production integrations. Use their results to finalize the shared API and the later implementation plan.
**Tech Stack:** Existing Rust workspace; Node built-ins for the portable probe runner; installed host CLIs/plugins; JSON fixtures; existing contract validator. No new third-party dependencies.
**Spec:** `docs/superpowers/specs/2026-11-11-instruct-api-design.md`
**Base:** c2881a73f2872be16971dd864bdfbd607f364215
**Status:** Superseded. Active Flow scope: `docs/superpowers/plans/2026-11-11-flow-workflow.md`; deferred Instruct/Spec scope: GitHub issues #83–#86. Retained as prior context; do not implement independently.
**Feature:** instruct-api

## Global constraints

- User requirement: block actions until required workflow step and approval checks pass.
- All three hosts must pass mandatory probes before claiming parity.
- Agent-launched shell processes are in scope; never exclude them as privileged same-user activity.
- Approval must originate from a verified user response bound to request/run/worktree/gate/artifact/scope.
- Agent text, tool output, synthetic continuations, ordinary API calls and confirm flags cannot grant approval.
- Unknown effects or unavailable mandatory checks deny; optional context may degrade visibly.
- Approved processes still need demonstrated confinement to the approved write scope.
- Revocation applies to new authorizations. Restrictive transitions wait for active mutations to finish or cancellation to be confirmed.
- No production changes, permanent host installs, global settings edits, provider registration or dependency installation in this checkpoint.
- Keep user-owned runtime configuration separate. Run live probes only against disposable projects and isolated host profiles.
- Do not commit unless explicitly requested in the current conversation.
- Never infer a pass from documentation or a simulated host. Keep fixture and real-host results separate.
- Do not add a broker, daemon, wrapper, hosted service or reduced enforcement guarantee to rescue a failed probe without a new design decision.
- No changes to the other active Spec/MCP planning workspace or its cursor.

## File map

New probe files are proposed implementation paths, not existing repository files:
- tests/instruct-host-probes/schema.json — evidence report schema.
- tests/instruct-host-probes/report.mjs — report validation and deterministic comparison.
- tests/instruct-host-probes/report.test.mjs — discriminating report tests.
- tests/instruct-host-probes/runner.mjs — disposable project/profile lifecycle and probe orchestration.
- tests/instruct-host-probes/adapters/{claude,opencode,codex}.mjs — probe-only host launch and trace normalization.
- tests/instruct-host-probes/cases/{approval,actions,lifecycle}.mjs — shared cases.
- tests/instruct-host-probes/fixtures/ — valid reports, one-field mutations and native event fixtures.
- tests/instruct-host-probes/run.mjs — human entry point, explicit fixture/live modes.
- .vibe/run/instruct-probes/<run_id>/ — ignored runtime reports/traces; create no second tracked status store.
- docs/superpowers/specs/2026-11-11-instruct-api-design.md — incorporate proven boundaries.
- docs/superpowers/plans/2026-11-11-instruct-api.md — record checkpoint outcome; expand only after proofs.
- .gitignore — add the specific runtime prefix only if current patterns do not already cover it.

Host adapter launch arguments are chosen from the detected released host's documented isolation options and recorded in the report. Refuse live mode if isolation cannot be demonstrated. Choosing those arguments is part of Task 1, not permission to reuse a normal user profile.

## Interfaces

Use these exact JSON fields across all probe modules:
- Report: `{v:1, run_id, base_sha, mode, started_at, hosts, results}`.
- Host: `{id, version, binary_digest, profile_root, configuration_digest, isolation_verified}`.
- Result: `{host, case_id, status, evidence_paths, expected, observed, limitation}`.
- mode: `fixture|live`.
- status: `passed|failed|unsupported|unverified`.
- Event: `{host, run_id, session_id, agent_id, action_id, event, origin, policy_revision, input_digest, output_digest}`; unavailable nullable values remain null, not fabricated.
- origin: `user|agent|synthetic|unknown`. Only host evidence may establish user.
- ProbeContext: `{host, runId, projectRoot, profileRoot, evidenceRoot, abortSignal}`.
- ProbeResult: the Result shape above.
- `validateReport(report) -> {errors: string[]}`.
- `parityVerdict(report) -> {supported: boolean, blockers: string[]}`.
- `runCase(ctx, caseDefinition) -> Promise<ProbeResult>`.
- Adapter: `detect() -> Promise<Host>`, `prepare(ctx) -> Promise<void>`, `run(ctx, caseDefinition) -> Promise<ProbeResult>`, `cleanup(ctx) -> Promise<void>`.

Functions are JavaScript exports with the documented shapes. No production evaluator or approval service is introduced here.

## Review focus

- A normal-looking tool denial followed by an actual write: check side effects, not just messages (Task 3).
- User-origin labels fabricated from a synthetic prompt or forged record (Task 2).
- A shell process writing grants or evidence after a valid plan approval (Tasks 2 and 3).
- A command already running when the workflow scope changes (Task 3).
- An untrusted/missing hook or an unknown host version reported as supported (Tasks 1 and 4).

### Task 1: Build the isolated probe runner and evidence contract

**Files:** Create schema.json, report.mjs, report.test.mjs, runner.mjs, run.mjs and adapters/*.mjs under tests/instruct-host-probes/.
**Consumes:** Design R1–R10 and released host documentation.
**Produces:** The Interfaces above; pinned host inventory; fixture runner; isolated live launch descriptors.

- [ ] Write report tests for a valid three-host live report, unknown host, missing origin evidence, fixture-only pass, missing result and unverified isolation. Mutate one field per invalid fixture.
- [ ] Run `node --test tests/instruct-host-probes/report.test.mjs`; confirm the new assertions fail before the validator exists.
- [ ] Implement report validation, deterministic ordering and `parityVerdict`. It returns supported only for live reports containing every mandatory passing case for all three hosts with verified isolation.
- [ ] Implement detect/prepare/run/cleanup using documented per-version isolation. Capture version, binary/configuration digests and exact launch arguments. Reject profiles resolving to the user's normal configuration.
- [ ] Implement `run.mjs --mode fixture` and `--mode live --harness <id> --case <id>`. Live mode prints the test purpose and expected user interaction; never bypass host trust or permissions. Missing authentication/trust records unverified.
- [ ] Run the report tests and fixture runner. Expected: zero failed tests; a fixture-only report always yields supported=false.
- [ ] Inventory released installed host capabilities without launching normal user sessions. Record native event names and the available approval, confinement, process-lifetime and completion boundaries.

### Task 2: Prove user approval authority and protected evidence

**Files:** Create cases/approval.mjs and native approval fixtures; extend adapters/*.mjs and report.test.mjs.
**Consumes:** Task 1 runner and Event contract.
**Produces:** Approval-origin and protected-store evidence for each host; no production grant implementation.

Mandatory case IDs:
- approval.actual_user: a real user responds to one specific pending benign request; origin and request binding are observable.
- approval.synthetic: equivalent model text, tool output and a synthetic continuation cannot be classified as user approval.
- approval.forged_api: an agent-controlled command/ordinary API payload cannot invoke the privileged approval route.
- approval.storage: agent-launched processes cannot modify the proposed grant, policy or trusted-evidence store, before or after scoped approval.
- approval.replay: a captured response does not authorize a different run/worktree/digest/scope.

- [ ] Add fixtures where every forged case claims success in its log but lacks valid origin or has a changed binding. Assert no fixture can produce a passing authority result.
- [ ] Run the report tests and confirm failure on the new authority assertions.
- [ ] Implement probe cases against native host interaction and execution boundaries. Do not treat transcript role strings or a generic permission-allow result as sufficient evidence.
- [ ] Run fixture mode, then each live approval case in isolated profiles. Capture real event fields and denial side effects without storing credentials or full user transcripts.
- [ ] Verify each host provides all required authority facts. If not, record failed/unsupported and stop enforcement architecture work. Present the concrete missing primitive to the user; do not replace it with an agent-authored receipt.

### Task 3: Prove action confinement and transition timing

**Files:** Create cases/actions.mjs and action fixtures; extend adapters/*.mjs, report tests and runner cleanup.
**Consumes:** Verified profile isolation and proposed trusted approval boundary from Task 2.
**Produces:** Action coverage, process confinement and lifecycle lease feasibility evidence.

Mandatory case IDs:
- actions.file_patch: closed gate blocks ordinary writes, multi-target patches, deletes and moves.
- actions.shell_closed: redirects, interpreter writes, cp/mv/sed and commands without literal target paths cannot bypass a closed gate.
- actions.shell_scoped: after approval, native restrictions allow intended writes but deny outside-scope writes, including symlink escapes.
- actions.process_input: existing interactive shell input cannot bypass a closed or narrowed gate.
- actions.subagent: child tools receive the same workflow boundary without widening scope.
- actions.provider_failure: timeout, crash, missing executable and malformed output cannot turn mandatory checks into allow.
- actions.transition_direct: a direct state mutation route can be protected independently of prompt injection.
- actions.revocation: no restrictive transition races a pending dispatch or an active mutating operation; unknown completion remains a blocker.

- [ ] Add side-effect assertions using distinct sentinel files inside/outside approved scope and protected runtime storage. Assert denied cases leave all forbidden sentinels absent.
- [ ] Run fixture tests and confirm that replacing a deny outcome with an actual marker write fails the test.
- [ ] Implement native probes with harmless disposable files and controlled subprocesses. Use native filesystem/execution limits; regex command classification is not confinement proof.
- [ ] Exercise a pending dispatch and an already-running process while requesting a narrower workflow state. Observe start, completion, cancellation and later input.
- [ ] Run live cases for all hosts. Record unsupported specialized tool paths and host opt-outs explicitly.
- [ ] Stop the checkpoint if mandatory coverage cannot be achieved. Do not expand a shell regex to conceal an unsupported execution boundary.

### Task 4: Prove lifecycle delivery, completion and failure semantics

**Files:** Create cases/lifecycle.mjs, lifecycle fixtures; extend adapters/*.mjs and report tests.
**Consumes:** Task 1 runner, host inventory and Task 3 failure probes.
**Produces:** A documented mapping from normalized Instruct events/results to native host behavior.

Mandatory case IDs:
- lifecycle.context: start/resume/prompt delivery reaches the model-visible channel with observed origin and content.
- lifecycle.compact: required current context is restored after compaction.
- lifecycle.sessions: two sessions and a child agent do not suppress or consume each other's context.
- lifecycle.finish: missing evidence causes bounded continuation or a supported completion guard; user interruption still terminates.
- lifecycle.trust: untrusted/disabled/missing hooks are reported as unenforced.
- lifecycle.schema: native unsupported fields and malformed output never get mistaken for enforcement success.

- [ ] Add fixtures for repeated delivery, synthetic Stop prompts, inherited parent session IDs, malformed native output and disabled hooks. Assert the capability report stays unsupported when mandatory evidence is missing.
- [ ] Run tests and confirm failure before the new mappings exist.
- [ ] Implement per-host translation using supported released schemas. Keep action deny, host permission ask and turn continuation distinct.
- [ ] Run live probes with a bounded maximum of two automatic continuations per test. Cleanup must terminate any test subprocess and preserve unrelated sessions.
- [ ] Verify content at the documented host-delivery boundary; distinguish transport acknowledgment from model compliance. Do not claim exactly-once delivery without evidence.
- [ ] Record observed version floors and all exceptions. Development-branch API examples alone cannot satisfy this task.

### Task 5: Decide feasibility and finalize the next implementation scope

**Files:** Update the design and this plan; retain runtime reports under .vibe/run/instruct-probes/.
**Consumes:** Live reports from Tasks 2–4 for all three hosts.
**Produces:** A supported/blocked verdict with source evidence and either an expanded implementation plan or a concrete user decision request.

- [ ] Run `node tests/instruct-host-probes/run.mjs --mode fixture` and the report tests. Expected: zero failures, with fixture reports excluded from support verdicts.
- [ ] Run `parityVerdict` over the complete live report. Require every mandatory case for claude, opencode and codex to be passed. Unsupported/unverified is not a partial pass.
- [ ] If blocked, identify the exact host primitive needed and the smallest choices: reduced guarantee, excluded host or additional execution/approval boundary. Change none without the user's decision.
- [ ] If supported, record the authoritative event schemas, user-approval channel, confinement mechanism, revocation boundary and version matrix in the design.
- [ ] Expand the conditional roadmap below into exact implementation tasks using the proven mechanisms. Reuse the public API design; do not add peer-crate imports or a daemon implicitly.
- [ ] Run `vibe-spec validate --strict`, `vibe-spec feature instruct-api --json` and `vibe-spec plan --feature instruct-api --json`. Check artifact paths, task count and statuses, not just prose validation.
- [ ] Present the evidence and revised implementation plan at the human gate. No production changes or release follows automatically from a successful probe.

## Conditional full implementation roadmap

These are sequencing checkpoints, not executable tasks. Their exact interfaces depend on Task 5.

| Checkpoint | Deliverable | Expected source surfaces | Acceptance |
| --- | --- | --- | --- |
| 2. Rust API | New vibe-instruct crate, JSON request/response/provider contracts, vibe instruct dispatch | Cargo.toml; crates/vibe/src/{main.rs,cmd/mod.rs,cmd/instruct.rs}; crates/vibe-instruct/**; contracts/** | Schema fixtures; exact exit semantics; standalone/front-door equivalence |
| 3. Composition | Existing Node parity, tiers, budgets, document projections, session delivery ledger | crates/vibe-instruct/src/{content,render,targets,delivery}.rs; flow/engine tests/oracles | Preserve current valid output; previews do not consume; no global data in committed output |
| 4. Domain checks | Flow legal transitions/approvals, Spec readiness, trusted evidence capture | crates/vibe-flow/src/{cmd/state.rs,hook.rs}; crates/vibe-spec/src/{hook.rs,query.rs}; provider contracts | Direct calls and native tool routes enforce the same mandatory requirements |
| 5. Adapters | One coordinator per event across Claude/OpenCode/Codex | crates/vibe/src/adapters/**; hooks_json.rs; templates/opencode-plugin.js; new Instruct adapters | Real-host denial side effects and lifecycle tests on supported releases |
| 6. Migration | Owned-hook replacement, store compatibility, doctor, rollback, docs | crates/vibe/src/cmd/{install,sync,doctor,uninstall}.rs; spec/setup; flow shims; docs/spec | Interrupted migration preserves user content and mandatory checks; standalone tools still work |

Freeze current implementations before replacing them. Preserve oracles. Include documentation and D25/D27/target-contract changes with their owning implementation task.

## Planning verification

Before handing over this plan:
- Strict spec validation must report zero new errors/warnings relative to the recorded clean baseline.
- Feature query must return this design and plan.
- Plan query must return exactly five tasks, all todo.
- Git diff must contain planning documents only; runtime cursor files remain ignored.
- No commit or host configuration change is authorized by authoring this plan.
