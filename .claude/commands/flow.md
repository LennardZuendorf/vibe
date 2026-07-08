---
description: Transition the vibe flow state machine to a new state
argument-hint: <flow.phase>
---

You have been asked to transition to state: $ARGUMENTS

Do exactly this, in order:

1. Read `.agents/skills/vibe/state.json` to find the current `<flow>.<phase>` state.
2. Read `.agents/skills/vibe/state-machine.json` to find that state's `next` array.
3. If `$ARGUMENTS` is NOT in `next`, refuse: print the current state, the legal `next` values, and stop. Do not write.
4. If `$ARGUMENTS` IS in `next`, transition by running `.agents/skills/vibe/scripts/set-state.sh $ARGUMENTS` — do not write `state.json` directly. `feature` is preserved unless the user asked to change it. There is no `notes` field.
5. Print a one-line confirmation: `→ <new_state>`.

Continue directly into the new state's work now — do not wait for the next user prompt.
