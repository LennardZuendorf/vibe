---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "docs/superpowers/**"
updated: 2026-10-19
---

# Safety

**Safety.** Today's behaviour of `promote`, `export`, `import` and
`check-drift`:

- Every read that feeds a write fails closed: a read error other than "not
  found" exits 1 and writes nothing (`promote`, `export`, `import`), also
  under `--dry-run`.
- `promote` writes are all-or-nothing: originals are kept in memory and
  restored if a later write fails. `--dry-run` runs the same plan and
  refuses exactly like the real run.
- `promote <f> --done` refuses (deletes nothing) if `docs`,
  `docs/superpowers`, `docs/superpowers/plans` or `docs/superpowers/specs`
  is a symlink. It binds artifacts by exact slug.
- `export` refuses a destination inside the spec root, a symlinked `<dir>`
  and an existing `<dir>/openspec/`. On a mid-write failure it removes only
  what it created.
- `promote` refuses to write through a symlinked target (also under
  `--dry-run`) and keeps the original file mode. A merge target and a delta
  target that are the same file under two paths (compared by identity) get
  one write.
- `promote <f> --done` also refuses in a shallow clone (no trustworthy branch
  base) and when a bound artifact is a symlink. `check-drift` reports a
  symlink under `docs/superpowers` as an artifact finding.
- `import` never overwrites an existing `features/<id>/` and never writes
  root documents. It refuses any symlink under the source `specs/` and a
  symlinked `features/`. Rollback removes only what it created.
- `check-drift --pre-merge` fails closed on an unreadable
  `docs/superpowers` subtree.
- `--json`: exit 0 always prints one JSON object. `--json` without a spec
  root exits 3 with empty stdout.
- Deliberate divergences from the bash oracles: root discovery walks up from
  the cwd; a symlinked `docs/spec` root is used; read errors fail closed where
  the oracle sometimes proceeded.
