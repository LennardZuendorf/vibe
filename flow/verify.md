# verify — evidence before completion

Verification is first-class here (the cheapest high-value steal from GSD: real
evidence, dedicated review, fix-plans before "done").

Serves two states:

- `feature.verify` → `feature.compound` (pass) | `feature.impl` (targeted fix) |
  `feature.plan` (major drift)
- `quick.verify` → `quick.fix` (findings) | `idle`

## Procedure

1. **Locate.** Read `.agents/skills/vibe/state.json`; confirm you are in a `*.verify`
   state (else the caller transitions there first).

2. **Verify.**

   > **Delegate — superpowers:verification-before-completion**
   > - announce: "delegating to `superpowers:verification-before-completion` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: run the real tests/build; for a feature, tie each result to a plan **unit ID**
   > - redirect: evidence is observed output shown in-turn; no spec writes
   > - skip: any "done"/commit claim on a failing check

3. **Receipt.** Write the evidence receipt —
   `.agents/skills/vibe/evidence/feature-<feature>.md` (or `evidence/quick.md`
   for `quick.verify`): the commands run + their observed output, plus a per-unit
   verdict for a feature. Runtime, not memory — the `evidence/` dir is gitignored.
   The Stop gate blocks any "done" in a `*.verify` state without a fresh receipt.
   Not verifying? Abort: `set-state.sh idle`.

4. **Review — one protocol.** Not two separate reviews: use
   `superpowers:requesting-code-review`'s dispatch pattern with feature-dev's
   `code-reviewer` as the reviewer template.

   > **Delegate — superpowers:requesting-code-review + code-reviewer**
   > - model: code-reviewer → opus (review) — pin explicitly, never inherit the default
   > - announce: "delegating review to `superpowers:requesting-code-review` (dispatch) with `code-reviewer` (reviewer template) — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: requesting-code-review's dispatch protocol; `code-reviewer`'s 0–100 confidence rubric; keep only findings at confidence ≥ 80
   > - redirect: findings route to `feature.impl` (feature) / `quick.fix` (quick) — NEVER fixed here; verify writes no `src/**`. COPY these redirect/skip lines into the reviewer (Task) prompt — subagents get no per-turn orders
   > - skip: the upstream "fix Critical immediately" step and any self-commit — routing owns the fix

5. **On failure.**

   > **Delegate — superpowers:systematic-debugging**
   > - announce: "delegating to `superpowers:systematic-debugging` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: produce a fix-plan before any retry; mark nothing done on a failing check
   > - redirect: the fix-plan routes to `feature.impl` / `quick.fix` — no writes here
   > - skip: applying the fix in verify — verify diagnoses and routes only

6. **Report + route.** Summarize evidence (commands run, output, review verdict),
   then route. The fix loops (`feature.impl`/`feature.plan`, `quick.fix`) are
   non-gated — advance immediately and continue. In `quick.verify`, if a durable
   lesson surfaced record it inline before going `idle` (see [quick.md](quick.md)
   step 5). The one gated edge is the ship gate
   (`feature.verify > feature.compound`): **Human gate** before shipping a feature.

## Rules

- Evidence is observed behaviour, not assertion. Show the command and its output.
- At a non-gated edge, advance immediately: `set-state.sh <next>`, announce in
  one line, continue. Stop and ask only at the ship gate (see state-machine.json).
- No spec writes from verify beyond the optional `quick.verify` lesson entry.
- Output density follows the machine's `style` note (see [SKILL.md](SKILL.md)
  § Style); security/irreversible actions in normal prose.
