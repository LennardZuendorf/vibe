---
type: tech
applies: ["crates/vibe-instruct/**", "crates/vibe-core/**"]
updated: 2026-11-21
---

# Rust Instruct provider contract

Instruct owns instruction composition and delivery. Spec and Flow own their
domain facts. The host adapter translates available channels without creating
workflow state or granting authority.

## Contracts

- A versioned provider contribution identifies its provider, applicable project
  or worktree, revision and delivery channel. Validate shape, scope and size
  before composition. Missing optional context degrades visibly. Missing mandatory
  policy stops the affected action through its native authority boundary.
- Context may carry declared command guidance. Only the configured trusted
  provider policy can enable execution. Publishing text confers no command or
  approval authority. Preserve a bounded, offline session-start path.
- Follow the [Instruct tier and channel rules](../parts/instruct/concepts/instruct-contract.md).
  Resolve precedence deterministically and enforce individual and total budgets.
  Keep output order, source provenance and diagnostics stable for equal inputs.
- Session state tracks which edge and event contributions were consumed.
  Session start and compaction restore the current level context. A late or
  cross-worktree payload cannot override the active contribution.
- Select durable lessons for the repo-shared instructions channel. Rewrite only
  the owned block of the correct worktree's `AGENTS.md`. Retain text outside it
  and refuse malformed ownership markers. Temporary guidance goes to prompts.
- Reference-host checks inspect actual prompt delivery and readable managed
  files. Include over-budget context, unavailable provider, duplicate event,
  unsupported version and user edits.

## Open design

Set the provider ABI, command execution timing and missing host primitives in the
[Instruct integration issues](https://github.com/LennardZuendorf/vibe/issues/83).
Existing file boundaries and local delivery remain usable during migration.
Remote configuration has independent discovery and does not gate the local path.

[Milestones](../concepts/product-milestones.md) · [Product requirements](../concepts/milestone-rust-instruct.md)
