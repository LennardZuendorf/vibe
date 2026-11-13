---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "spec/**"
  - ".vibe/spec.json"
  - "docs/spec/**"
updated: 2026-11-12
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

Legacy unregistered discovery keeps this order in one implementation. Registered
operations first resolve explicit project/native checkout/stored-branch selection
against project identity. Matching clones/worktrees select the same project;
ambiguity refuses. Repo-first uses selected or matching ambient checkout files.
Otherwise project-only access follows its registered main checkout's actual branch.
Global-first can select a stored branch outside code; code-sensitive operations
require matching files. `code_available` does not guarantee Git/history.

Code, spec and artifact anchors stay separate. `Root.project` is the matching
code project, never the global storage tree. Legacy `--root`/environment discovery
still applies where supported; a conflicting managed global override refuses.
Branch init separately selects exact stored base/target refs. It retains only a
validated matching caller code anchor, otherwise reports stored-only context; it
does not scan other clones or fall back to main. Normal Context rules stay intact.
See [Projects and MCP](spec-projects-mcp.md) for ownership and storage.

`docs/superpowers/` is the recognised repo-first artifact root (D33); global-first
uses its selected external artifact root. Both read
`specs/YYYY-MM-DD-<topic>-design.md` and `plans/YYYY-MM-DD-<topic>.md`,
binds them to a feature by topic slug, and deletes them only through
`promote <feature> --done`.
