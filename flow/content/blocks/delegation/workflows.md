---
id: delegation.workflows
title: Dynamic workflows (Workflow tool)
channels: [agents-md]
---
<!-- vibe:summary -->
workflows: reach for the Workflow tool at 3+ independent parallelizable subtasks · without ultracode, propose shape + cost in 1–2 sentences and wait for my yes · every `agent()` call sets `model`; never `fable` inside a workflow
<!-- /vibe:summary -->

Applies to ALL sessions, any model. Dynamic workflows do not need to be avoided —
reach for the Workflow tool when a task has 3+ independent parallelizable subtasks or
would benefit from a pipeline/judge panel.

Standing rule on opt-in: if ultracode is NOT on for the session (no "ultracode"
keyword, no toggle, no orchestration request in my own words), plan first — propose
the workflow in one or two sentences with the rough shape and cost, and wait for my
reply; my "yes" is the opt-in. If ultracode IS on, invoke directly.

**Agent models inside workflow scripts:** every `agent()` call MUST set the `model`
parameter explicitly, chosen per "Delegating to sub-agents" above — with one
tightening: NEVER use `fable` agents in a dynamic workflow, not even with approval.
Only `haiku`, `sonnet`, or `opus`. If a Fable review is warranted, it happens AFTER
the workflow completes, as a standalone Agent-tool call (ask first, per above) —
never as a workflow stage.
