# Flow workflow and toolkit entrypoint design

**Status:** Proposed; planning only. Implementation requires explicit approval.
**Date:** 2026-11-11
**Feature:** flow-workflow
**Plan:** `docs/superpowers/plans/2026-11-11-flow-workflow.md`
**Supersedes:** `flow-instruct` and `instruct-api` proposals for active implementation scope.
**Parents:** `docs/spec/parts/flow/flow.md`, `docs/spec/parts/flow/concepts/flow-contract.md`, `docs/spec/decisions/d12-orders-in-the-skill.md`, `docs/spec/decisions/d27-the-writer-is-the-gate.md`.

## User decisions and scope

Implement only the Flow parts of the audit. Replace the separate `using-spec` bootstrap with one root `vibe` skill that explains the toolkit and routes to its components. The workflow skill is `flow`, exposed as `vibe:flow` where plugin namespaces apply. Preserve the requirement that a coding action must pass step and approval checks; do not replace it with an advisory prompt.

In scope: toolkit orientation needed to enter Flow; canonical Flow naming and owned skill migration; detailed Flow procedures and bounded agents; existing Flow state/policy/guard/Stop runtime; transition legality, approval requirements, evidence and candidate handling; Flow-specific host capability probes and enforcement; tests and owning documentation.

Out of scope: a Rust Instruct crate/API/composer; general provider registry; shared cross-tool host coordinator; new Instruct skill; Spec engine redesign; new promotion protocol; global-store redesign; remote sources; TUI; a broker/daemon. Narrow packaging, entrypoint and read-only Spec integration changes needed by Flow remain in scope. These are not permission to redesign Spec.

Deferred work is tracked on GitHub:
- [#83 — Rust Instruct API and trusted providers](https://github.com/LennardZuendorf/vibe/issues/83), including the future Instruct skill.
- [#84 — composition, session delivery and managed projections](https://github.com/LennardZuendorf/vibe/issues/84).
- [#85 — shared provider coordinator across three hosts](https://github.com/LennardZuendorf/vibe/issues/85).
- [#86 — bounded Spec provider and promotion contracts](https://github.com/LennardZuendorf/vibe/issues/86).

The existing Flow issues #27 (writer/ledger) and #28 (orders/payload) remain related ownership. Reconcile their older `--confirm` assumptions against actual human-origin requirements. This plan does not depend on building Instruct.

## Skill identities and routing

| Skill | Canonical source | Role |
| --- | --- | --- |
| `vibe` | New `skills/vibe/SKILL.md` | Toolkit overview, component availability, onboarding and single dispatch |
| `flow` | Existing `flow/SKILL.md`, renamed frontmatter | Lifecycle, sequencing, task delegation, gates and recovery |
| `spec` | Existing `spec/SKILL.md` | Existing artifact discovery, validation and knowledge operations |
| `instruct` | Deferred to #83 | Root guide describes only the capabilities actually installed |

`using-spec` has no independent capability that requires a public skill. Fold its useful spec-first discovery into the root guide and direct Flow entry. Retire its owned registrations and redirects after caller migration. Keep standalone Spec usable without installing the root guide or Flow. Do not delete third-party or user-owned skills.

The root skill explains the relationship, discovers availability and routes once. It does not copy Flow orders or Spec procedures, own a cursor, or demand loading all component skills. Direct `$flow` and `$spec` continue to work. A normal feature request reaches Flow; Flow requests the necessary Spec artifact operation. Spec answers without recursively restarting Flow. Read-only questions can be answered without opening an implementation flow.

Use `name: vibe` and `name: flow` in their matching discovery folders. Keep package/CLI brand `vibe`, `.vibe/run/flow`, and historical oracles. Namespaced hosts may show `vibe:vibe` for the one toolkit guide and `vibe:flow` for workflow; do not invent a custom namespace to conceal this distinction. Use clear UI names: “Vibe toolkit” and “Vibe Flow.” Multiple physical copies must represent the same capability/version, with shadowing reported rather than different instructions hidden under one name.

## Skill content and agent capability

Flow's entrypoint holds discriminating triggers, current-state lookup, ownership, gate invariants and short orders. Detailed content is progressively disclosed:

```text
skills/vibe/SKILL.md
skills/vibe/agents/openai.yaml
flow/SKILL.md
flow/procedures/{feature,quick,setup,strategy}.md
flow/reference/{task-contract,review-contract,authorization,recovery,compatibility}.md
flow/agents/{openai.yaml,code-explorer.md,code-architect.md,implementer.md,code-reviewer.md}
```

Create each resource with a concrete consumer; reuse existing templates/scripts. No skill per phase and no duplicate external manuals. D12 stays: orders have one authored home; machine links and host projections do not duplicate procedure prose.

Each procedure provides inputs, prerequisites, permitted artifacts, decisions, completion evidence, recovery routes and a useful example. Agent briefs specify role, approved requirements, exact write scope, dependencies, checks and return format. The controller owns state and review dispatch; workers cannot widen scope, approve themselves, commit, clean the worktree or finish branches. Parallelize independent exploration and disjoint implementation; serialize shared-file dependencies.

TaskBrief: `{task_id, run_id, plan_digest, requirements, read_paths, write_scope, dependencies, checks, role}`. TaskResult: `{task_id, status, changed_paths, check_results, findings, artifacts, candidate_digest}`; status `done|needs_context|blocked|failed`. Worker results are claims until checked. Keep task progress in runtime records, not the frozen plan. Resume incomplete/stale tasks without redoing valid completed work.

Use bounded external techniques for brainstorming, planning, TDD, debugging and review. Do not invoke full external executors that assume commits and branch finishing. Resolve capability/provider/version separately from model. Missing required independent review is visible and cannot silently become self-review. Host-native agent registration is optional; a generic subagent can consume the same brief.

Codex UI metadata and tested host-specific enhancements can be projected separately. They never become enforcement. Test selection, resource loading and behavior from installed packages, not just source frontmatter.

## Lifecycle

Feature target: `design → plan → [approval] → impl → compound → verify → [approval] → ship → idle`.

Compound means reversible candidate preparation: additive documentation, lessons and generated rules, followed by final checks/review. It never means release. The Flow-only slice does not alter or invoke Spec's destructive promotion/removal operation as an automatic step; retain feature artifacts until the existing separately authorized Spec wrap-up or #86 protocol can safely remove them. Do not bypass Spec's existing promotion approval by passing an owner string. This avoids an extra mandatory gate within the Flow feature cycle.

Legal feature edges: design→plan|idle; plan→design|impl|idle; impl→compound|plan|design|idle; compound→verify|impl|plan|idle; verify→ship|compound|impl|plan|idle; ship→idle after the authorized action succeeds. Return to plan/design invalidates affected approval; candidate changes invalidate final readiness.

Exactly two human gates apply to feature implementation: design+plan approval before impl; final candidate+named action approval before ship. Discussion and read-only planning do not authorize implementation. Non-Quick setup mutations and requested implementation of strategy outcomes enter feature planning. Legacy setup.apply cannot become a bypass in the upgraded runtime. Explicitly requested spec scaffolding remains preparation.

Quick is at most two files, twenty changed lines and no interface/schema/contract change. It has its stated gate exception, but gains no implicit permission to commit, push, merge or deploy. Scope growth returns to feature planning.

## Flow runtime contract

Use existing `vibe-flow` and compatibility wrappers. No new general Instruct API or front door is required. All active state writers must call one transition evaluator or refuse the upgraded runtime.

Preserve `.vibe/run/flow/state.json` fields `flow`, `phase`, `feature`, `updated`; add versioned metadata `v:2`, `run_id`, `revision`, `plan_digest`, `scope_digest`, `task_state`, `evidence_refs`. Publish immutable artifacts/results under `.vibe/run/flow/runs/<run_id>/` before the cursor atomically references them. Reuse `vibe-core` locking before reads. No independent revision sidecar. Corrupt current state is an explicit recovery condition, never implicit idle or permission to select old state.

TransitionRequest: `{v:1, run_id, expected_revision, target, feature, approval_ref, candidate_digest, action_ref}`. Nullable references are required only for relevant edges. Result: `{v:1, status, revision, cursor, reasons}`. Validate edge, prerequisites, identity, expected revision and applicable grant together. No force bypass. Known state name alone is insufficient.

An approval request binds project/worktree/run, request ID, gate, artifact digest, scope digest and expected revision. A human-origin grant must be verified through a demonstrated trusted boundary. `--confirm`, agent text, synthetic continuations, tool output and writable records cannot manufacture it. The exact native authority mechanism is a Flow-owned feasibility result, not assumed solved by a JSON schema.

Before approval, current-phase design/plan writes and reads remain available. Product writes and unknown effects deny. Planning permission cannot modify grants or trusted runtime evidence. Runtime records change only through validated trusted operations. Re-entering planning invalidates approval before unfreezing the design/plan.

## Evidence and action blocking

EvidenceRecord: `{v:1, evidence_id, run_id, task_ids, candidate_digest, source, checks, review, verdict}`. CheckResult: `{id, argv, cwd, exit_code, output_digest, output_path, started_at, finished_at}`. Verdict: `passed|gaps_found|human_needed`. Required checks and independent-review requirements must actually be satisfied; timestamp freshness alone never passes.

Candidate snapshots include tracked and untracked work, prepared documentation and generated rules, without requiring a commit. Exclude only declared runtime/output paths. Keep agent-authored records distinct from trusted execution observations. Writer and Stop consume the same readiness predicate; a bounded Stop continuation cannot mark evidence passed.

Flow-specific adapters must intercept relevant edits/patches/deletes/moves, process start/input and subagent actions before execution. All patch targets and move endpoints matter. Shell regexes cannot prove confinement. Even after approval, process writes need a demonstrated approved-scope boundary. Restrictive transitions wait for active mutation completion or confirmed cancellation; pending dispatch must not race revocation.

Probe Claude Code, OpenCode and Codex in disposable roots/isolated profiles for genuine user origin, forged/replayed grants, protected records, direct transition bypass, forbidden side effects, process/input lifetime, child agents and missing/failed hooks. Record installed host version, profile/configuration digest and actual event/side-effect evidence. Do not use normal user profiles or synthesize human responses. Installation is distinct from trust, activation and demonstrated enforcement.

This work remains Flow-owned and can call its domain evaluator directly; no general provider coordinator is built here. If a mandatory boundary is unavailable, stop dependent enforcement rollout and present the precise gap. Independent skill improvements can still be reviewed and delivered, but they do not satisfy the full blocking requirement. Never silently weaken the guarantee, exclude a host or add a broker.

## Migration and acceptance

The old `.agents/skills/vibe` contains Flow resources and possibly legacy runtime state; the new entry at that same public name is toolkit orientation. Preserve/migrate legacy state, receipts and resources before repointing it. Install `flow` separately, update every caller, then retire owned `using-spec` links. Test missing/new/old/both/conflicting layouts and interrupted migration. Do not preserve an advertised legacy workflow alias named vibe.

Keep generated packages generated. Preserve unrelated hooks, foreign skills, user prose and existing store ownership. Detect stale marketplace copies and report normal package-management remediation; never rewrite third-party caches. Doctor reports selected capability/source/version and limitations.

Acceptance:
- R1: one toolkit `vibe` meaning, one workflow `flow` meaning, no owned `using-spec` registration after migration.
- R2: detailed, progressively loaded Flow procedures and bounded agents work from installed resources.
- R3: legal, revision-checked transitions and explicit recovery share one evaluator across writers.
- R4: real human approval and step/scope checks block actions on all three requested hosts, or remain explicitly incomplete with evidence.
- R5: full-candidate verification rejects failed/stale/insufficient evidence and includes untracked files.
- R6: compound preparation precedes review; explicit ship action uses approved candidate bytes.
- R7: migration preserves data/content and behavior tests catch collisions, premature finishing and resume errors.
- R8: no deferred Instruct/Spec subsystem enters this implementation scope.
