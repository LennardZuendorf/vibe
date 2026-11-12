<!-- vibe:global:start -->
## vibe — spec-first workflow harness

This machine has the **vibe** skills installed at user scope: `spec` (the durable
spec layer) and `vibe` (the flow that routes each phase). They are available in
every repository. A repository opts in fully with `./install.sh <repo> --local`,
which adds the stateful flow: the cursor, `/flow`, and the write/stop hooks.

Working model, every session:

```text
read the specs -> plan -> confirm -> implement -> verify -> record the lesson
```

- Read the spec root (`docs/spec/`, or a feature's own specs) before writing code.
  Sessions are ephemeral; the specs are the memory.
- Two edges need an explicit human yes: plan -> implement, and verify -> ship.
  Every other transition may proceed.
- Answer first, then the detail. One idea per sentence, active voice, no hedging.
  Compress receipts and summaries — never commands, paths, or security warnings.
- Verify before claiming done: run the suite, quote the output. "Should work" is
  not evidence.
- Record what was learned where the next session will find it: durable conclusions
  go in the spec tree, not in the transcript.

Nothing in this block is repository state; it is the same on every machine and is
replaced in place between the markers on re-install. Your own prose outside the
markers is never touched.
<!-- vibe:global:end -->
