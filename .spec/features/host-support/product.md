---
type: feature-product
feature: host-support
sibling: tech.md
parent: ../../product.md
updated: 2026-10-01
---

# Feature: Native coding-host support — Product

vibe installs its reusable workflows and user-invoked entry points in the native project surfaces of Claude Code, OpenCode, Codex, and Factory Droid. One project install makes the same spec and flow workflows available across those hosts. This first slice covers skills, commands where the host supports a distinct command surface, and installation. It adds no hook integrations for OpenCode, Codex, or Factory Droid and no native subagent registration; existing Claude Code hook behavior remains.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)
**Plan:** [plan.md](plan.md)

## Scope

| | |
|---|---|
| **Owns** | Host-native skills and command entry points for Claude Code, OpenCode, Codex, and Factory Droid; one project/worktree installer path; install preview, repeat install, removal, tests, and user documentation. |
| **Does not own** | New hook integrations for OpenCode, Codex, and Factory Droid; native subagent/custom-agent registration; the internal behavior of spec, flow, or instruct; support for additional hosts. |

## Requirements

### Requirement: Native workflow discovery (R1)

The system SHALL make the spec and flow workflows discoverable through each supported host's documented project-level skill surface.

#### Scenario: Supported host opens an installed project

- **Given** the project installation completed for Claude Code, OpenCode, Codex, or Factory Droid
- **When** that host loads the project
- **Then** it can discover the installed spec and flow skills from a documented native or compatible skills path

### Requirement: Native workflow invocation (R2)

The system SHALL expose each included user-invoked workflow through the host's documented native invocation surface, using skills as the invocation surface when the host has no separate custom-command mechanism.

#### Scenario: User invokes a workflow in a supported host

- **Given** the project's native host files are installed
- **When** the user invokes the spec or flow workflow with that host's documented syntax
- **Then** it reaches the same workflow behavior and project artifact paths as the shared implementation

### Requirement: Single project installation (R3)

The system SHALL let a user install host-neutral workflows and supported native host surfaces with one invocation. It SHALL resolve the nearest repository/worktree root when run from within one; when no Git root exists, it SHALL use the current directory as the workspace root. By default, it SHALL configure only supported hosts detected on the machine. An explicit host selection MAY override detection.

#### Scenario: Install from a nested worktree directory

- **Given** a Git worktree with no prior vibe installation
- **When** the user runs the installer from a nested directory without an explicit target
- **Then** the installer resolves that worktree root, installs host-neutral workflows there, and configures only detected host surfaces

#### Scenario: Install from a plain workspace root

- **Given** a workspace directory with no enclosing Git repository or worktree
- **When** the user runs the installer from that workspace root
- **Then** the installer uses that directory as the target and configures only detected host surfaces

#### Scenario: Explicitly select hosts

- **Given** a valid project root
- **When** the user supplies an explicit selection of supported hosts
- **Then** the installer provisions that selection regardless of host detection and reports the selected hosts

### Requirement: Safe repeat and removal (R4)

The system SHALL make project installation repeatable and SHALL remove only vibe-managed files and registrations during uninstall, preserving user-owned files and settings.

#### Scenario: Reinstall an existing project

- **Given** a project with a completed vibe installation and user-owned host files
- **When** the user runs the same install again
- **Then** the managed surfaces are refreshed without duplicate registrations or changes to user-owned content

#### Scenario: Uninstall from a project

- **Given** a project with vibe-managed host files and unrelated host configuration
- **When** the user uninstalls vibe
- **Then** only vibe-managed artifacts are removed and unrelated configuration remains byte-identical

### Requirement: Accurate capability reporting (R5)

The installer SHALL report only capabilities actually configured for each host. It SHALL preserve existing Claude Code hook behavior and SHALL NOT claim hook or native subagent support for a host whose integration it did not install.

#### Scenario: User checks installed capabilities

- **Given** a project installed for any supported host, with the existing Claude Code hook integration retained
- **When** the installer reports completion or capabilities
- **Then** it identifies installed skill and invocation surfaces, reports Claude Code hooks only when their existing wiring is installed, and does not claim new hook integrations for OpenCode, Codex, or Factory Droid or native subagent registration

## Non-Goals

- New hook-based injection, guards, stop gates, or event handlers for OpenCode, Codex, or Factory Droid.
- Host-specific agent, droid, or subagent registration.
- Per-user/global installation; this feature targets a repository, worktree, or plain workspace root.
- Hosts beyond Claude Code, OpenCode, Codex, and Factory Droid.
