---
type: feature-tech
feature: host-support
sibling: product.md
parent: ../../tech.md
updated: 2026-10-01
---

# Feature: Native coding-host support — Architecture

The feature extends the existing three-tool package and installer design with project-level native skill and command carriers for Claude Code, OpenCode, Codex, and Factory Droid. Shared workflow instructions remain canonical; host files are adapters that point to or package those instructions. It preserves existing Claude Code hook behavior but adds no new hook integration for OpenCode, Codex, or Factory Droid and no native agent registration.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Design:** [design.md](design.md)
**Plan:** [plan.md](plan.md)

## Existing surfaces

| Path | Current role |
|---|---|
| `install.sh` | Bash installer; resolves a project root, copies `.agents/skills/{spec,vibe}`, links Claude skills, installs Claude commands/hooks, and supports dry-run/uninstall. The planned v0.4 replacement is in F22. |
| `flow/reference/adapters.json` | Instruction-file aliases (`CLAUDE.md`, `WARP.md`), not runtime adapters. |
| `.claude/commands/` | Current `/flow` command files. |
| `flow/scripts/merge-settings.sh` | Merges only the current Claude hook registrations. Hooks are outside this feature. |
| `flow/tests/adapters/run.sh` | Current install, adapter, and uninstall behavior tests. |
| `README.md`, `flow/README.md` | Current installation and host support claims. |
| `docs/roadmap/issues/21-F21-packaging.md`, `docs/roadmap/issues/22-F22-release-install.md` | Planned Claude/OpenCode package and installer surfaces; both need host-support scope reflected before implementation. |

## Host surface contract

| Host | Skills | User-invoked entry point |
|---|---|---|
| Claude Code | Project plugin `skills/` and `.claude/skills/` compatibility path | Native command files in the package `commands/` directory |
| OpenCode | `.agents/skills/` or `.opencode/skills/` | Native project command files under `.opencode/commands/` |
| Codex | Project skills under `.agents/skills/` and/or the Codex plugin package `skills/` | Invoke the skill through Codex's skill picker/mention syntax; do not invent a separate slash-command mechanism |
| Factory Droid | Native plugin `skills/` or `.factory/skills/`; `.agents/skills/` is also a documented compatibility path | Native plugin `commands/` or `.factory/commands/` |

Each host entry point delegates to the same workflow instructions and preserves the shared `.spec/` artifact destinations and flow state contract. The package adapter may carry host-required frontmatter or wrapper text; workflow behavior must not fork by host.

The Codex package uses the portable Agent Plugins manifest, with a Codex compatibility overlay only where the documented install surface requires it. The Factory package uses `.factory-plugin/plugin.json`; its marketplace lists the project plugin. OpenCode uses its project command/skill discovery and a local plugin only where the current package contract requires it. Claude keeps the existing plugin command and skill surfaces.

## Installer contract

Extend the planned F22 `install.sh` host selector to accept `claude`, `opencode`, `codex`, `droid`, and `none`. A project install resolves the nearest repository/worktree from a nested current directory. If no Git root exists, the current directory is the workspace root. Its selected host actions are recorded in the install manifest so repeat installation is idempotent and uninstall removes only the recorded vibe artifacts. The `none` target installs host-neutral skills and instructions without native host registration.

By default, detect host CLIs on `PATH` and configure only those hosts. An explicit host selection overrides detection. If no supported host is detected, install host-neutral files and report that no native host surface was configured, with the explicit selection syntax.

## Files

```
install.sh                         # planned F22 installer; expand host selection and lifecycle
spec/, flow/, instruct/            # planned F21 tool packages; add Codex and Factory carriers
flow/tests/adapters/run.sh         # extend install, host-matrix, and uninstall fixtures
README.md                          # host support and one-command install guidance
flow/README.md                     # flow entry-point and capability matrix
docs/roadmap/issues/21-F21-packaging.md # sync package scope and acceptance
docs/roadmap/issues/22-F22-release-install.md # sync host selector and install matrix
```

Host-specific package manifests and command files belong inside the planned F21 package layout; keep the shared skill source single-copy. Do not add hook integrations for OpenCode, Codex, or Factory Droid, subagent definitions, or droid definitions in these packages in this feature. Preserve the existing Claude Code hook wiring.

## Verification surface

- Fixture each host package and assert that its native manifest resolves the intended skill and command files.
- Exercise installer selection for each host, all hosts, and `none` from a nested directory and a worktree.
- Assert reinstall leaves the target byte-stable and uninstall removes only installer-owned artifacts while preserving user files in shared directories and user settings.
- Validate host invocation contracts from their shipped files; tests must not claim live host execution unless run against the actual CLI.
- Run `bash tests/run.sh`, `cargo test --workspace`, and `bash .agents/skills/spec/scripts/validate.sh` when the feature is implemented.

## Open Questions

1. **Codex skill packaging.** Decide during implementation whether the shared project skills alone satisfy Codex discovery or whether each plugin package must also carry a plugin-local skills copy; avoid maintaining divergent bodies either way.
