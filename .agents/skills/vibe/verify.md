# verify — evidence before completion

Verification is first-class here (the cheapest high-value steal from GSD: real
evidence, dedicated review, fix-plans before "done"). Caveman **full**.

Serves two states:

- `feature.verify` → `feature.compound` (pass) | `feature.impl` (targeted fix) |
  `feature.plan` (major drift)
- `quick.verify` → `idle`

## Procedure

1. **Locate.** Read `.agents/skills/vibe/state.json`; confirm you are in a `*.verify`
   state (else the caller transitions there first).
2. **Verify.** Delegate to `superpowers:verification-before-completion`. Run the
   real tests/build. For a feature, tie each result back to a plan **unit ID**.
3. **Review.** Delegate to `superpowers:requesting-code-review` and the
   `code-reviewer` subagent (confidence-filtered). Capture findings.
4. **On failure.** Delegate to `superpowers:systematic-debugging`; produce a
   fix-plan before any retry. Do not mark anything done on a failing check.
5. **Report + route.** Summarize evidence (commands run, output, review verdict).
   Then suggest the routing target. **Human gate** before shipping a feature.

## Rules

- Evidence is observed behaviour, not assertion. Show the command and its output.
- No spec writes from verify.
- Caveman full; security/irreversible actions in normal prose.
