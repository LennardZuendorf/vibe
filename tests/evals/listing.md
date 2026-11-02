<!-- vibe-spec: literal-file -->
# Selection-eval listing (frozen fixture)

Dated: 2026-10-24. Source: the skill listing of a Claude Code session with the
superpowers, feature-dev and code-review plugins installed. Descriptions of
`spec`, `vibe` and `using-spec` are copied from this repo at fd6ce39 (the
"today" side of every old-versus-new eval). A candidate description replaces the
one named `spec`, `vibe` or `using-spec` and nothing else changes, so old and new
meet the same competitors. Re-copy this file whenever the evals re-run
(tech.md, Selection evals).

Format: `- name: description`, one skill per line.

## Under test

- spec: You MUST use this before changing what the product does, how it is built, or any file under docs/spec, .spec or docs/superpowers — features, <!-- literal --> refactors, architecture, decisions, PRDs, plans, roadmaps, and also one-line fixes, typos and new flags. "Quick" and "just fix it" still route here first. Invoke it before reading files, before writing code, before brainstorming alone, before answering "how should we build X", and whenever the user mentions spec, PRD, design doc, plan, or roadmap.
- vibe: Vibe workflow router — one skill for the whole flow. Routes to the right phase for the current state: setup, strategy, feature, quick, verify, compound. Trigger on: any vibe flow state, "what should I do next", flow navigation, set up vibe, project direction/strategy, build a feature, quick fix, verify/evidence, compound/wrap up, or user says vibe.
- using-spec: Use when starting any session in a repo that keeps a spec, before any response or action, including clarifying questions.

## Competitors

- superpowers:brainstorming: You MUST use this before any creative work - creating features, building components, adding functionality, modifying behavior, or planning anything new, in software or out of it (a talk, a business, a renovation).
- superpowers:diagnosing-superpowers: Use when a superpowers session went wrong and your human partner wants to know why — repeated work, ignored plans, stumbles, poor results, a skill that didn't fire, "it took too long", "why is it so expensive", "what is it doing" — or wants to build a bug report for the superpowers maintainers, for the current session or a past one identified by id or path, on any harness.
- superpowers:dispatching-parallel-agents: Use when facing 2+ independent tasks that can be worked on without shared state or sequential dependencies
- superpowers:executing-plans: Use when executing an implementation plan in the current session as the implementer yourself — your human partner chose inline execution, or no subagent tool is available
- superpowers:finishing-a-development-branch: Use when implementation is complete, all tests pass, and you need to decide how to integrate the work
- superpowers:receiving-code-review: Use when receiving code review feedback, before implementing suggestions, especially if feedback seems unclear or technically questionable - requires technical rigor and verification, not performative agreement or blind implementation
- superpowers:requesting-code-review: Use when completing tasks, implementing major features, or before merging to verify work meets requirements
- superpowers:subagent-driven-development: Use when executing implementation plans with independent tasks in the current session
- superpowers:systematic-debugging: Use when encountering any bug, test failure, or unexpected behavior, before proposing fixes
- superpowers:test-driven-development: Use when implementing any feature or bugfix, before writing implementation code
- superpowers:using-git-worktrees: Use when starting feature work that needs isolation from current workspace or before executing implementation plans - ensures an isolated workspace exists via native tools or git worktree fallback
- superpowers:using-superpowers: Use when starting any conversation - establishes how to find and use skills, requiring skill invocation before ANY response including clarifying questions
- superpowers:verification-before-completion: Use when about to claim work is complete, fixed, or passing, before committing or creating PRs - requires running verification commands and confirming output before making any success claims; evidence before assertions always
- superpowers:writing-plans: Use when you have a spec or requirements for a multi-step task, before touching code
- superpowers:writing-skills: Use when creating new skills, editing existing skills, or verifying skills work before deployment
- feature-dev:feature-dev: Guided feature development with codebase understanding and architecture focus
- code-review:code-review: Code review a pull request
