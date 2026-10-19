---
type: hub
applies:
  - "contracts/**"
updated: 2026-10-19
---

# Cross-Tool Contracts — Tech

[tech.md](../tech.md)

Branch doc for the only coupling between the three tools: versioned files
under `.vibe/`, tested byte-exact by `contracts/run.sh` in every tool's CI.
A tool never imports a peer; it reads these files or execs one soft peer
call, probed, absence-is-degrade.

## Notes

- [The contracts directory](contracts/contracts-dir.md) — layout and the additivity rule
- [Payload v1](contracts/contracts-payload.md) — provider payload files and the per-session ledger
- [Provider manifest v1](contracts/contracts-provider-manifest.md) — command and file sources, trust gate
- [Marker grammar v1](contracts/contracts-marker-grammar.md) — managed-block markers and rules
- [Spec JSON v1](contracts/contracts-spec-json.md) — the nine queries, required sets, plan and feature answers
- [Exit codes and versioning](contracts/contracts-exit-codes.md) — exit-code contract, release tags, compatibility
