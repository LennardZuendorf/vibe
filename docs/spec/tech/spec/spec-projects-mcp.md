---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "crates/vibe/**"
  - "spec/**"
updated: 2026-11-12
---

# Spec projects and MCP

Back to [Spec Framework — Tech](../spec.md).

## Implemented branch surface

One Rust application API serves embedded `vibe spec`, compatibility `vibe-spec`
and local stdio MCP. Local verification and independent review passed.
Release distribution and installer delivery remain separate.
Source delivery does not establish release/install or CI. The canonical root remains `docs/spec/`.

Repo-first keeps durable and feature specs in the code repository. Operations
follow a selected or matching ambient checkout; otherwise project-only access
uses the registered main checkout's actual branch. Selection never switches code.
Global-first keeps the entire durable spec and feature designs/plans outside
code, per registered project and stored code branch. Matching clones/worktrees
share a branch's spec; different branches stay isolated. Stored branches survive
code-branch deletion. Code, durable-spec and feature-artifact anchors are separate.
Branch init uses exact stored refs and only its caller's matching code anchor;
otherwise it reports stored-only context, without selecting another clone/main.

Global history uses project-private Git and branch worktrees. Locks and opaque
expected revisions protect writers; raw edits require explicit snapshots.
Recorded drafts may have semantic diagnostics. Completed reconciliation requires
zero validation errors; warnings remain visible. Code Git/index stays untouched.

Explicit reconciliation previews exact captured inputs; saved plans apply without
selection overrides. Conflicts retain an isolated workspace and readable base,
source, target and proposed content. Resolution submits the full manifest bundle
with separate target/workspace tokens; canonical content changes only on success.
Previews neither recover nor refresh. Explicit record operations do not run a
broader refresh; later selected reads observe final context/report state.

Only explicit snapshots capture exact code/spec bindings. First observation
baselines; subsequent reads can reconcile a newly observed direct two-parent
merge only with unique clean unchanged evidence for both sides. Squash/rebase,
intervening history and missing/ambiguous evidence need explicit reconcile.
There is no code-merge blocker, daemon, fetch or automatic artifact cleanup.

MCP is local stdio over 28 typed tools, with immutable launch defaults and
per-call selection. Its reviewed rmcp/Tokio dependency closure is spec-only.
Admission is bounded at 32 application operations. Documents have an 8 MiB
UTF-8 bound; encoded frames/envelopes include newline in their separate 8 MiB
bound. Stdout is protocol-only. Post-publication
cancellation finishes bookkeeping and preserves results; a reply may be absent.

No mode migration, remote transport, multi-machine sync, Flow/Instruct, general
Spaces or harness MCP configuration is included. Locks are never stolen by age.
Unrecorded interrupted merge workspaces remain preserved needs-input material.

The durable [contract](../../parts/spec/concepts/spec-framework-contract.md) and [feature sequence](../../parts/spec/concepts/spec-feature-sequence.md) record scope.
