---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "spec/**"
  - ".vibe/spec.json"
  - "docs/spec/**"
updated: 2026-10-19
---

# Migrate

`vibe-spec migrate --to <dir> [--layout layered] [--dry-run] [--check] [--json]`
moves the spec root (`git mv` when tracked), writes `root` into
`.vibe/spec.json` and rewrites legacy-root literals across a configured file
set. The set is the default globs plus `migrate.include`, minus
`migrate.exclude`; symlinks are never followed. A **population assertion**
guards it: literals before at least 1 (only when the root moves), files
scanned at least `migrate.floor`, literals after 0. A miss reverts and exits 1.

Any failure restores originals, removes created files and reverses moves.
`migrate` also reverts when it introduces validate errors; pre-existing
errors are only noted. A second run prints `migrate: already migrated`.
`--check` writes nothing and exits 1 while literals remain: the zero-literal sweep.
After the write, `migrate` inserts `<!-- vibe-spec: allow SF-nn -->` markers
for SF22, SF30, SF31, SF33 and SF34 warnings (SF32 is not included) and lists them. It bumps `updated:` on every
doc it touches. Lines tagged `<!-- literal -->`, `// literal` or `# literal`
at the line end are exempt, as is a file with `vibe-spec: literal-file` in its first 5 lines.

Deviations from the plan text:
- The literal regex does not match after a `.` or before `.` plus a word char (`..spec`, `.spec.ts`).
- Relative links are re-resolved when the root moves, in moved notes and in links to them.

`--layout layered`: `tech-<t>.md` becomes `tech/<t>.md`, each D-record
becomes `decisions/dNN-<slug>.md`, and `type: entrypoint` becomes `type: hub`.
Only decision files are created; hubs are not seeded. `product-` and `plan-`
topics refuse. Splitting prose into notes is agent-led.

Known limits:
- A move runs only from the legacy root; any other source root, or both roots present without a move, exits 1.
- `migrate` is untested on Windows.
- P4 code has not run on Windows CI until the first push.
- No guard against a panic or signal mid-run; recover through git.
- Feature docs only warn on unclosed frontmatter; root docs error (SF20).
- `AGENTS.md` keeps a file-level `literal-file` marker and the rendered template is exempt from the sweep, until P5.
