---
name: vibe
description: |
  Drive this repo's vibe flow: match the request to one flow, then run it state
  by state to done. Invoke BEFORE reading files, writing code, or planning.
  Enter from idle on: "set up vibe"/"repair the harness" → setup; "project
  direction"/"refocus"/"roadmap" → strategy; "build/add/implement <named
  feature>" → feature; "quick fix"/"just fix it"/"one-line fix"/"add a
  --flag"/"fix this typo"/"rename X" → quick. Mid-flow on: "verify"/"is it
  done"/"review before ship"; "wrap up"/"record the lesson"/"finish the
  branch" → compound. Also on "what should I do next", any /flow, GSD, or
  vibe-flow state, or a live .vibe cursor.
user-invocable: true
argument-hint: "[setup|strategy|feature|quick|verify|compound]"
allowed-tools: Read, Edit, Write, Bash
compatibility: Requires bash + jq. macOS and Linux.
metadata:
  author: lennarddib
  version: "2.0"
---

# vibe — flow

## Contract

The cursor owns sequencing and artifact paths; state orders override delegated skill instructions. Scope edits do not change state. At non-gated edges, run `vibe-flow state set <next>` yourself. The only human gates are `feature.plan>feature.impl` and `feature.verify>feature.compound`; cross them only after explicit approval with the `confirm` token or `--confirm` flag. Never hand-edit `.vibe/run/flow/state.json`; `idle` is always legal and aborts a flow. `/flow` and GSD aliases are thin wrappers over `vibe-flow state set`.

## Delegation protocol

Announce and delegate each state procedure, or self-execute it if the delegate is unavailable or the user chooses self. Pin a model tier on every dispatch: `sonnet` for implementation and trace work; `opus` for design and review. Copy the state's orders into each subagent prompt; subagents receive no per-turn orders.

## Flows

## setup

Audit repo files, harness data, the managed instructions block, and adapters; report missing and present items and preflight plugins. Apply only requested setup, use spec for the baseline root, merge without clobbering, and regenerate active rules.

## strategy

Read `lessons.md` first and use brainstorming to shape direction. In `strategy.spec`, update root `docs/spec/{product,tech,design,plan}.md`, validate, and record a durable lesson when one surfaced.

## feature

For a named feature, read lessons and root product/tech, then create the design through `superpowers:brainstorming`; use `code-explorer` for trace and `code-architect` for needed sketches. Create the plan through `superpowers:writing-plans`. Plans link with `Spec:` to the design and define verification for each task. Executors are `superpowers:executing-plans` or `superpowers:subagent-driven-development`; implement with TDD and use `superpowers:receiving-code-review` on review re-entry. Verify each plan task, use `superpowers:systematic-debugging` for failed checks, record evidence, and route findings to implementation or planning. On completion, promote durable facts with `vibe-spec promote <feature> --done`, regenerate active rules, and finish the branch last.

## quick

Read lessons and classify the request. For defects, use systematic debugging to diagnose before the fix; otherwise scope the bounded change and its verification. Keep quick work to three files with no unknowns. If scope grows, announce and confirm escalation to `feature.design`. Apply TDD with one reproducing test for bug-shaped changes; on review re-entry, use `receiving-code-review`. Verify and review the change, record evidence, and add a tagged durable lesson only when one surfaced.

## verify

Run the real checks and review the change. Record observed commands and output in the state receipt with verdict `passed`, `gaps_found`, or `human_needed`. Write no source. Route targeted findings to the fix state and design drift to planning.

## compound

Append a tagged durable lesson, promote durable facts, regenerate active rules, and delegate branch finishing last because it merges.

<!-- vibe:doctrine -->
vibe flow — working model. The sessions are ephemeral. Read `lessons.md` and `plan.md`, then active design and plan. Missing `.vibe/run/flow/state.json` means idle.

Orders own sequencing and paths; delegates own method. Never edit `.vibe/run/flow/state.json` by hand; use `vibe-flow state set`.

At non-gated edges, run `vibe-flow state set <next>` yourself. Gates: `feature.plan>feature.impl` and `feature.verify>feature.compound` require human approval and `--confirm`.

Output: brief technical English; answer first, one idea per sentence. Keep security and irreversible warnings in full; keep code, paths, commands byte-exact. Check policy with `vibe-flow policy decide`.
<!-- /vibe:doctrine -->

<!-- vibe:orders:setup.detect -->
skill=vibe · read-only repo+harness audit (.agents/flow machine+policy+skill, AGENTS.md block, adapters) · report missing/present · preflight plugins · write nothing · done → vibe-flow state set setup.apply
<!-- /vibe:orders -->

<!-- vibe:orders:setup.apply -->
skill=vibe · apply requested setup: merge instructions, .agents/flow data, baseline spec root, opt-in adapter symlinks · never clobber (diff+ask) · regen active rules · done → vibe-flow state set idle
<!-- /vibe:orders -->

<!-- vibe:orders:strategy.brainstorm -->
skill=vibe · delegate superpowers:brainstorming (sonnet) · read spec-root lessons.md first · scratch only, no writes · done → vibe-flow state set strategy.spec
<!-- /vibe:orders -->

<!-- vibe:orders:strategy.spec -->
skill=vibe · delegate spec (sonnet, root specs only) · write docs/spec/{product,tech,design,plan}.md · validate · durable lesson? append tagged lessons.md + regen rules · done → vibe-flow state set idle | iterate: strategy.brainstorm
<!-- /vibe:orders -->

<!-- vibe:orders:feature.design -->
skill=vibe · delegate superpowers:brainstorming (opus, topic=<feature>) · read lessons.md + root product/tech · write docs/superpowers/specs/*-<feature>-design.md only · done → vibe-flow state set feature.plan
<!-- /vibe:orders -->

<!-- vibe:orders:feature.plan -->
skill=vibe · delegate superpowers:writing-plans (sonnet) · write docs/superpowers/plans/*-<feature>.md (Spec: → design doc) · verify each task · human gate · approval → vibe-flow state set feature.impl --confirm
<!-- /vibe:orders -->

<!-- vibe:orders:feature.impl -->
skill=vibe · delegate executing-plans (sonnet, interactive) | subagent-driven-development (sonnet, handover) + TDD · write src/**, tests/** · freeze docs/spec/** + docs/superpowers/** · cite plan task numbers · review re-entry: receiving-code-review · done → vibe-flow state set feature.verify
<!-- /vibe:orders -->

<!-- vibe:orders:feature.verify -->
skill=vibe · delegate superpowers:verification-before-completion + superpowers:requesting-code-review + code-reviewer (opus) · evidence per task → .vibe/run/flow/evidence/feature-<feature>.md (frontmatter verdict: passed|gaps_found|human_needed) · write no src · human gate · ship → vibe-flow state set feature.compound --confirm | fix → feature.impl | drift → feature.plan
<!-- /vibe:orders -->

<!-- vibe:orders:feature.compound -->
skill=vibe · append tagged docs/spec/lessons.md · vibe-spec promote <feature> --done · regen active rules · delegate superpowers:finishing-a-development-branch (opus) last · done → vibe-flow state set idle
<!-- /vibe:orders -->

<!-- vibe:orders:quick.triage -->
skill=vibe · read lessons.md · defect → superpowers:systematic-debugging (opus, diagnose only) | other → self-scope · ≤3 files, no unknowns → fast path; escalate to feature.design: announce + ask first · done → vibe-flow state set quick.fix
<!-- /vibe:orders -->

<!-- vibe:orders:quick.fix -->
skill=vibe · delegate superpowers:test-driven-development (sonnet; bug: reproducing test first) · write src/** (+ optional docs/spec/quick/<slug>.md) · no root spec writes · review re-entry: superpowers:receiving-code-review · done → vibe-flow state set quick.verify
<!-- /vibe:orders -->

<!-- vibe:orders:quick.verify -->
skill=vibe · delegate superpowers:verification-before-completion + code-reviewer (opus) · evidence → .vibe/run/flow/evidence/quick.md (frontmatter verdict) · durable lesson? append tagged lessons.md + regen rules · findings → vibe-flow state set quick.fix | else → vibe-flow state set idle
<!-- /vibe:orders -->
