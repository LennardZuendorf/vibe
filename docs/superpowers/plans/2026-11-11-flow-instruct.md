# Flow skills and Instruct integration implementation plan

> **For agentic workers:** Execute through the Flow-owned task contract below. Use bounded Superpowers methods, not its unmodified whole-lifecycle executors. Steps use checkboxes as the static acceptance checklist; record execution status in the run ledger without editing the approved plan. Subagents handle independent tasks and review. No automatic commits or branch finishing.

**Goal:** Ship clearly named, useful Vibe skills and one Rust instruction/policy interface with proven workflow blocking across Claude Code, OpenCode and Codex.
**Architecture:** Flow owns lifecycle and approvals, Spec owns artifacts and validation, Instruct coordinates composition and policy, and adapters translate host events. Keep independent binaries behind `vibe flow` and `vibe instruct`. Separate immediately buildable skill/composition work from enforcement that depends on demonstrated host authority.
**Tech Stack:** Existing Rust 2024 workspace, Rust floor 1.88, existing workspace dependencies, Node built-ins and existing Bash/package tooling. No new third-party dependency is assumed.
**Spec:** `docs/superpowers/specs/2026-11-11-flow-instruct-design.md`
**Feature:** flow-instruct
**Base:** c2881a73f2872be16971dd864bdfbd607f364215
**Status:** Superseded by `docs/superpowers/plans/2026-11-11-flow-workflow.md`. Broad non-Flow work moved to GitHub issues #83–#86. Do not implement these predecessor tasks independently.
**Supersedes:** `docs/superpowers/plans/2026-11-11-instruct-api.md`. The probe requirements are incorporated in Tasks 7–8.

## Global constraints

- Canonical workflow skill is `flow`; namespaced invocation is `vibe:flow`. Keep plugin/CLI brand `vibe` and runtime `.vibe/run/flow`.
- Three public skills: `flow`, `spec`, `instruct`; retain narrow read-only `using-spec` bootstrap. No skill per phase and no advertised legacy `vibe` body.
- Feature sequence: design → plan → approval → impl → compound preparation → verify → approval → ship. Quick: at most two files, twenty changed lines, no interface/schema/contract change.
- User approval is bound to a specific design/plan or candidate/action. Agent text, confirm flags, worker results and synthetic prompts are not approval.
- Block unsupported or indeterminate mandatory checks. Optional context may degrade with diagnostics. Native host permissions still apply.
- Keep approved plans immutable. Include uncommitted/untracked changes in candidate review. No commit, push, merge, deployment or global installation without its explicit authorization.
- Use existing `vibe-core` lock/atomic/marker primitives and freeze replaced implementations first. Never edit frozen oracles to make intended changes look like parity.
- Do not change live host settings during tests. Use disposable roots and isolated profiles; refuse live probes if isolation cannot be established.
- No peer-crate imports, implicit broker/daemon, reduced guarantee, excluded host or new dependency to work around failed feasibility.
- Evidence and approval stores require a proven authority boundary. Agent-launched shell processes, interactive input and subagents remain in scope.
- Follow current global model policy; resolve agent capability independently from model. No whole-lifecycle external executor may control Flow's gates or cleanup.
- Conditional Tasks 9–13 cannot claim production enforcement until Task 8 records passing mandatory live proofs and concrete authority interfaces. If proof fails, return for a design decision; approved independent tasks may continue.

## Checkpoints and dependencies

| Checkpoint | Tasks | Deliverable / exit |
| --- | --- | --- |
| A: skill foundation | 1–3 | One workflow identity, usable procedures and bounded delegation; no new enforcement claims |
| B: shared core and feasibility | 4–8 | Rust composition/API plus isolated three-host capability results; authority design decision before dependent work |
| C: executable workflow | 9–12 | Versioned transitions, approvals, candidate evidence and domain evaluators, conditional on B |
| D: host integration | 13–16 | Equivalent adapters, safe migration, behavioral acceptance and full reviewed candidate |

Dependencies: 2←1; 3←2; 4←1; 5←4; 6←5; 7←1; 8←7; 9←4,8; 10←9; 11←9,10; 12←4,11; 13←6,8,12; 14←2,13; 15←3,6,11,14; 16←15. Tasks 4–6 and 7–8 may run independently of skill work with disjoint files. Main-thread integration owns shared manifests, installer and generated packages.

## File ownership and interfaces

Existing owners: `flow/` for skill/machine/legacy engine; `spec/` for artifact skill; `crates/vibe-flow/` for Flow runtime; `crates/vibe-spec/` for validation; `crates/vibe/` for store, front door and installation; `contracts/` for schemas; `build-plugin.sh` for package generation. Do not edit generated packages independently.

New roots: `instruct/` for its public skill; `crates/vibe-instruct/` for its independent binary; `tests/instruct-host-probes/` for isolated feasibility tests. New modules listed below are proposed implementation paths, not claims that files already exist.

Use the design's exact API envelope, Context, ProviderManifest, Action, TaskBrief/TaskResult, TransitionRequest/TransitionResult and EvidenceRecord/CheckResult names. `decision=allow|warn|deny`; provider-only `abstain|error`; `verdict=passed|gaps_found|human_needed`. Do not substitute `block` for `deny` inside the shared API. Hook adapters translate native vocabulary.

New machine entrypoints: `vibe instruct api` and `vibe flow api` read one versioned JSON request from stdin. Flow API operations are `status`, `transition`, `task_result`, `gate_request`, `recover`; none grant human approval. `transition.data` is TransitionRequest. Other operation shapes are pinned in Task 4 before consumers are implemented. Existing human state commands call the same transition logic once upgraded.

## Review focus

1. Duplicate or foreign skill registrations must not be overwritten or silently selected (Tasks 2, 14).
2. A model-authored approval or evidence claim must not satisfy trusted checks (Tasks 7, 10–11).
3. A rejected shell/patch action must leave forbidden side effects absent, including interactive processes and children (Tasks 7, 13).
4. Concurrent transitions, crash recovery and stale delivery acknowledgments must not lose state or widen permissions (Tasks 6, 9–10).
5. Promotion, generated rules and untracked files must be included in final review, with candidate changes invalidating approval (Tasks 11, 15–16).

### Task 1: Establish the baseline and behavioral evaluation cases

**Files:** Read `docs/spec/features/skill-revision/{product,tech}.md`, `tests/skills/`, `flow/tests/`, `flow/engine/tests/`, `contracts/`; extend `spec/tests/pressure/{run.sh,parse.mjs,scenarios.tsv}` and `spec/tests/evals/selection.json`; add Flow scenario data at `flow/tests/evals/{selection,scenarios}.json`. Freeze replaced engine/scripts under a new task-owned oracle directory without editing existing oracles.
**Interfaces:** Produces baseline report keyed by test/scenario, source digest, host/model version, observed result and evidence path. Raw runtime evidence stays under ignored `.vibe/run/flow/runs/<run_id>/` or a disposable directory.

- [ ] Inventory exact existing evaluation entrypoints and capture baseline `git status`, package generation, targeted suites, strict Spec validation and relevant existing warnings. Discover paths before reading them.
- [ ] Add independent scenarios for skill selection, unapproved coding, scope drift, premature branch finishing, duplicate providers, resume, missing reviewer and final-candidate changes. Include expected observable outcomes from R1–R12, not matching prose.
- [ ] Extend the existing runner/scorer to accept the Flow dataset and explicit host/model selection. Preserve the current Spec suite. Selection scores and full action-trace scores are separate; record unsupported host/model execution instead of silently falling back. Codex supporting evaluations use the controlling Sol selection.
- [ ] Run scenarios against current skills before revision; preserve actual violations and successful controls. Do not invent a failed baseline if a scenario already passes.
- [ ] Freeze the exact current composer and write/state compatibility implementations that later tasks replace. Verify byte equality and record intended divergences separately.
- [ ] Run `bash tests/skills/run.sh`, `bash build-plugin.sh --check`, `bash contracts/run.sh`, `vibe-spec validate --strict`. Record real results; baseline failures are not permission to ignore changed behavior.

### Task 2: Rename the workflow skill and migrate owned discovery entries

**Files:** Modify `flow/SKILL.md`, `flow/state-machine.json`, `flow/reference/host-surfaces.json`, `build-plugin.sh`, `install.sh`, `crates/vibe/src/{cmd/install.rs,store/skills.rs,adapters/skills.rs}`, discovery/link fixtures; regenerate `plugin/skills/`, `plugins/vibe/skills/`, `factory-plugin/skills/`. Replace owned repo discovery symlinks only through the tested migration.
**Interfaces:** Produces `flow` directory/frontmatter identity and a migration plan with owned, foreign, same-source and conflicting entries. Runtime data-home compatibility remains separate from skill discovery.

- [ ] Add failing discovery/install tests for new `flow`, stale `vibe`, both-present, foreign collisions, plugin-plus-shared copies and unchanged runtime state/receipts.
- [ ] Rename portable skill identity and package destination mappings. Keep brand/path occurrences that mean plugin, CLI, store or historical oracle unchanged.
- [ ] Update runtime resource discovery, command/hook paths and skill links together. Legacy data is copied/preserved before removing an owned discoverable entry; no duplicate alias SKILL.md is installed.
- [ ] Generate packages and test relative resources from installed copies. Verify `vibe:flow` on namespace-aware hosts and `flow` on plain discovery hosts.
- [ ] Run `bash build-plugin.sh --check`, `bash tests/skills/run.sh`, `bash flow/tests/adapters/run.sh`, `cargo test -p vibe`. Expected: no newly introduced failures; owned old entries migrate, foreign entries remain with a conflict report.

### Task 3: Build useful Flow procedures and bounded agent briefs

**Files:** Modify `flow/SKILL.md`, `spec/SKILL.md`, `spec/using-spec/SKILL.md`, `flow/reference/deps.json`; add `flow/procedures/{feature,quick,setup,strategy}.md`, `flow/reference/{task-contract,review-contract,authorization,compatibility,recovery}.md`, `flow/agents/{openai.yaml,code-explorer.md,code-architect.md,implementer.md,code-reviewer.md}`; reuse Spec agent briefs and add its UI metadata where supported.
**Interfaces:** Consumes TaskBrief/TaskResult from the design. Produces a bounded native Flow task loop and capability-specific external method contract. State orders have one authored home and link to detailed procedure sections.

- [ ] Use Task 1 scenarios to expose premature finishing, plan mutation, extra approval stops and unbounded fallback before rewriting procedures.
- [ ] Write precise entry descriptions and phase procedures with prerequisites, inputs, outputs, evidence and recovery. Load only applicable resources. Bootstrap is read-only; Spec artifact operations cannot recursively restart Flow.
- [ ] Specify implementation/review briefs and resumable task results. Controller owns state, reviewer dispatch and authorization. External TDD/debug/review techniques cannot add commits, cleanup or release.
- [ ] Separate method capability, provider/version and model selection in dependency data. Missing required independent review reports blocked; optional techniques may have an explicit bounded fallback.
- [ ] Keep new runtime-dependent orders capability-conditional until Tasks 9–13 exist. Do not advertise unsupported `--confirm` or claim current gates enforce approval. Final state-order activation is Task 15.
- [ ] Run structural lint plus independent scenario reruns. Verify one owner per request, correct resource loading, preserved user scope and no autonomous finishing. Record model/version and actual outputs; do not count lint as behavioral proof.

### Task 4: Define contracts and independent Rust front doors

**Files:** Add `contracts/{instruct-api,provider-manifest,payload,flow-runtime}.schema.json` and matching fixtures; modify `contracts/{run.mjs,tests/run.test.mjs,exit-codes.json,exit-codes.schema.json}` as required. Add `crates/vibe-instruct/{Cargo.toml,src/lib.rs,src/main.rs,src/api.rs}`; modify workspace `Cargo.toml`, `crates/vibe/src/{main.rs,cmd/mod.rs}`; add `cmd/{flow,instruct}.rs` and dispatch tests.
**Interfaces:** Produces the design's typed JSON contracts and `vibe instruct`/`vibe flow` process forwarding. Internal Rust types deserialize these schemas; no peer-crate imports. Preserve payload `v,provider,seq,level,edge,session,events[{id,ts,text}]` while wrapping it in the new API.

- [ ] Write valid fixtures first; mutate one field for unknown major/operation/decision, identity mismatch, oversized input and malformed provider command cases. Extend the contract runner to actually enumerate every new fixture tree.
- [ ] Pin Flow API payloads: status `{run_id}`; transition TransitionRequest; task_result `{run_id,expected_revision,result:TaskResult}`; gate_request `{run_id,expected_revision,gate,artifact_digest,scope_digest,action_ref}`; recover `{expected_revision,reason,action:"abort"}`. Missing run/authority facts refuse; ordinary calls never create approval.
- [ ] Add forwarding tests for arguments, stdin/stdout, absent binary, structured denial and usage failure. Confirm failure before dispatcher implementation.
- [ ] Implement independent crate/front-door dispatch and schema-backed transport bounds. Operational errors stay distinct from valid deny decisions and native hook exits.
- [ ] Run `bash contracts/run.sh`, `cargo test -p vibe-instruct`, `cargo test -p vibe`. Expected: fixtures exercise all new definitions and standalone/front-door responses agree.

### Task 5: Port instruction composition and trusted provider resolution

**Files:** Add `crates/vibe-instruct/src/{config,blocks,channels,providers,render,evaluate}.rs` and `tests/{composition,providers}.rs`; consume frozen Task 1 corpus and current `flow/engine/content.mjs` behavior. Modify existing parity harness only to add the new implementation.
**Interfaces:** `render(request)` returns ordered fragments, provenance and delivery ID without mutation. `evaluate(request)` aggregates provider decisions without executing the proposed action. Provider registration is separate from `publish`.

- [ ] Add differential tests for tiers, placeholders, budgets, deterministic order, custom blocks and valid current outputs; add explicit tests for intended stricter errors.
- [ ] Add fake-provider tests for deny precedence, timeout, invalid response, missing required evaluator, optional context failure and untrusted manifest execution. A context publication cannot register an executable.
- [ ] Implement Rust composition and provider dispatch using argv arrays, bounded execution and declared trust. No host vocabulary or Flow states enter `vibe-core`.
- [ ] Preserve required policy independently of text override precedence. Return actionable owner/rule/recovery reasons.
- [ ] Run `cargo test -p vibe-instruct`, `bash contracts/run.sh`, `node flow/engine/tests/run.mjs content.test.mjs render.test.mjs`. Expected: compatible corpus passes; new semantic cases assert explicit expected decisions.

### Task 6: Implement session delivery and managed document projection

**Files:** Add `crates/vibe-instruct/src/{delivery,write}.rs`, `tests/{delivery,write}.rs`; integrate existing store/marker ownership rather than adding a second global root.
**Interfaces:** Delivery key is project/worktree/harness/session/agent/context epoch. `record.data` distinguishes delivery acknowledgment from untrusted observation. Acknowledgment contains delivery ID and exact provider sequence/event IDs. Managed writes target explicit regions only.

- [ ] Add tests where two sessions both receive initial context; source changes replay without state movement; previews do not consume; stale/duplicate/out-of-order acknowledgments cannot skip events.
- [ ] Add resume/compaction/child-agent tests and crash points around publish/ack. Validate opaque IDs before using them in paths.
- [ ] Implement locking and stable replay IDs. Mark delivered only at the demonstrated adapter boundary; report at-least-once behavior when host acknowledgment is unavailable.
- [ ] Add managed-write tests for AGENTS.md/CLAUDE.md sharing one physical target, malformed markers, foreign content, symlink escapes and global/local/session content excluded from committed output.
- [ ] Run `cargo test -p vibe-instruct` and `bash contracts/run.sh`. Expected: deterministic output, no cross-session consumption and no user-owned content loss.

### Task 7: Build and run isolated three-host feasibility probes

**Files:** Add `tests/instruct-host-probes/{schema.json,report.mjs,report.test.mjs,runner.mjs,run.mjs}`, `adapters/{claude,opencode,codex}.mjs`, `cases/{approval,actions,lifecycle}.mjs`, and fixture directories. Runtime reports remain ignored.
**Interfaces:** Report `{v:1,run_id,base_sha,mode,started_at,hosts,results}`; Host `{id,version,binary_digest,profile_root,configuration_digest,isolation_verified}`; Result `{host,case_id,status,evidence_paths,expected,observed,limitation}`. Mode `fixture|live`; status `passed|failed|unsupported|unverified`. `validateReport(report)` returns errors; `parityVerdict(report)` returns `{supported,blockers}`.

- [ ] Add failing report tests for missing hosts/cases, fixture-only success, unsupported origin, unverified isolation and logs that claim denial despite forbidden side effects.
- [ ] Implement disposable project/profile creation using documented installed-version launch options. Reject normal profiles and preserve unrelated sessions. Missing authentication/trust means unverified, not a bypass.
- [ ] Probe actual-user approval, synthetic/agent/forged API requests, replay binding and agent writes to proposed grant/evidence storage.
- [ ] Probe file writes, multi-target patches, deletes/moves, shell/interpreter writes, approved-scope escapes, interactive input, child agents, direct transition access and provider failure. Assert forbidden sentinel files remain absent.
- [ ] Probe pending dispatch and running mutations during revocation, completion/cancellation, startup/resume/compaction, independent sessions and disabled/untrusted hooks. Bound automatic continuation to two attempts.
- [ ] Run `node --test tests/instruct-host-probes/report.test.mjs` and `node tests/instruct-host-probes/run.mjs --mode fixture`; then explicit `--mode live --harness <id> --case <id>` probes for each installed host. Keep real human interactions pending when required; never synthesize them.

### Task 8: Resolve the enforcement feasibility checkpoint

**Files:** Update this design/plan only through planning re-entry; preserve immutable Task 7 reports. No production integration mutation in this task.
**Interfaces:** Produces an evidenced host/version matrix and concrete authority/confinement implementation contract, or a blocked decision identifying the missing primitive.

- [ ] Require all mandatory live cases on all three hosts for an enforced-parity decision. Fixture passes, undocumented development APIs and configuration presence cannot satisfy it.
- [ ] For a passing result, pin native approval origin, request binding, protected store, process confinement, action lifecycle/lease primitive and trusted evidence collector. Add exact adapter call shapes and version floors to the design before implementing Tasks 9–13.
- [ ] For an unsupported result, state the smallest options: an additional boundary, a reduced guarantee or a host restriction. Make no such choice without user agreement. Independent approved skill/composition work can continue.
- [ ] Reconcile conditional task interfaces with the proven mechanism and present any material design/scope change at the plan gate. No automatic additional gate is required for a result already within the approved design, but unresolved architecture cannot be filled with assumptions.
- [ ] Run strict Spec validation and check all contract field names against Task 4 schemas. Mark capability results in runtime evidence; do not mark unimplemented tasks done in the approved plan.

### Task 9: Make Flow transitions atomic, legal and recoverable

**Files:** Modify `crates/vibe-flow/src/{cli,machine,cursor,paths}.rs`, `cmd/state.rs`; add `src/{runtime,transition}.rs`, `tests/{runtime,transitions}.rs`; update `flow/state-machine.json`, all active JS/shell writers and wrappers. Preserve frozen oracles.
**Interfaces:** v2 cursor and TransitionRequest/Result from the design. Immutable run records are published before cursor references. Existing generic `vibe_core::lock::Lock` owns each read/validate/write transaction.

- [ ] Add failing table tests for every legal/illegal edge, missing feature, stale revision, corrupt state, invalid current ancestor, direct CLI bypass, v1 upgrade and two writers racing at one revision.
- [ ] Implement v2 runtime snapshots, immutable run record publication and explicit abort recovery. A lost lock refuses commit. Corrupt state never becomes idle implicitly.
- [ ] Implement target lifecycle including preparation-only compound and actual ship state. All writers use the same transition evaluator or refuse v2; no compatibility fallback may restore the old bypass.
- [ ] Route non-Quick setup mutations and requested strategy implementation to feature.design. Legacy setup.apply resumes through planning rather than authorizing writes; add edge and migration tests.
- [ ] Implement task-result recording with controller checks, dependencies, scope and stale-result rejection. Gate prerequisites consume trusted references, not context text.
- [ ] Run `cargo test -p vibe-flow`, `bash flow/tests/run.sh`, `node flow/engine/tests/run.mjs state.test.mjs`. Register expected security divergences separately from oracle parity; never change the frozen oracle.

### Task 10: Enforce approvals and scope through the proven authority

**Files:** Add `crates/vibe-flow/src/{approval,actions}.rs`, `tests/{approval,actions}.rs`; integrate the Task 8 authority boundary and Task 9 transition evaluator. Exact host primitive files are added to this plan at Task 8.
**Interfaces:** GateRequest binding and trusted grant reference from the design. Ordinary Flow/Instruct APIs request or inspect gates but never mint a grant. Action authorization binds policy revision and protected scope until dispatch/completion resolution.

- [ ] Add rejection tests for agent-authored confirm/user fields, wrong worktree/run/request, changed plan/scope, expired/revoked/replayed grant and synthetic continuation.
- [ ] Implement authenticated approval verification through the proven native boundary. Protect grants and trusted evidence against agent processes before and after approval.
- [ ] Implement current-phase planning-artifact allowances before approval, trusted command-only runtime updates and denied product/authority writes. Test allowed active plan edits, denied neighboring files and scope-confined process equivalents. Re-entry invalidates approval before unfreezing the plan. Scope authorizes classes of planned edits without asking for every write.
- [ ] Implement action leases or the proven equivalent. Restrictive transitions wait for active writes or confirmed cancellation; stale pending dispatch cannot execute after revocation.
- [ ] Run `cargo test -p vibe-flow` plus Task 7 live approval/action cases. Expected: forged grants fail, denial has no forbidden side effect, valid scoped work proceeds and native host restrictions remain effective.

### Task 11: Bind verification and ship readiness to the complete candidate

**Files:** Add `crates/vibe-flow/src/{evidence,candidate}.rs`, `tests/{evidence,ship_gate}.rs`; modify `hook.rs`; extend Spec promotion integration in `crates/vibe-spec/src/{promote,drift}.rs` only where required.
**Interfaces:** EvidenceRecord/CheckResult from the design; candidate identity covers approved review scope including tracked and untracked files, prepared docs and generated rules. Snapshot plan/design before feature artifact removal. Exclude only declared runtime/output paths, never arbitrary changed files.

- [ ] Add tests for fresh failed receipts, missing checks/task coverage, false worker claims, stale candidate, untracked changes, unresolved review findings, pre-existing baseline failures and no-Git projects with explicit limitations.
- [ ] Implement trusted check capture and candidate snapshots without requiring commits. Verify evaluator and Stop consume the same readiness predicate; bounded Stop continuation cannot manufacture success.
- [ ] Integrate reversible compound preparation before final verify. Preserve approved plan/evidence snapshots before removing feature documents. Standalone Spec promotion keeps its existing human gate; Flow-owned preparation is reviewed at ship.
- [ ] Bind ship grant to exact candidate plus named action; failure remains pending. Any subsequent bytes or integration change invalidates readiness and requires relevant rechecks/reapproval.
- [ ] Run `cargo test -p vibe-flow`, `cargo test -p vibe-spec`, `bash tests/skills/run.sh`. Expected: final review sees all candidate changes and no file timestamp alone authorizes ship.

### Task 12: Expose Flow and Spec as Instruct providers

**Files:** Add `crates/vibe-flow/src/provider.rs`, `crates/vibe-spec/src/provider.rs` and integration tests; modify their CLI dispatch; add built-in trusted manifest generation under `crates/vibe-instruct/src/providers.rs`.
**Interfaces:** Both providers speak Task 4 contracts over subprocess JSON. Flow evaluates workflow/scope/approval; Spec evaluates spec-path/readiness rules. Instruct knows provider capabilities, not domain-specific state names.

- [ ] Add same-input tests through direct domain API and coordinated Instruct evaluation. Assert equal rule IDs/decisions and denial precedence across providers.
- [ ] Implement context publication separately from policy evaluation. Missing mandatory peer refuses; absent optional context emits a bounded diagnostic. Signals and worker observations cannot become trusted evidence.
- [ ] Add a fake third provider fixture without editing any host adapter; verify registration, namespaced publish, render and deny behavior.
- [ ] Run `cargo test -p vibe-instruct`, `cargo test -p vibe-flow`, `cargo test -p vibe-spec`, `bash contracts/run.sh`. Expected: provider extension requires no adapter/domain import changes.

### Task 13: Implement equivalent native host adapters

**Files:** Add `crates/vibe-instruct/src/adapters/{mod,claude,opencode,codex}.rs`, `src/templates/opencode-plugin.js`, fixture/integration tests; integrate with `crates/vibe/src/{hooks_json.rs,adapters/claude.rs,adapters/opencode.rs,adapters/codex.rs}` and existing store refresh projection.
**Interfaces:** Normalize lifecycle and Action inputs; return native context/deny/continuation formats from the same policy result. Use only Task 8 proven released-version APIs.

- [ ] Add fixtures for every supported event, unknown event/tool, malformed JSON, timeout, missing binary, native trust disabled and required capability absent.
- [ ] Implement one coordinator per event with domain routing, preserving native permission precedence. Keep action denial separate from a permission question and a synthetic continuation.
- [ ] Cover patch targets, shell/process input, child agents, startup/resume/compaction and action completion. Reject unsupported required paths; never label a skipped tool enforced.
- [ ] Run `cargo test -p vibe-instruct`, `cargo test -p vibe`, adapter fixture suites and complete Task 7 live cases for each supported version. Verify side effects and delivery boundary, not only expected response JSON.

### Task 14: Migrate installations and make doctor report real capability

**Files:** Modify `crates/vibe/src/cmd/{install,sync,doctor,uninstall}.rs`, adapters/store ownership modules, `install.sh`, `build-plugin.sh`, `flow/scripts/{doctor,check-skills}.sh`, `flow/reference/{deps,host-surfaces}.json` and migration tests.
**Interfaces:** Migration inventory/plan distinguishes ownership, skill source/version, hook event, runtime layout and capabilities. Doctor reports installed, resolved, trusted, active, behaviorally verified and unsupported separately.

- [ ] Add interruption/retry/rollback tests for old-only/new-only/both/invalid-current layouts, foreign collisions, unrelated hooks and cached obsolete plugins.
- [ ] Implement owned skill/hook replacement without duplicate injection or silent loss of mandatory enforcement. Preserve cursor/evidence bytes and user prose; do not edit third-party caches.
- [ ] Add source/version/digest resolution and shadowing diagnostics. Detect configured but unregistered skills and self-review fallback. New dependency versions require compatibility evidence.
- [ ] Ensure rollback reports unenforced if strong checks cannot remain active. Uninstall removes owned projections only.
- [ ] Run `cargo test -p vibe`, `bash flow/tests/adapters/run.sh`, `bash build-plugin.sh --check`. Live user-profile migration remains a separately authorized rollout, not a test side effect.

### Task 15: Activate final skills and evaluate real task behavior

**Files:** Complete `flow/SKILL.md` and supporting procedures, `spec/SKILL.md`, `spec/using-spec/SKILL.md`; create `instruct/SKILL.md`, `instruct/reference/{providers,channels,diagnostics}.md`, `instruct/agents/openai.yaml`; regenerate package metadata/resources and update scenario fixtures.
**Interfaces:** Every advertised command is implemented and every state order resolves to one procedure. Public Instruct skill serves explicit instruction management, not ordinary feature routing. Host-specific enhancements are projected and tested separately from portable content.

- [ ] Activate final state orders only after their runtime and method prerequisites pass. Include real examples for task dispatch, review findings, recovery and instruction explanation without duplicating domain logic.
- [ ] Add Codex UI metadata and supported host discovery controls; use native fork/dynamic-context features only where they have a tested consumer and preserve Flow ownership.
- [ ] Run independent end-to-end scenarios for a feature, a qualifying Quick fix, a too-large Quick request, scope drift, interrupted task resume, missing reviewer and explicit instruction management.
- [ ] Verify subagents return without state mutation/commit/cleanup; review includes untracked changes; skill collisions are diagnosed; only relevant references load. Test legacy invocation redirects without reintroducing a `vibe` skill body.
- [ ] Run `bash tests/skills/run.sh`, `bash build-plugin.sh --check`, installed-resource discovery tests and recorded model-based scenarios. No scenario result is inferred from static checks.

### Task 16: Reconcile owning specs, verify and present the ship candidate

**Files:** Update owning Flow/Instruct/Spec/CLI notes under `docs/spec/parts/`, `docs/spec/tech/`, D25/D27/D40/D48 where changed, `docs/spec/concepts/feature-sequence.md`, `flow/reference/templates/AGENTS.md`, README/install guidance and generated active rules. Promote this feature only through the agreed preparation path.
**Interfaces:** Produces a complete candidate, requirement/task/evidence trace and named proposed ship action. This task does not authorize that action.

- [ ] Reconcile F7/F8 and F11–F15 status against actual delivered behavior; keep deferred source sync/TUI work deferred. Amend decisions and host contracts with evidence, not target claims.
- [ ] Prepare durable specs/lessons/rules and remove only feature artifacts captured in the immutable run snapshot. Regenerate packages/instructions before final review. Do not edit active-rule blocks manually.
- [ ] Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `bash tests/run.sh`, `bash build-plugin.sh --check`, `vibe-spec validate --strict`, `vibe-spec check-drift --pre-merge`, plus the recorded three-host live matrix. Report baseline deltas honestly; the umbrella suite does not include the Rust workspace suite.
- [ ] Obtain fresh independent whole-candidate review, resolve findings and rerun affected checks. Recompute candidate identity afterward.
- [ ] Present design requirements R1–R12 coverage, actual checks, host limitations, candidate identity and exact proposed integration/release action at gate two. If no ship action is authorized, leave the worktree intact and ready for review.

## Requirement coverage

R1→2,14,15; R2→3,15; R3→3,9–11; R4→7–10,12–13; R5→9–10; R6→11,16; R7→4–6,12; R8→7–8,13; R9→6,13; R10→2,14; R11→1,3,15; R12→11,16.

## Planning handoff checks

- `vibe-spec validate --strict` reports no new errors/warnings.
- `vibe-spec feature flow-instruct --json` resolves this design and plan.
- `vibe-spec plan --feature flow-instruct --json` resolves sixteen todo tasks.
- Changed tracked content is planning documentation only; no skill/runtime/host implementation is claimed.
- Cursor remains `feature.plan` for `flow-instruct`, awaiting explicit implementation approval.
