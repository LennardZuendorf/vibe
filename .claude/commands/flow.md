---
description: Transition the vibe flow state machine to a new state
argument-hint: <flow.phase> [feature] [confirm]
---

You have been asked to transition the vibe flow: $ARGUMENTS

`vibe-flow state set` owns this move. It refuses an unknown target, a target
outside the current state's `next`, and a gated edge without `--confirm`, and it
records every accepted write. Do not re-derive any of those rules here.

Parse `$ARGUMENTS` into up to three parts (whitespace-separated):

- **target** — the first token, a `<flow.phase>` state key (e.g. `feature.design`, `idle`).
- **confirm** — the literal token `confirm`, when it appears anywhere after the target:
  the human-approval token that unlocks a gated edge.
- **feature** — any other token after the target: the feature name to carry into the cursor.

Then run exactly one command, from the project root:

```bash
vibe-flow state set <target> [<feature>] [--confirm]
```

- Append `--confirm` only when the **confirm** token was present. There is no
  other flag and no override.
- Never pass `confirm` as the feature: the feature slot takes only the remaining
  token, and it is omitted when there is none (the current feature is preserved;
  `idle` clears it).
- Do not read the cursor or the state machine, do not run another writer, and do
  not write the cursor yourself.
- If `vibe-flow` is not on `PATH`, stop and say so; do not fall back to a shell
  writer.

Relay the command's own output verbatim:

- **Refused (exit 1):** print the refusal as it came and stop. Do not retry with
  `--confirm` unless the user has approved the gate in this message.
- **Accepted (exit 0):** print its `-> <state>` line (the `next:` line may follow).

Continue directly into the new state's work now — do not wait for the next user prompt.
