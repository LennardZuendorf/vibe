# Independent review contract

## Review brief

The controller supplies task IDs, reviewed plan/scope digest, candidate identity,
changed paths/diff, requirement references, actual check outputs and baseline
failures. The reviewer is independent from the implementer. Use the
[reviewer role](../agents/code-reviewer.md) with read-only scope and no gate rights.

## Review result

Return `pass`, `changes_required` or `blocked`; include each finding's severity,
path/line, requirement, observed impact and a concrete reproducer or check.
Separate baseline defects from introduced defects. Missing evidence is a gap,
not a pass. State what was examined and what remains unverified.

The controller checks findings and dispatches bounded corrections. Use receiving
code review techniques to verify a finding before applying it. Rerun affected
checks and independent review after candidate changes. Task review does not
replace final whole-candidate review after documentation preparation.

## Missing reviewer

If required independent review is unavailable, report `blocked` and the missing
capability. Continue independent authorized work. Do not relabel self-review as
independent review or authorize shipping from a worker's success claim.
