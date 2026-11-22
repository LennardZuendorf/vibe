---
type: concept
updated: 2026-11-21
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
| [1. Trusted Spec workflow](https://github.com/LennardZuendorf/vibe/milestone/7) | Skill regression exercises against real features. Common create, validate, query, resume, verify and closeout behavior across selected native backends. Keep durable authority explicit. | Skill [#103](https://github.com/LennardZuendorf/vibe/issues/103), durable decision [#104](https://github.com/LennardZuendorf/vibe/issues/104), lifecycle [#108](https://github.com/LennardZuendorf/vibe/issues/108), native OpenSpec [#107](https://github.com/LennardZuendorf/vibe/issues/107) |
| [2. Autonomous Flow execution](https://github.com/LennardZuendorf/vibe/milestone/8) | One host runs an approved single-repo plan, dispatches agents, verifies trusted candidate evidence, retries within limits, resumes and stops at the human gates. | Trusted verification [#101](https://github.com/LennardZuendorf/vibe/issues/101), runner [#106](https://github.com/LennardZuendorf/vibe/issues/106), advisory signals [#100](https://github.com/LennardZuendorf/vibe/issues/100) |
| [3. Rust Instruct and steering](https://github.com/LennardZuendorf/vibe/milestone/9) | Versioned Spec and Flow provider API, tested Rust composition and turn guidance on a reference host. Worktree-aware, deterministic managed lesson refresh in `AGENTS.md`. | Provider and delivery [#83](https://github.com/LennardZuendorf/vibe/issues/83)–[#86](https://github.com/LennardZuendorf/vibe/issues/86), managed lessons [#32](https://github.com/LennardZuendorf/vibe/issues/32) |
| [4. Feature workspaces across repos](https://github.com/LennardZuendorf/vibe/milestone/10) | Idempotent config across checkouts. One feature workspace joins independent worktrees for `indexed` and its web repo, sets up the selected spec backend, resumes and verifies both candidates, and stops for ship approval. | Config [#105](https://github.com/LennardZuendorf/vibe/issues/105), lifecycle [#44](https://github.com/LennardZuendorf/vibe/issues/44)–[#45](https://github.com/LennardZuendorf/vibe/issues/45), grouped workspace [#109](https://github.com/LennardZuendorf/vibe/issues/109), proof [#111](https://github.com/LennardZuendorf/vibe/issues/111) |
| [5. One CLI and six-host installation](https://github.com/LennardZuendorf/vibe/milestone/11) | One versioned binary install command, with host setup for MCP, skills, hooks, plugins and instructions. Each host proves discovery, real runtime behavior, upgrade, diagnostics and bounded uninstall on a clean profile. | Reference install [#113](https://github.com/LennardZuendorf/vibe/issues/113); Codex [#110](https://github.com/LennardZuendorf/vibe/issues/110), Claude [#112](https://github.com/LennardZuendorf/vibe/issues/112), Droid [#114](https://github.com/LennardZuendorf/vibe/issues/114), OpenCode [#115](https://github.com/LennardZuendorf/vibe/issues/115), OpenClaw [#116](https://github.com/LennardZuendorf/vibe/issues/116), Hermes [#117](https://github.com/LennardZuendorf/vibe/issues/117) |

## Gates and parallel work

- Agree on the Spec lifecycle contract before binding the runner to it. Trusted verification gates the runner. Rust Instruct can proceed alongside the runner after agreeing on the provider boundary.
- Prove the reference installer early. Full distribution requires supported versions and a host-specific verification matrix. Hook trust and consent stay host-owned. The TUI follows the working runner and advisory signals.
- Prove the grouped workspace and the integrated two-repo workflow with separate Git histories. The selected artifact backend has one owner per feature.
- Compare Git-backed and managed remote configuration in [#118](https://github.com/LennardZuendorf/vibe/issues/118). This discovery covers instructions, MCP definitions and pinned plugin manifests, with local credential references. It runs alongside local delivery.
- OpenSpec root ownership stays open in [#104](https://github.com/LennardZuendorf/vibe/issues/104). Keep the current Vibe root until a decision is approved. See [the fit assessment](../tech/spec/spec-openspec-durable-fit.md).
