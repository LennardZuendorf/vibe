---
type: feature-plan
feature: host-support
sibling: tech.md
parent: ../../plan.md
updated: 2026-10-01
---

# Feature: Native coding-host support — Implementation Plan

Deliver native skill and workflow entry points for Claude Code, OpenCode, Codex, and Factory Droid, then make one project installer provision and remove those surfaces safely. The work extends the package and install scope tracked by F21 and F22.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)

**Feature gate:** Starts when F8, F14, and F19 are `DONE`; reconcile this feature with F21/F22 in the root Feature Sequence during COMPOUND.

## F21/F22 Boundary

This feature delivers the four-host package and command surfaces tracked under F21, plus the project install, detection, root resolution, repeat, and uninstall behavior tracked under F22. It does not complete either tracking issue as currently written. New hook integrations for OpenCode, Codex, and Factory Droid, and native agent registration remain deferred; existing Claude Code hook behavior must be preserved. F22 release artifact builds, signing, and distribution remain outside this feature. Update the issue acceptance and root Feature Sequence during COMPOUND; leave each issue open until its remaining acceptance is delivered.

## Problem Frame

The current installer and native command packaging center on Claude Code, while OpenCode has only partial planned support and Codex and Factory Droid are absent from the host selector. Package the shared workflows in each host's documented project surfaces, then provision those files from one repeatable project-level install path.

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | [Native workflow discovery](product.md#requirement-native-workflow-discovery-r1) | host-support/1 |
| R2 | [Native workflow invocation](product.md#requirement-native-workflow-invocation-r2) | host-support/1 |
| R3 | [Single project installation](product.md#requirement-single-project-installation-r3) | host-support/2 |
| R4 | [Safe repeat and removal](product.md#requirement-safe-repeat-and-removal-r4) | host-support/2 |
| R5 | [Accurate capability reporting](product.md#requirement-accurate-capability-reporting-r5) | host-support/2, host-support/3 |

## Key Technical Decisions

1. **Keep workflow bodies shared.** Host-native skills and commands point to or package the same instructions and artifact paths; they do not fork behavior. See [tech.md](tech.md#host-surface-contract).
2. **Use documented project-level host surfaces.** Claude Code, OpenCode, Codex, and Factory Droid each get only supported skill and invocation mechanisms; Codex uses skill invocation without a synthetic slash command.
3. **Detect local hosts by their CLI.** The default configures only supported host CLIs found on `PATH`; explicit `--platform claude,opencode,codex,droid,none` selection overrides detection and follows the F22 installer interface. A plain workspace install uses the current directory when no Git root is found.
4. **Record installer ownership.** The project install manifest records the created host artifacts so repeat install and uninstall operate only on vibe-managed paths.
5. **Do not add hook integrations for the other hosts or agent registration.** Preserve and report existing Claude Code hook wiring accurately.

## Global Constraints

- Do not add hook integration files for OpenCode, Codex, or Factory Droid, or host-agent registration files.
- Preserve existing Claude Code hook behavior and report it only when its wiring is installed.
- Preserve the shared workflow source and `.spec/` and flow-state contracts.
- Installer operations must be deterministic, repeatable, and safe from a nested repository or worktree directory.
- Preserve user-owned files and settings; never remove shared-directory entries unless the install manifest proves vibe ownership.
- Use host fixture tests for file contracts; claim live host execution only when the actual CLI is available and exercised.
- Cite the unit ID in tests and commits.

## Unit IDs

Unit IDs are stable. Add new IDs if scope grows; do not renumber existing units.

### host-support/1 — Package native host skills and commands

**Goal:** Ship shared spec and flow workflows through all four hosts' documented project skill and invocation surfaces.

**Requirements:** R1, R2

**Dependencies:** —

**Files:**

```
spec/                              # native skill and plugin package files
flow/                              # native skill, command, and plugin package files
.claude-plugin/                    # Claude plugin manifest and marketplace if required
.opencode/                         # OpenCode native commands/config if required
.agents/plugins/                   # Codex project plugin registration if required
.factory-plugin/                   # Factory plugin manifest and commands
```

**Test scenarios:**

- Each host fixture discovers both shared workflows from the shipped project package.
- Each host exposes the documented invocation mechanism; Codex exposes skill invocation without a fabricated custom command.
- All entry points preserve common workflow instructions and artifact destinations.
- No new hook integration for OpenCode, Codex, or Factory Droid, or native agent registration is included; existing Claude Code hook behavior remains intact.

**Verification:** Host package fixtures assert manifest paths resolve to existing skill and command files and assert the supported host surface matrix is complete.

### host-support/2 — Extend the project installer lifecycle

**Goal:** Install, preview, repeat, and remove the selected native host surfaces from one project/worktree command.

**Requirements:** R3, R4, R5

**Dependencies:** host-support/1

**Files:**

```
install.sh                                  # target selection, root resolution, dry-run, install/uninstall
flow/tests/adapters/run.sh                  # installer and ownership fixtures
```

**Test scenarios:**

- Run from a nested repository path and a nested Git worktree; assert writes land at the correct root.
- Run from a plain workspace root without a Git ancestor; assert the current directory is the target.
- Detect each host with a CLI fixture and assert only detected hosts are configured by default.
- Explicitly select hosts with `--platform` while CLI detection is both present and absent; assert the explicit selection wins. Cover `none`.
- When no host CLI is detected, assert host-neutral workflows still install and the summary explains how to select a host.
- Dry-run reports planned paths without creating them.
- Reinstall produces byte-stable managed files and does not duplicate registrations.
- Uninstall removes only recorded vibe-owned artifacts and preserves unrelated skills, commands, settings, and files byte-for-byte.
- Completion output names installed capabilities, preserves Claude Code hook behavior, and does not claim hook support for other hosts or native agent registration.

**Verification:** `bash flow/tests/adapters/run.sh` passes with populated root-resolution and host-detection fixtures; install manifests contain at least one managed action per selected host before uninstall checks run.

### host-support/3 — Document host support and the install path

**Goal:** Explain the one-command project install, host invocation methods, selection, and deferred capabilities accurately.

**Requirements:** R2, R3, R5

**Dependencies:** host-support/1, host-support/2

**Files:**

```
README.md
flow/README.md
```

**Test scenarios:**

- Installation documentation covers Claude Code, OpenCode, Codex, and Factory Droid with their actual workflow invocation surfaces.
- The common install example works from a repository root and explains explicit host selection and uninstall.
- Documentation accurately describes retained Claude Code hook behavior and does not claim hook support for other hosts or native agent registration.

**Verification:** Documentation review against the host surface table in [tech.md](tech.md#host-surface-contract) and installer option help; `bash .agents/skills/spec/scripts/validate.sh` passes.

## Spec vs Implementation

| Gap | Tracked in | Notes |
|---|---|---|
| Current installer/package plan centers on Claude Code and OpenCode; Codex and Factory Droid are not supported in the selector. | host-support/1, host-support/2 | Existing F21/F22 docs are the primary package and install integration points. |
| Existing installer targets legacy Claude files and hook wiring. | host-support/2 | Reconcile with the planned v0.4 installer contract; hooks remain explicitly deferred here. |

## Open Questions

1. **Codex package-local skills.** During implementation, verify whether shared `.agents/skills/` discovery is sufficient for project use or whether a Codex plugin-local copy is required. If a copy is needed, generate or link it so workflow bodies cannot diverge.
