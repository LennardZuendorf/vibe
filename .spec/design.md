---
type: entrypoint
scope: design
design_format: google-labs-code/design.md-inspired
children: []
updated: 2026-09-04
---

# vibe — Design

Cross-cutting design language for the personal coding-agent toolkit. This is not a visual product; design here means interaction shape, information hierarchy, agent-facing tone, and the ergonomics of moving through work across three independent tools.

For UI-heavy projects, `.spec/design.md` should follow or reference the
google-labs-code [`DESIGN.md`](https://github.com/google-labs-code/design.md)
pattern: design tokens in YAML frontmatter plus markdown rationale. vibe
uses the same idea, but this repo's current design doc is prose-first because
the product is an agent workflow rather than a visual interface.

**Product:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Design Principles

1. **The current state should be obvious.** An agent and a human should be able to tell what flow is active, what phase comes next, and which files are in scope without reconstructing history.
2. **The workflow should feel guided, not trapped.** Warnings and legal next states should make the right path easy while preserving escape hatches for recovery.
3. **Delegation should be explicit.** When `flow` uses another skill, it names the output path and expected artifact so the delegated skill cannot invent its own file layout.
4. **Adapter copy should be boring.** Codex, Claude Code, and OpenCode wording may differ, but their rules should point back to the same tool and spec-root contracts.
5. **Each tool must feel whole alone.** Running `spec`, `flow`, or `instruct` by itself should never look like a piece is missing — the other two are enhancements, not prerequisites.
6. **The gate is a mechanism, not a request.** A human gate is enforced by the state-machine writer refusing an unconfirmed edge, never by a hook or prose asking nicely.

---

## Interaction Conventions

- Use `vibe-flow`, `vibe-spec`, and `vibe-instruct` for their recurring workflows; each is a complete CLI (and, for `flow`, a TUI) on its own.
- Use `.vibe/run/` for runtime state (cursor, ledger, payloads) and the spec root (`docs/spec/`, or `.spec/` today) for durable memory.
- In agent-facing prompts, phrase constraints as positive targets first: "write only these paths" before "do not write elsewhere."
- End each phase with a concise receipt: changed files, verification evidence, and next legal transition.

---

## Information Hierarchy

| Surface | Primary Question It Answers |
|---|---|
| `AGENTS.md` / `CLAUDE.md` (per-owner managed blocks) | How should this runtime behave in this repo, by whose rule? |
| `.vibe/*.json` (`spec.json`, `flow.json`, `instruct.json`) | How is each tool configured in this repo? |
| `.vibe/run/` (cursor, ledger, payloads) | What phase are we in, and what just happened? |
| `docs/spec/**` (or `.spec/**` today) | What are we building, why, how, and what remains? |
| The three skills (`spec`, `flow`, `instruct`) | What should the agent do right now, for which tool? |
| The `flow` TUI | What are the live cursor, legal next states, and recent history, at a glance? |

---

## TUI Conventions

`vibe-flow tui` is the one interactive surface in the toolkit. It follows the same conventions everywhere so a returning user never re-learns it:

- **One term per concept.** "State" always means the cursor's current phase; "transition" always means a legal move to a next state; a "signal" is always advisory, never a verdict.
- **Keys.** `r` starts recording a workflow, `n` adds a note to the recording, `s` saves it, `a` aborts the current action — mnemonic letters, each with exactly one meaning.
- **Gated transitions prompt `y/N`** before crossing a human gate — the same confirmation the CLI's `--confirm` flag grants, never a silent auto-yes.
- **The live pane** shows the cursor, legal next states with gate marks, the current orders, the last 20 ledger events, and one doctor line — polled every 500 ms off the ledger's mtime, no push channel.

---

## Spec Document Ergonomics

The spec framework should feel like a small map, not a documentation maze:

- Root docs answer project-level questions; feature docs answer one buildable unit.
- Branch docs are rare — only for concerns spanning multiple features.
- `design.md` is first-class when UX, interaction, language, or workflow ergonomics matter; omit token groups for non-visual workflow design.
- Feature authoring follows the 6-step interview flow in `spec`'s `feature.md` (locate → WHAT → rigor gate → HOW → plan → skip).
- Requirements adopt OpenSpec's grammar for interchange: `### Requirement: <ID> — <name>` (SHALL/MUST), `#### Scenario:` with GIVEN/WHEN/THEN bullets, and `## ADDED|MODIFIED|REMOVED Requirements` delta headers for feature proposals. The document model — root vs. feature vs. branch docs — stays vibe's own; only the requirement grammar is shared with OpenSpec.
- The spec root defaults to `docs/spec/`; `.spec/` still resolves for repos that haven't migrated.

---

## Feature Design Index

No feature designs are open. Feature `design.md` fragments are branch-scoped and were removed when their features compounded; the cross-cutting design language lives in this document.
