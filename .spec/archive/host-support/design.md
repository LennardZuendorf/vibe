---
type: feature-design
feature: host-support
sibling: product.md
parent: ../../design.md
updated: 2026-10-01
---

# Feature: Native coding-host support — Design

**Parent:** [../../design.md](../../design.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

## Install flow

The user runs one install command from a repository, worktree, or plain workspace root. The installer resolves the nearest repository/worktree root; when none exists, it uses the current directory as the workspace root. It previews or installs host-neutral files and native files for detected hosts, then reports each configured host and its available workflow entry point. Explicit host selection overrides detection.

The host selector remains optional for the common path. Detection checks supported host CLIs on `PATH`. If none are detected, the installer still installs host-neutral workflows and reports how to select hosts explicitly. `--dry-run` shows the exact project paths and registrations before writing. Re-running refreshes vibe-owned files without duplicating user configuration. Uninstall removes only those managed files and registrations.

## Invocation map

| Host | How the user starts a workflow |
|---|---|
| Claude Code | Host-native slash command; skill remains discoverable |
| OpenCode | Host-native slash command; skill remains discoverable |
| Codex | Native skill picker/mention; no synthetic slash-command file |
| Factory Droid | Host-native slash command or skill token |

Every entry point uses the same workflow name and instructions where the host supports them. Codex uses its native skill invocation as the equivalent user entry point.

## User-owned configuration

Installation reports the host-specific paths it created or merged. It preserves unrelated command, skill, marketplace, and settings entries. A collision with an existing user-owned entry is reported with a safe recovery path instead of silently replacing the entry.

## Deferred capabilities

The install summary reports existing Claude Code hook wiring when it is installed. It does not imply hook support for OpenCode, Codex, or Factory Droid, or native subagent registration.
