---
id: delegation.subagents
title: Delegating to sub-agents
channels: [agents-md]
---
<!-- vibe:summary -->
delegation: set model on EVERY sub-agent call — haiku (mechanical bulk) · sonnet (default implementation, exploration, per-task review) · opus (architecture, adversarial or final review) · unsure → the cheaper tier, escalate on failure
<!-- /vibe:summary -->

Model tiers for ANY delegated work — Agent-tool calls and Workflow-script `agent()`
calls alike. Set the `model` parameter explicitly on every call; never omit it
(omission silently inherits the session model):

- `haiku` — mechanical bulk work: renames, boilerplate, format conversion, log triage.
- `sonnet` — default for well-specified implementation, exploration and per-task review.
- `opus` — architecture, adversarial or final review, genuinely tricky work:
  concurrency, subtle algorithms, gnarly debugging.

When unsure between tiers, pick the cheaper and escalate on failure.
