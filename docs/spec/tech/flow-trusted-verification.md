---
type: tech
applies: ["crates/vibe-flow/**", "contracts/flow-runtime.schema.json", ".vibe/run/flow/**"]
updated: 2026-11-26
---
# Trusted verification receipts
Design record for [#101](https://github.com/LennardZuendorf/vibe/issues/101), serving MS-FLOW-03 in the [runner contract](milestone-flow-runner-contract.md) and gating [#106](https://github.com/LennardZuendorf/vibe/issues/106). A receipt is an **observation, not a claim**: `state task-result` writes `{"source":"agent",…}` (`crates/vibe-flow/src/runtime.rs:314`) and `evidence::production_readiness` always returns `Err` (`evidence.rs:91-96`), so every adopted v2 run stops at verify. This document defines the missing producer, the document it writes, and the one path that consumes it.
## What exists at `9bd377a`
| Mechanism | Today | Source |
|---|---|---|
| Candidate identity | `git-object:` digest over tracked, untracked and ignored bytes, modes and symlink targets; `.vibe/run/flow` and `target` excluded | `candidate.rs:34`, `:53-143`, `:86` |
| Claims | task records are `source:"agent"` and grant nothing; `validate_record` accepts **any** non-empty source, so a receipt-shaped claim passes it today | `runtime.rs:314`, `:465`; `evidence.rs:21-54`, `:36` |
| Trusted readiness | always `Err`; the gate branch refuses before evidence is read, and the request's `candidate_digest` is never compared to the tree | `evidence.rs:90-96`; `transition.rs:11-15`, `:47-55` |
| Receipt store | **does not exist**; `evidence_refs` is shape-checked and only ever reset to `[]` | `runtime.rs:47-54`; `transition.rs:69` |
| Bindings | `plan_digest`/`scope_digest` are caller strings checked for non-emptiness only; a feature's `layer` is resolvable but Flow never reads it | `runtime.rs:206-225`, `:207`; `crates/vibe-spec/src/query.rs:847` |
## The receipt
One `v: 1` document, `kind: "verification-receipt"`, at `.vibe/run/flow/runs/<run_id>/receipts/<receipt_id>.json` — written through `immutable` and `confined` (`runtime.rs:146-166`, `:134-136`), and outside the tree it attests because `.vibe/run/` is gitignored and `candidate::snapshot` skips it. Field by field, the receipt *is* its bindings:

| Field | Binds | Source |
|---|---|---|
| `receipt_id` | `rcpt-` + `candidate::digest` of the body without it; a reader re-hashes and refuses a mismatch | `candidate.rs:10-35` |
| `run_id`, `revision` | the run lock and the cursor revision at production; consumption requires `expected_revision == revision`, so a receipt replayed at another revision is dead | `runtime.rs:137-145`, `:323-328` |
| `feature`, `backend`, `layer` | what the Spec layer resolves now (`superpowers`\|`fallback` today); a different answer is a backend move | `query.rs:847`, `:176-181` |
| `plan`, `approval_ref` | plan path, content digest, approval counter (1 = first approval of this run's plan), and the approval that authorized this revision and scope | approval store |
| `scope_digest`, `checks_digest` | approved task set, write scopes and check list; a widened scope cannot reuse a receipt | approval store |
| `candidate.worktree` | **content identity**: the snapshot re-derived on every read — byte, mode and symlink equality, never path equality | `candidate.rs:51-56`, `:83-143` |
| `candidate.commit`, `tree`, `clean` | **provenance**: `git rev-parse HEAD` and `HEAD^{tree}`, present only when `git status --porcelain -z -uall` was empty; ship requires it, verify does not | producer |
| `checks[]`, `task_ids`, `verdict` | argv, cwd, exit code and output digest as observed; covered tasks; a derived verdict, never a caller's | producer |
The id is integrity, not authenticity: content addressing proves the bytes were not edited after minting, not who minted them. Only the producer rule below does that, to the limit stated there.
## Who produces it
**The Flow writer**, subcommand `vibe-flow state verify` — not the runner, not a task agent, not a subagent, not a hook, not a signal processor, and there is no import path. The producer resolves the check list from the **approved scope** (a task brief's `checks` is an id list an agent asked for, `runtime.rs:333-353`), spawns each check itself, captures stdout/stderr to the run directory and digests it, records `ExitStatus`, and re-derives `candidate.worktree` before the first check and after the last: a run whose checks mutate the tree cannot certify the tree it started on.
- It refuses any receipt document handed to it, and any caller-supplied `--candidate`, `--commit`, `--tree`, `--checks` or `--verdict`.
- It refuses to mint outside `runs/<run_id>/receipts/`, over an existing receipt, or without the run lock and a matching `expected_revision`; a check that cannot be spawned is `inconclusive`, a refusal, not a pass.
- It refuses to cover tasks the cursor does not list as done, or a feature whose `layer` it cannot resolve.
A runner may *invoke* the producer; it cannot write a receipt, choose a check, name a candidate, or self-approve. Nothing here separates two processes under one OS user: the store is a directory, not a capability, so v1 claims procedural separation only. A gate needing more needs a key outside the worktree — open question 1, not an assumption this document makes.
## What invalidates one
| Trigger | Refusal |
|---|---|
| Any byte, mode or symlink target differs from `candidate.worktree`, or `HEAD`/`HEAD^{tree}` differs from `candidate.commit`/`tree` at ship — an edit, amend, rebase, or a promotion writing into the tree | `verification_stale_candidate` |
| The plan artifact is edited or re-approved — `plan.digest` is identity, `plan.revision` the label | `verification_stale_plan` |
| The approved scope gains, loses or rewrites a task or check | `verification_scope_changed` |
| `.vibe/spec.json` or the artifacts move the feature to another backend or layer | `verification_backend_moved` |
| The run is recovered into a new id (`runtime.rs:262-291`), or the revision advances after production | `verification_missing` |
| A rework lands: verify→plan clears `evidence_refs` (`transition.rs:67-70`), **verify→impl does not** | must clear; a stale ref is `verification_corrupt` |
No freshness clock applies: the legacy mtime tooth (`hook.rs:852-902`) is untouched and unrelated. A promotion that revises the plan or the tree re-opens verification by construction — there is no re-verify in place.
## How #106 consumes it
The runner never reads the store. It asks the writer once per candidate (`state verify --run <id> --expected-revision <n>`), then asks for the edge with `expected_revision=n`, the digest it observed, and `approval_ref`. The writer re-derives everything and answers exit 1 through `runtime::outcome` (`runtime.rs:256-261`) with `vibe state: ERROR: verification_<code>: <detail>`:
```
vibe state: ERROR: verification_missing: no receipt covers run <run> at revision <n>
vibe state: ERROR: verification_stale_candidate: receipt <id> binds <digest>, candidate is <digest>
vibe state: ERROR: verification_backend_moved: receipt <id> binds superpowers/fallback, feature resolves openspec
```
The remaining codes are `stale_plan`, `scope_changed`, `uncovered`, `not_independent`, `corrupt` and `unknown`; the runner matches `^vibe state: ERROR: (verification_[a-z_]+): `, records the code as the pause reason, and leaves the cursor alone. It must not retry `corrupt`, `backend_moved`, `stale_plan` or `scope_changed` — those need a human — and must never reuse a receipt across attempts, runs or rework, treat `source:"agent"` records or any signal as evidence, fall back to the legacy mtime receipt on a v2 cursor (`hook.rs:429-437`), or write into the store. An agent or human may run `state verify` in the dev loop; that receipt is `review.independent = false` and cannot close a scope that requires independence. Both gate edges keep their meaning (`flow/state-machine.json:8-9`): plan approval binds plan, scope, feature and backend; ship approval adds candidate and `receipt_id`. `transition::evaluate` must run **one verifier for both branches** — today `:47-49` refuses the gate before `:50-55` reads evidence, and neither knows a receipt exists.
**Out of scope.** Signals stay advisory ([#100](https://github.com/LennardZuendorf/vibe/issues/100), D28: nothing here reads `ledger.jsonl` or `vibe-signals`); TUI ([#102](https://github.com/LennardZuendorf/vibe/issues/102)) inherits the writer's refusals; the OpenSpec backend ([#107](https://github.com/LennardZuendorf/vibe/issues/107)) must exist before an `openspec` receipt can be issued; reference-host confinement and retry policy belong to #106; legacy F7/F8 behavior is unmodified. Nothing in this design exists yet: the units are a `receipt`/`approval` schema def, `receipt.rs`, `state verify`, `state approve`, one verifier in `transition.rs`, and computed plan/scope digests at `runtime.rs:206-225`.
## Open for approval
1. Is "only the writer's code path mints" enough, or must the ship edge require a key held outside the worktree? 2. Must a receipt be commit-bound for ship, or may a dirty worktree reach `feature.compound`? 3. Does "independent" mean a separate process (recordable) or a separate actor (not provable on one host)?
4. Do receipts travel — CI artifact, PR attachment, tracked path — or die with the gitignored run directory? 5. Does a toolchain change invalidate a receipt that recorded the old one? 6. Is a plan-revision counter worth an approval record per revision, or is the plan digest alone enough?

[Milestones](../concepts/product-milestones.md) · [Runner contract](milestone-flow-runner-contract.md) · [Feature contract](milestone-spec-feature-contract.md) · [Flow runtime](rust/rust-flow-runtime.md)
