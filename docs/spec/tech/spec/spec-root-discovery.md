---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "spec/**"
  - ".vibe/spec.json"
  - "docs/spec/**"
updated: 2026-10-19
---

# Root and discovery

Default root `docs/spec/`; `.spec/` is legacy and stays supported. <!-- literal -->
Discovery, first hit wins:

1. `--root` flag
2. `$VIBE_SPEC_ROOT`
3. `.vibe/spec.json` key `root`
4. `docs/spec/`
5. `.spec/` <!-- literal -->
6. repeat 4–5 per parent directory, up to the one holding `.git`
7. nothing found → exit 3 (no spec tree; see [tech-contracts.md](../contracts.md))

One function implements this order; every subcommand, hook, and peer call
goes through it. `vibe-spec root [--json]` is the one answer peers call —
if the binary is absent, a peer runs the same order via a vendored
resolution routine, so the order exists in one place in code, never
duplicated in prose.

`docs/superpowers/` is a recognised artifact root (D33): `vibe-spec` reads
`specs/YYYY-MM-DD-<topic>-design.md` and `plans/YYYY-MM-DD-<topic>.md`,
binds them to a feature by topic slug, and deletes them only through
`promote <feature> --done`.
