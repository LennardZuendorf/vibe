---
id: flow.invariants
title: Write invariants
channels: [agents-md]
---
<!-- vibe:summary -->
write invariants (generated from `content/policy.json`, never hand-authored):
{{invariants}}
<!-- /vibe:summary -->

Every write this harness restricts is one rule in `content/policy.json`. The
list below is RENDERED from that file — the same data
`detect-context.sh decide` enforces — so the prose and the enforcer cannot
disagree. Edit the rules, never this text.

{{invariants}}

Check a path before writing it:

```bash
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>
```
