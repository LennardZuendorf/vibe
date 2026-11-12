---
id: delegation.subagents
title: Delegating to sub-agents
channels: [agents-md]
---
<!-- vibe:summary -->
delegation: choose capability before model; set an available allowed model explicitly under controlling user/repository preferences; bounded briefs and independent review; no silent forbidden fallback
<!-- /vibe:summary -->

Choose the needed capability before its provider or model. Apply controlling user
and repository model preferences. Set an allowed available model explicitly for
all delegated work; do not infer one from the role name. Report unavailable
required capabilities instead of silently substituting a forbidden model.

Workers receive exact scope, dependencies and checks. The controller owns state,
progress, gate decisions and independent review. A worker returns checked-result
claims; it cannot widen scope, approve itself, commit, clean or finish a branch.
