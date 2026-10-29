---
id: delegation.workflows
title: Dynamic workflows (Workflow tool)
channels: [agents-md]
---
<!-- vibe:summary -->
workflows: reach for the Workflow tool at 3+ independent parallelizable subtasks · propose the shape and cost in 1–2 sentences and wait for your human partner's yes unless they asked for orchestration · every `agent()` call sets `model`
<!-- /vibe:summary -->

Applies to ALL sessions, any model. Dynamic workflows do not need to be avoided —
reach for the Workflow tool when a task has 3+ independent parallelizable subtasks or
would benefit from a pipeline/judge panel.

Opt-in: unless your human partner already asked for orchestration in their own words,
plan first — propose the workflow in one or two sentences with the rough shape and
cost, and wait for their reply; their "yes" is the opt-in.

**Agent models inside workflow scripts:** every `agent()` call MUST set the `model`
parameter explicitly, chosen per "Delegating to sub-agents" above. Only `haiku`,
`sonnet`, or `opus`.
