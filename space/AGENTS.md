# Global workflow rules

Scope: method only — workflow, planning, tools, skills, subagents, output style.
Repo `AGENTS.md` files own repo specifics (commands, paths, conventions). Repo
files may tighten method; they may never weaken the gates below. On gate
conflicts: this file wins.

## Workflow

Follow the vibe flow model.

If the repo has `.agents/skills/vibe/`: the cursor `state.json` owns sequencing.
Read it, do the current state's orders, transition only via `/flow` or
`set-state.sh`. Never hand-edit `state.json`.

- File absent → idle: proceed with the specs; if none exist, ask.
- File present but corrupt or unreadable → stop and report. Do not guess.

Without vibe, run the same arc manually:

```
ASK → PLAN → CONFIRM → IMPL → VERIFY → COMPOUND
```

Task classes — apply the mechanical tests in order, no self-serve upgrades:

- **Big** — new subsystem or interface, or multi-checkpoint work. Full arc;
  spec skill writes the spec before code.
- **Medium** — multi-step, single track. Full arc; writing-plans skill writes
  the plan; present it at the gate.
- **Quick** — ≤2 files, ≤20 lines, no interface/schema/contract change.
  Triage → fix → verify. Skips both gates; verification still applies.

Two human gates: plan→impl and verify→ship (Quick excepted). IMPL starts only
after an explicit approval reply from the user in this conversation. Silence is
not approval. A change that outdates a doc or spec is not done until the doc is
fixed.

## Verification

- Run the checks the change affects (lint, types, tests). Show the real output
  before claiming done. No output, no claim.
- Prefer executable verification over self-review. A fresh test beats re-reading
  your own code.

## Compounding

- Read `.spec/lessons.md` at session start when present.
- Any failed command, reverted change, or late-found defect adds a lesson —
  including ones you caught yourself. Write it to the lessons file in the same
  cycle.
- Keep the repo AGENTS.md active-rules digest (top lessons) current in the same
  cycle.

### Active lessons

- Build valid structured fixtures first, then mutate one field per validation test.
- Use locale-independent comparisons for deterministic generated output.
- Test CLI output separately from orchestration return values.
- Use `lstat` and exclusive temporary file descriptors for declared regular outputs.
- Acquire exclusive locks before reading state used to build a write plan.
- Derive resumable migration state from all persisted markers, not one output file.
- Enforce generated-config ownership with explicit key allowlists and strict section types.

## Tools and MCP

Precedence: dedicated tool over bash emulation. If a tool exists for an action,
use the tool. Batch independent tool calls in parallel.

- **Serena** — symbol-level code work: find symbol, references, rename, file
  overview, symbol-body edits.
- **Grep / Glob / Read** — quick text search, file discovery, non-code files,
  whole-file reads.
- **context7** — resolve library/framework docs before writing or upgrading
  dependency code. Unprompted.
- **Exa** — primary web research: current events, post-cutoff facts, error
  research, ecosystem comparisons. Built-in websearch is the fallback.
- **indexed** — semantic search over the project's indexed collections when
  present.
- **rtk** — the plugin rewrites bash commands automatically. Rewritten commands
  are expected, not an error.

## Skills

Hard gate — invoke the matching skill before acting; do not improvise around it:

- brainstorming — before any behavior-, interface-, config-, or
  dependency-changing work, including refactors
- test-driven-development — before implementation code
- systematic-debugging — before any bug fix
- spec — for Big tasks
- verification-before-completion — before any done/complete/fixed claim

Any other matching skill: use it over improvising.

## Subagents

Standard dispatch set: code-architect, code-explorer, code-reviewer, plus
subagents shipped inside superpowers and spec skills. Never dispatch ce-*
agents. Prefer shared `code-*` names when available.

Delegate research, multi-file exploration (≥3 files or cross-module), and
independent tracks to subagents. Dispatch independent tracks in parallel; run
dependent steps sequentially. Keep same-file work in the main thread. Brief
each subagent on the task and the expected return format; synthesize results
yourself.

Choose an agent by capability before choosing the model role. Model roles
select provider, model, and effort; they do not provide system instructions or
replace agent prompts and skills.

| Work | Agent capability | Model role | Resolved model |
| ---- | ---------------- | ---------- | -------------- |
| architecture / detailed planning | shared `code-architect`; OMP `designer` | `@architect` | GPT-5.6 Sol |
| independent review / validation | shared `code-reviewer`; OMP `reviewer` | `@review` | Claude Opus 5 |
| codebase tracing / research | shared `code-explorer`; OMP `librarian` | `@workhorse` | GPT-5.6 Terra |
| fast read-only discovery | OMP `scout` or `sonic` | `@efficient` | GPT-5.6 Luna |
| general implementation | shared `general` or OMP `task` | `@workhorse` | GPT-5.6 Terra |
| passive advice only | OMP `advisor` | `@advisor` | Claude Opus 5 |

- Review and validation use `@review`; architecture and detailed design use
  `@architect`; normal implementation and complex exploration use
  `@workhorse`; mechanical edits and fast read-only discovery use `@efficient`.
- Use the lowest sufficient role and escalate only on failure. When unsure,
  default to `@workhorse`.
- If a listed model is unavailable, drop to a lower role. Never substitute
  upward.
- Only an explicit user request in the current session enables maximum effort.
- The advisor is passive and never replaces independent review.

## Output

Brief, simplified technical English. Minto pyramid: answer first, then detail.
One idea per sentence, active voice, no filler, no hedging, no analogies.
Quote commands, paths, diffs, and error lines verbatim; compress only the
surrounding prose. Never compress security warnings or irreversible-action
confirmations.

## Commits

Commit only on explicit request within the current session. Conventional
Commits, imperative mood, ≤50-char subject, no body or footer.
Example: `feat(flow): add hook wiring`.
