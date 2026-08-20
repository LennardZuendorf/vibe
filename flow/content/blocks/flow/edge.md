---
id: flow.edge
title: Flow orders
channels: [user-prompt.edge]
---
<!-- vibe:summary -->
{{orders}}
delegates: {{delegates}}
{{lessons:.state}}
<!-- /vibe:summary -->

The full payload for a state, injected on the turn AFTER the cursor moves and
not again until it moves once more. Re-reading it every turn bought nothing —
the text is identical while the cursor sits still — and cost the whole prompt
budget it occupies.

`{{lessons:.state}}` is the indirect form: the tag is the CURRENT state, so a
lesson tagged with a state name reaches the flow that has to act on it.
