---
type: concept
updated: 2026-11-26
---

# Product milestones

Back to [the plan](../plan.md). The owner approved this direction on 2026-11-21. Issues hold implementation detail. Feature designs, implementation plans and ship decisions retain their approval gates.

## Milestone specifications

- **Spec workflow** — [product requirements](milestone-spec-workflow.md), [technical contract](../tech/milestone-spec-feature-contract.md)
- **Flow execution** — [product requirements](milestone-autonomous-flow.md), [technical contract](../tech/milestone-flow-runner-contract.md)
- **Rust Instruct** — [product requirements](milestone-rust-instruct.md), [technical contract](../tech/milestone-instruct-provider-contract.md)
- **Feature workspaces** — [product requirements](milestone-feature-workspaces.md), [technical contract](../tech/milestone-workspace-contract.md)
- **CLI installation** — [product requirements](milestone-cli-installation.md), [technical contract](../tech/milestone-install-contract.md)

| Milestone | Completion evidence | Issues |
|---|---|---|
| [1. Trusted Spec workflow](https://github.com/LennardZuendorf/vibe/milestone/7) | Skill regression exercises against real features. Common create, validate, query, resume, verify and closeout behavior across selected native backends. Keep durable authority explicit. | Skill [#103](https://github.com/LennardZuendorf/vibe/issues/103), durable decision [#104](https://github.com/LennardZuendorf/vibe/issues/104), lifecycle [#108](https://github.com/LennardZuendorf/vibe/issues/108) (contract agreed, [#124](https://github.com/LennardZuendorf/vibe/pull/124)) with its contract tests in [#126](https://github.com/LennardZuendorf/vibe/issues/126), native OpenSpec [#107](https://github.com/LennardZuendorf/vibe/issues/107) |
| [2. Autonomous Flow execution](https://github.com/LennardZuendorf/vibe/milestone/8) | One host runs an approved single-repo plan, dispatches agents, verifies trusted candidate evidence, retries within limits, resumes and stops at the human gates. | Trusted verification [#101](https://github.com/LennardZuendorf/vibe/issues/101), runner [#106](https://github.com/LennardZuendorf/vibe/issues/106), advisory signals [#100](https://github.com/LennardZuendorf/vibe/issues/100) |
| [3. Rust Instruct and steering](https://github.com/LennardZuendorf/vibe/milestone/9) | Versioned Spec and Flow provider API, tested Rust composition and turn guidance on a reference host. Worktree-aware, deterministic managed lesson refresh in `AGENTS.md`. | Provider and delivery [#83](https://github.com/LennardZuendorf/vibe/issues/83)–[#86](https://github.com/LennardZuendorf/vibe/issues/86), managed lessons [#32](https://github.com/LennardZuendorf/vibe/issues/32) |
| [4. Feature workspaces across repos](https://github.com/LennardZuendorf/vibe/milestone/10) | Idempotent config across checkouts. One feature workspace joins independent worktrees for `indexed` and its web repo, sets up the selected spec backend, resumes and verifies both candidates, and stops for ship approval. | Config [#105](https://github.com/LennardZuendorf/vibe/issues/105), lifecycle [#44](https://github.com/LennardZuendorf/vibe/issues/44)–[#45](https://github.com/LennardZuendorf/vibe/issues/45), grouped workspace [#109](https://github.com/LennardZuendorf/vibe/issues/109), proof [#111](https://github.com/LennardZuendorf/vibe/issues/111) |
| [5. One CLI and six-host installation](https://github.com/LennardZuendorf/vibe/milestone/11) | One versioned binary install command, with host setup for MCP, skills, hooks, plugins and instructions. Each host proves discovery, real runtime behavior, upgrade, diagnostics and bounded uninstall on a clean profile. | Reference install [#113](https://github.com/LennardZuendorf/vibe/issues/113); Codex [#110](https://github.com/LennardZuendorf/vibe/issues/110), Claude [#112](https://github.com/LennardZuendorf/vibe/issues/112), Droid [#114](https://github.com/LennardZuendorf/vibe/issues/114), OpenCode [#115](https://github.com/LennardZuendorf/vibe/issues/115), OpenClaw [#116](https://github.com/LennardZuendorf/vibe/issues/116), Hermes [#117](https://github.com/LennardZuendorf/vibe/issues/117) |

## Gates and parallel work

- The [Spec lifecycle contract](../tech/milestone-spec-feature-contract.md) is agreed; the runner binds to it. Trusted verification gates the runner. Rust Instruct can proceed alongside the runner after agreeing on the provider boundary.
- Prove the reference installer early. Full distribution requires supported versions and a host-specific verification matrix. Hook trust and consent stay host-owned. The TUI follows the working runner and advisory signals.
- Prove the grouped workspace and the integrated two-repo workflow with separate Git histories. The selected artifact backend has one owner per feature.
- Compare Git-backed and managed remote configuration in [#118](https://github.com/LennardZuendorf/vibe/issues/118). This discovery covers instructions, MCP definitions and pinned plugin manifests, with local credential references. It runs alongside local delivery.
- OpenSpec root ownership is decided: `docs/spec/**` stays the single durable owner and `openspec/specs/` is a change-format surface, not a second home ([D49](../decisions/d49-the-spec-root-stays-the-durable-owner.md), closing [#104](https://github.com/LennardZuendorf/vibe/issues/104)). See [the fit assessment](../tech/spec/spec-openspec-durable-fit.md).

## When decisions are needed

Review this roadmap as planning and specification. Decide each open question in the feature design that needs it. Record approved durable rules in [Decisions](../decisions/README.md) and link the owning issue. Start implementation only after its plan approval.

| Decide by | Decision | Owning issues |
|---|---|---|
| Native OpenSpec feature design | Agree supported schemas and versions, and one writer for durable promotion, under the agreed shared lifecycle. The root is settled by [D49](../decisions/d49-the-spec-root-stays-the-durable-owner.md); any replacement needs a new equivalence decision. Skill review can use Superpowers now. | [#103](https://github.com/LennardZuendorf/vibe/issues/103), [#104](https://github.com/LennardZuendorf/vibe/issues/104), [#107](https://github.com/LennardZuendorf/vibe/issues/107) |
| Autonomous Flow design | Agree on the trusted evidence producer, approval binding and the reference host's scope enforcement. Choose retry and recovery policy in the runner design. | [#101](https://github.com/LennardZuendorf/vibe/issues/101), [#106](https://github.com/LennardZuendorf/vibe/issues/106) |
| Instruct integration design | Set the provider interface, command policy, delivery rules, and behavior when a host lacks a required capability. | [#83](https://github.com/LennardZuendorf/vibe/issues/83), [#84](https://github.com/LennardZuendorf/vibe/issues/84), [#85](https://github.com/LennardZuendorf/vibe/issues/85), [#86](https://github.com/LennardZuendorf/vibe/issues/86) |
| Grouped workspace design | Set feature-artifact ownership, member references, and partial-create recovery. Verify worktree config on its existing scope. | [#105](https://github.com/LennardZuendorf/vibe/issues/105), [#109](https://github.com/LennardZuendorf/vibe/issues/109) |
| Installer design | Choose the reference host, publication method, and artifact format. Decide supported host versions in the adapter work. Review any change to independent release trains separately. | [#113](https://github.com/LennardZuendorf/vibe/issues/113), [#39](https://github.com/LennardZuendorf/vibe/issues/39) |
| After remote config discovery | Choose Git-backed sync, managed storage, or continued local-only config after the two-machine experiment. | [#118](https://github.com/LennardZuendorf/vibe/issues/118) |
