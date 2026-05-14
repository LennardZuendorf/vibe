---
description: Transition the code flow state machine to a new phase
argument-hint: <phase>
---

You have been asked to transition to phase: $ARGUMENTS

Do exactly this, in order:

1. Read `.agents/flow/state.json` to find the current phase.
2. Read `.agents/flow/state-machine.json` to find that state's `next` array.
3. If `$ARGUMENTS` is NOT in `next`, refuse: print the current phase, the legal `next` values, and stop. Do not write.
4. If `$ARGUMENTS` IS in `next`, write the new cursor to `.agents/flow/state.json`, preserving `feature` and `notes` unless the user asked to change them.
5. Print a one-line confirmation: `→ <new_phase>`.

Do NOT start doing the new phase's work in this turn. The user will prompt next; the inject hook will tell you what to do then.
