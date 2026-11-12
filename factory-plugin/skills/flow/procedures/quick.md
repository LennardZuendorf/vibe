# Quick procedure

## Inputs

Concrete bounded request, relevant lessons/spec, current cursor and affected checks.

## Prerequisites

Quick means at most two files, twenty changed lines and no interface, schema or
contract change. Count the actual diff, including required tests/docs. Unknowns,
a third file or contract drift require feature planning and its approval gate.
For a defect, use systematic debugging to reproduce and isolate the cause first.

## Artifacts

Keep the change minimal. Write a reproducing test before implementation for a
bug; observe red, apply the bounded fix, then observe green. Use existing tests
and a short runtime record. A Quick exception never authorizes commit or release.

## Checks

Run affected checks and inspect actual scope after the fix. Obtain independent
review when required by controlling rules. Record observed output and baseline
deltas. Report missing evidence. Do not claim done from rereading the patch.

## Recovery

A growing diff returns to feature planning before further writes. A failed check
returns to diagnosis and the fix state. Keep existing completed work. Follow
[recovery](../reference/recovery.md); do not broaden scope to repair unrelated failures.

## Example

A misspelled output string and its existing assertion may fit two files/twenty
lines. Adding a new flag changes a contract, so it uses feature planning even
when its implementation is short.
