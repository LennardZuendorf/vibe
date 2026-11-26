---
type: tech
applies: ["crates/vibe-spec/**", "spec/**"]
updated: 2026-11-26
---

# Spec feature backend contract

Design record for [#108](https://github.com/LennardZuendorf/vibe/issues/108); it serves [MS-SPEC-01…03](../concepts/milestone-spec-workflow.md).
The shared Spec application layer owns feature discovery and lifecycle queries; Flow consumes that interface and never parses native artifact files.
A feature has exactly one **backend** and, within it, exactly one **artifact owner**.

| Backend | Change artifacts | Durable capability |
|---|---|---|
| `superpowers` (default, D45) | `docs/superpowers/specs/<date>-<f>-design.md` and `docs/superpowers/plans/<date>-<f>.md`; fallback layer `<root>/features/<f>/{product,tech,design,plan,research}.md` | the spec root (`docs/spec/**`) |
| `openspec` (native, opt-in) | `openspec/changes/<f>/{proposal,tasks,design}.md` plus delta specs under `specs/<capability>/`; archived at `openspec/changes/archive/<date>-<f>/` | `openspec/specs/<capability>/spec.md` |

The fallback folder is the `superpowers` backend's degraded layer, not a third backend: it applies when superpowers is absent or the layer is set to `vibe` (D45), with the same lifecycle.

## Contracts

- A resolved feature exposes project and branch context, stable identity, backend, authoritative paths, task IDs and readiness. Repeated reads of unchanged artifacts agree. Contract tests cover routing, resume, missing prerequisites, native edits and promotion, and assert real task ids and statuses.
- **Discovery** applies the selected layer first — a project whose layer is `vibe` never consults the superpowers candidates — then orders what remains: an `openspec/changes/<f>/` directory, a plan or design bound by the exact `-<f>.md` / `-<f>-design.md` suffix, then `<root>/features/<f>/`. Two backends holding one name is an error, not a precedence question; neither holding it is a lookup miss, not an empty feature: exit 1 naming the feature and both candidate roots, and never an implicit create.
- **Feature identity** is the slug `^[a-z0-9][a-z0-9-]*$`, no trailing hyphen, derived from the artifact that carries it: the `-<f>.md` / `-<f>-design.md` stem under `superpowers`, the `changes/<f>/` directory under `openspec`. There is no rename: a rename orphans every bound artifact, so a write refuses when the name matches a bound artifact filed under a different one. Identity is reported with backend and path.
- **Task identity** is the artifact's own: `### Task <n>`, `### <f>/<n>` or the `| Unit | Status |` table for `superpowers`; `- [ ] <n>.<m> <title>` in `tasks.md` for `openspec`. Queries return native ids verbatim, never renumber, read status from that backend's markers only, and answer "what is left, in what order" from one backend in one read — mixed reads are refused, not merged. No rows is a stated answer at exit 0; an unreadable artifact is exit 1.
- **Creation** goes through the backend's own writer — a superpowers plan, or `openspec new change`. A name the other backend already holds is refused with both paths. Vibe never scaffolds an authoritative copy of a change it did not create.
- Preserve repo-first and global-first resolution and reconciliation. A backend cannot silently switch code refs, change storage mode or start a second authoritative copy. Keep private spec history and index separate from code Git state.
- **Validation** stays three separate results: repository (`vibe-spec validate`), backend artifacts (`openspec validate --strict` or SF28–SF36), and Flow verification. A green in one cannot stand in for another or approve a gated edge.
- **Resume** re-reads the backend and its native task state before any write. A cursor whose artifacts no longer resolve under the recorded backend is reported, not repaired: an explicit transition corrects it, never a fallback to another backend.
- **Verification** binds evidence to the backend that owns the change; a receipt from one backend never closes a feature that has since moved. A promotion that revises the reviewed candidate re-opens verification.
- **Closeout** has one writer per fact. `promote <f> --done` (D46) retires superpowers artifacts after the durable fact lands in the spec root; `openspec archive` applies deltas to `openspec/specs/` and moves the change under `archive/`. Both at once, or either twice, is refused. Archive is not reversible upstream, so closeout states what it moved.
- **Durable ownership** waits for [#104](https://github.com/LennardZuendorf/vibe/issues/104): `docs/spec/` is authoritative for product, architecture, design, plan and lessons; an `openspec` backend's `specs/` only for the capabilities it owns. One fact, one writer.

## Mixed and ambiguous state

Each is an explicit refusal naming both candidates; none is resolved by precedence, newest-wins or silent fallback.

- one name under both backends, or a change directory plus a bound plan;
- a design/plan pair disagreeing on the topic slug, or a plan without its `**Spec:**` line (SF36);
- a change touching more than one capability — already refused as `touches <n> capabilities; a vibe feature targets one document`;
- a stale plan: DONE rows the tree contradicts, or rows past closeout still open (drift warns, it never silently fixes);
- a backend switch mid-feature, which needs a recorded decision and gives the moved change a new identity;
- a cursor or receipt naming a backend other than the one that now resolves.

## Version pinning

`openspec` support is validated against 1.14.2 only: ADDED/MODIFIED/REMOVED, `### Requirement:` plus `#### Scenario:`, whole-block MODIFIED replacement, checkboxes where only `x`/`X` count as done, archive at `changes/archive/<date>-<id>/`. Upstream also supports `RENAMED`; vibe refuses it and requires REMOVED + ADDED so a durable delta has one spelling. Any other OpenSpec version is unverified: `validate` warns and closeout refuses until re-pinned. Upstream stores and references are beta — names, flags and JSON shapes can change — so no design may depend on them.

## Where the tree stands today — these gaps are real

- Closed since this section was first written at `952ed3d`: `.vibe/spec.json` selects a layer ([#125](https://github.com/LennardZuendorf/vibe/issues/125), #128), and `feature`/`plan` now refuse a name two backends hold and name both candidate roots on a miss ([#126](https://github.com/LennardZuendorf/vibe/issues/126)); both selectors and both entry points — CLI and typed API — share one rule (`query.rs` `Lookup`).
- No query reads openspec artifacts. A name only `openspec/changes/<f>/` holds is reported as such and exits 1, never as a superpowers miss and never served; reading `tasks.md`, native ids and closeout stay [#107](https://github.com/LennardZuendorf/vibe/issues/107).
- `export|import --openspec` already carry the five feature files byte for byte (`openspec.rs:20-26`). Import refuses a multi-capability change, requirements edited on the OpenSpec side and an edited `tasks.md`; export refuses duplicate requirements, a MODIFIED block that drops scenarios, and `RENAMED`. But `plan_from_tasks` (`openspec.rs:687`) accepts `- [`/`* [`/`+ [` and renumbers units to `<f>/<i>`, so an OpenSpec-side number does not survive. The interchange half exists; selection, validation and closeout do not.
- `valid_name` (`query.rs:27-33`) admits a trailing hyphen that `validate` rejects for topic docs (`validate.rs:565-570`).

## Open inputs

[#104](https://github.com/LennardZuendorf/vibe/issues/104) owns the durable-root authority: keep `docs/spec/` with native OpenSpec as a selectable backend (the recommended default), or replace the root after an equivalence decision. [#107](https://github.com/LennardZuendorf/vibe/issues/107) builds and dogfoods the backend.

[Milestones](../concepts/product-milestones.md) · [Product requirements](../concepts/milestone-spec-workflow.md) · [OpenSpec interchange](spec/spec-openspec.md) · [Project storage](spec/spec-projects-mcp.md)
