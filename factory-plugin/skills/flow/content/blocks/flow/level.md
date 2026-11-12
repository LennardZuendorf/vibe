---
id: flow.level
title: Flow cursor
channels: [user-prompt.level]
---
<!-- vibe:summary -->
state={{state}} · transition: {{transition}}
<!-- /vibe:summary -->

The cursor line. It rides EVERY turn, so it carries the two facts that go stale
the moment the flow moves and nothing else: which state the cursor is in, and
the command that leaves it. A gated edge renders as its `/flow … confirm` form,
because crossing it needs a human.

Everything a state implies but does not change per turn — its orders, its
delegates, its lessons — belongs in the `user-prompt.edge` channel, which is
injected only on the turn after the cursor moves.
