---
id: delegation.subagents
title: Delegating to sub-agents
channels: [agents-md]
---
<!-- vibe:summary -->
delegation: set `model` explicitly on EVERY sub-agent call — haiku (mechanical bulk) · sonnet (well-specified impl) · opus (tricky/adversarial) · fable only with my prior approval · unsure → cheaper tier, escalate on failure
<!-- /vibe:summary -->

Model tiers for ANY delegated work — Agent-tool calls and Workflow-script `agent()`
calls alike. Set the `model` parameter explicitly on every call; never omit it
(omission silently inherits the session model):

- `haiku` — mechanical bulk work: renames, boilerplate, format conversion, log triage.
- `sonnet` — default for well-specified implementation with clear acceptance criteria.
- `opus` — genuinely tricky work: concurrency, subtle algorithms, adversarial
  verify/judge panels, gnarly debugging.
- `fable` — rare; only when independence from your context is the point (e.g.
  adversarial review of your own plan or a large diff). If the complexity of the task
  warrants a Fable sub-agent, ALWAYS check with me first — never spawn one unprompted.

When unsure between tiers, pick the cheaper and escalate on failure.
