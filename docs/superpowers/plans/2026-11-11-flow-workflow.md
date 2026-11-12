# Flow workflow implementation plan

> **For agentic workers:** Flow owns task execution and state. Use bounded method skills and explicit agent briefs; do not invoke a whole-lifecycle executor that commits or finishes branches. Track progress outside this frozen plan. No implementation or publication is authorized by this document.

**Goal:** Deliver one root Vibe guide and a rich Flow skill with mechanically checked progression, meaningful verification and proven Flow approval enforcement.
**Architecture:** Reuse the existing Flow runtime and direct host integration. Keep Spec as an existing dependency for artifact operations. General Instruct and Spec redesign work lives in GitHub issues, not this plan.
**Tech Stack:** Existing Rust workspace, Node/Bash compatibility and packaging, Markdown skills. No new third-party dependency assumed.
**Spec:** `docs/superpowers/specs/2026-11-11-flow-workflow-design.md`
**Feature:** flow-workflow
**Status:** Proposed; eight tasks, all todo. Supersedes the broad flow-instruct plan.

## Constraints and deferred scope

- Root skill `vibe` means toolkit orientation; workflow skill `flow` means lifecycle. Fold owned using-spec bootstrap into these entrypoints and retire its registration.
- Keep plugin brand vibe, runtime `.vibe/run/flow`, standalone Spec and frozen oracles.
- Two feature human gates; no automatic commits, branch finishing, cleanup or release.
- Plan changes and candidate changes invalidate the relevant approvals. Planning-artifact writes remain possible before implementation approval.
- A confirm flag or agent-writable receipt is not authenticated approval or trusted test evidence.
- Full blocking requires live Flow-specific proofs on Claude Code, OpenCode and Codex. Unsupported capability blocks the enforcement rollout, not independent skill work.
- Do not implement Rust Instruct/API/skill (#83), composer/delivery/projections (#84), general host coordinator (#85), or Spec provider/promotion redesign (#86).
- No product changes yet; first approval is review of this design and plan. Current worktree is suitable; do not create another unnecessarily.

Dependencies: 2←1; 3←1,2; 4←2,3; 5←1; 6←5; 7←5,6; 8←2–7. Task 7's feasibility probes may start after Task 1; its production enforcement waits on Tasks 5–6 and passing probes. Shared packaging/runtime files are integrated serially by the controller.

## Review focus

- Repointing legacy vibe discovery must preserve the Flow cursor/receipts before installing toolkit orientation (Tasks 2–3).
- Root/Flow/Spec routing must terminate without loading all skills or bypassing artifact ownership (Tasks 2,4).
- Updated writer metadata must survive every active writer and interrupted transition (Task 5).
- False approval and fresh but failed evidence must not authorize coding or ship (Tasks 6–7).
- Independent skill improvements must not be reported as cross-host enforced parity (Task 8).

### Task 1: Freeze the baseline and define behavioral cases

**Files:** Existing `spec/tests/pressure/{run.sh,parse.mjs,scenarios.tsv}`, `spec/tests/evals/selection.json`, `tests/skills/`, `flow/engine/tests/`; new `flow/tests/evals/{selection,scenarios}.json`; task-owned frozen oracle copies for replaced state/guard code.
**Produces:** Baseline and scenario reports keyed by source digest, host/model, inputs, expected observable behavior and actual evidence. Runtime logs remain ignored.

- [ ] Inventory current skill sources, callers, all cursor writers and host wiring. Baseline targeted checks and current errors before editing.
- [ ] Add root-orientation, direct-flow, direct-spec, no-root, missing-component, near-miss and delegated-worker selection cases. Add scenarios for unapproved coding, plan edits, scope drift, premature finish, resume and candidate changes.
- [ ] Extend existing evaluation tooling to load Flow data and explicit host/model selection. Keep selection scores separate from action-trace scores; unavailable model/host means unverified, not a silent fallback.
- [ ] Run current-skill baseline scenarios and preserve actual results. Freeze current replaced implementations byte-for-byte before changes; record intended divergences separately.
- [ ] Run `bash tests/skills/run.sh`, `bash build-plugin.sh --check`, `vibe-spec validate --strict`; record exact results. Do not assume the umbrella suite covers Rust tests.

### Task 2: Add the root vibe toolkit guide and retire redundant bootstrap

**Files:** New `skills/vibe/{SKILL.md,agents/openai.yaml}`; modify `spec/using-spec/SKILL.md` callers, narrow bootstrap references in `spec/SKILL.md`, `flow/reference/host-surfaces.json`, packaging/discovery tests. Source removal occurs only after all callers migrate in Task 3.
**Produces:** Toolkit guide with one-hop routing, component availability and “Vibe toolkit” UI name. Existing Spec behavior is preserved.

- [ ] Write failing selection tests distinguishing root orientation from workflow and knowledge tasks, including explicit `$flow`/`$spec` bypass of the root guide.
- [ ] Write concise toolkit purpose, capability map, spec discovery and single-dispatch rules. Do not duplicate workflow orders or advertise unavailable Instruct commands.
- [ ] Fold using-spec's useful read-only discovery into root and direct Flow entry. Preserve standalone Spec triggers; delegated workers obey their bounded brief instead of recursively launching the toolkit.
- [ ] Test missing components, conflicting providers and no spec root with actionable guidance and no unauthorized installation.
- [ ] Run structural and independent selection scenarios. Expect one owner per request and no routing recursion. Keep migration changes staged with Task 3 so legacy runtime resources are not lost.

### Task 3: Rename Flow and migrate discovery safely

**Files:** `flow/SKILL.md`, `flow/state-machine.json`, `.agents/skills/`, `.claude/skills/`, `build-plugin.sh`, `install.sh`, `crates/vibe/src/{cmd/install.rs,store/skills.rs,adapters/skills.rs}`, existing commands/hooks/resource discovery, generated plugin skill trees and migration tests.
**Produces:** `flow` source/discovery identity, namespaced `vibe:flow`, root vibe guide, no owned using-spec registration. Runtime paths remain unchanged.

- [ ] Add tests for legacy vibe-only, new toolkit+flow, both generations, foreign collisions, shared+marketplace shadowing, interrupted upgrade and uninstall.
- [ ] Preserve/migrate legacy cursor, evidence and resources before replacing `.agents/skills/vibe` or corresponding host entries. Do not let the new toolkit directory hide the only old runtime.
- [ ] Rename frontmatter and directory-based registrations together, update resource callers, and retire owned using-spec entries after callers move. Keep third-party/user-owned entries intact with diagnostics.
- [ ] Generate packages with clear toolkit/Flow UI names. Test relative resources in installed copies; no runtime files ship. Old command redirects may remain, but not a second old workflow skill body named vibe.
- [ ] Run `cargo test -p vibe`, `bash flow/tests/adapters/run.sh`, `bash build-plugin.sh --check`, `bash tests/skills/run.sh`. No general store or plugin-platform redesign.

### Task 4: Expand useful Flow procedures and bounded agent methods

**Files:** `flow/SKILL.md`; new `flow/procedures/{feature,quick,setup,strategy}.md`, `flow/reference/{task-contract,review-contract,authorization,recovery,compatibility}.md`, `flow/agents/{openai.yaml,code-explorer.md,code-architect.md,implementer.md,code-reviewer.md}`; `flow/reference/deps.json`; scenario fixtures.
**Consumes/produces:** Design TaskBrief and TaskResult shapes. Controller-owned progress, exact write scope and reviewed results; no worker-owned gates.

- [ ] Test premature branch finishing, frozen-plan edits, missing independent reviewer and redispatch of completed tasks before rewriting methods.
- [ ] Write prerequisites, inputs, artifacts, checks, re-entry and examples per procedure. Load relevant sections only. Keep D12 orders in one authored home.
- [ ] Implement the bounded execution method using external design/TDD/debug/review techniques as appropriate; whole-lifecycle executors cannot assume commits or finishing.
- [ ] Add task and reviewer briefs, dependencies, allowed scope, failure statuses and resume handling. Select capability independently of model; retain controlling user model preferences.
- [ ] Keep commands truthful to available runtime capabilities; activate new gated-state instructions only after Tasks 5–7. Quick uses two files/twenty lines/no contract change.
- [ ] Run lint plus independent realistic scenarios against installed skill resources. Observe behavior and outputs; text matching alone does not prove compliance.

### Task 5: Enforce legal Flow progression and consistent runtime state

**Files:** `crates/vibe-flow/src/{cli,machine,cursor,paths}.rs`, `cmd/state.rs`; new `src/{runtime,transition}.rs`, `tests/{runtime,transitions}.rs`; `flow/state-machine.json`, active shell/Node writers, `/flow` wrappers; new `contracts/flow-runtime.schema.json` and fixtures plus runner registration.
**Interfaces:** v2 cursor and TransitionRequest/Result from design. Immutable run records precede atomic cursor references; generic lock acquired before state-dependent reads.

- [ ] Add failing legal/illegal edge matrix, wrong run/revision, missing feature, corrupt current, absent-only fallback, concurrent writers, lock loss and crash tests.
- [ ] Implement one evaluator for legal edges and prerequisites. Update all writers to call it or refuse v2. No fallback to old ungated writers; no force bypass.
- [ ] Implement feature compound-preparation and ship states, rework paths and explicit recovery. Non-Quick setup mutation routes to feature planning; legacy setup.apply cannot authorize writes.
- [ ] Gate evaluation requires a verified grant interface from Task 7. Until that authority is implemented, return unavailable/deny for required gates; fixture verifiers never enter production wiring. Do not call edge validation full approval enforcement.
- [ ] Implement task/result references and resumable progress with stale result rejection. Preserve v1 data through explicit upgrade without inventing approval.
- [ ] Run `cargo test -p vibe-flow`, `bash contracts/run.sh`, `bash flow/tests/run.sh`, `node flow/engine/tests/run.mjs state.test.mjs`; pin intentional stronger behavior separately from frozen parity.

### Task 6: Strengthen verification and final-candidate handling

**Files:** New `crates/vibe-flow/src/{candidate,evidence}.rs`, `tests/{evidence,ship_gate}.rs`; `hook.rs`; Flow procedures and candidate fixtures. No changes to the Spec promotion engine.
**Interfaces:** EvidenceRecord/CheckResult from design; candidate snapshot includes tracked/untracked changes and prepared documentation. Read-only existing Spec validation remains available.

- [ ] Add failing tests for fresh failed/blank receipts, missing required checks, stale candidate, untracked changes, unresolved independent review and untrusted worker claims.
- [ ] Implement one readiness evaluator shared by writer and Stop. Distinguish agent-recorded observations from trusted collector evidence; unsupported trust remains explicit.
- [ ] Prepare additive docs/lessons/generated rules before final verification. Retain feature artifacts; do not automatically invoke destructive promotion/removal or bypass standalone Spec approval. Richer promotion is #86.
- [ ] Capture candidate identity without requiring commits, excluding only declared runtime/output paths. Bind ship to candidate and named action; any later change invalidates readiness.
- [ ] Run `cargo test -p vibe-flow`, `bash tests/skills/run.sh`, relevant hook tests and strict Spec validation. No timestamp-only success and no evidence bypass on Stop re-entry.

### Task 7: Prove and implement Flow-specific approval and action blocking

**Files:** New `flow/tests/authority/` isolated probe cases/report validator; `crates/vibe-flow/src/{approval,actions}.rs`, tests; existing Flow adapter/shim surfaces and narrowly scoped Flow-native adapters where absent. Exact native API files are fixed from live evidence before production implementation; no general provider coordinator.
**Interfaces:** Verified grant binding from design, approval verifier used by Task 5, trusted evidence collector used by Task 6, and pre-action evaluation. Probe result records host/version/profile digest, case ID, status and side-effect evidence; fixture/live modes remain distinct.

- [ ] Build discriminating fixture tests and isolated probes for genuine user response, forged/synthetic/replayed approval, protected records and direct state bypass. Refuse normal profiles; missing trust/authentication stays unverified.
- [ ] Probe file/patch/delete/move, scoped shell/interpreter writes, interactive input, child agents, missing/failed hooks and pending/running mutations during revocation. Denied cases must leave forbidden sentinels absent.
- [ ] Document actual released-version authority, confinement and completion/cancellation primitives for each host. A process string, writable grant file or generic permission prompt is not proof.
- [ ] If a mandatory primitive is absent, stop dependent enforcement rollout and return a concrete scope/design decision. Continue only independently approved skill work; never silently introduce a broker, excluded host or reduced guarantee.
- [ ] If proven, implement Flow's direct native integration using the shared domain evaluator. Invalid/missing grants or mandatory checks deny; reads and current planning-artifact writes remain available. Native permissions still apply.
- [ ] Bind pending dispatch/active mutations to revocation safely; require completion or confirmed cancellation before narrowing permissions. Invalidate plan approval before re-entry permits edits.
- [ ] Run Flow Rust tests, host fixtures and all three isolated live matrices. Report configured versus activated versus behaviorally verified separately. No user-profile rollout is authorized by tests.

### Task 8: Verify installed Flow behavior and reconcile its documentation

**Files:** Flow tests/doctor, package metadata, `flow/reference/templates/AGENTS.md`, Flow README/guidance and owning `docs/spec/parts/flow/` notes/D12/D27 as changed. Minimal toolkit/Spec bootstrap documentation updates only; no deferred subsystem work.
**Produces:** Full candidate, requirement/evidence trace and proposed named ship action. Independent skill delivery and complete blocking capability have separate status.

- [ ] Run fresh installed-resource scenarios for toolkit routing, direct Flow/Spec, feature/Quick, scope growth, task resume, worker return and independent final review.
- [ ] Verify migration preserves runtime data and user content, removes owned redundant registrations and diagnoses stale foreign copies. Do not modify third-party caches.
- [ ] Reconcile Flow contract/sequence with actual behavior; link #83–#86 as deferred ownership without claiming delivered integration. Root guide advertises available components only.
- [ ] Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `bash tests/run.sh`, `bash build-plugin.sh --check`, `vibe-spec validate --strict`, and relevant Flow drift/live-host checks. Report baseline deltas and missing live evidence.
- [ ] Obtain fresh independent whole-candidate review after documentation preparation. Recheck changed bytes and present candidate identity, limitations and the exact ship action at gate two. No commit, push, merge or deployment without authorization.

## Coverage and planning checks

R1→2–3; R2→4; R3→5; R4→7; R5→6; R6→6,8; R7→1–4,8; R8→all task boundaries.

Before handoff: strict Spec validation clean; feature query resolves the design and plan; plan query finds eight todo tasks; only planning documents changed; cursor remains `feature.plan flow-workflow`. The broad predecessor plans are superseded, not alternative execution instructions.
