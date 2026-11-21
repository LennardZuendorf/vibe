---
type: concept
updated: 2026-11-21
---

# One CLI and six-host installation

A developer on a clean machine can install Vibe with one command, enable the intended hosts and see which features work on each host.

## Requirements

### Requirement: MS-INSTL-01 — Install a versioned CLI
The reference install SHALL provision a verified Vibe CLI without a local Rust toolchain. It SHALL support upgrades and keep the CLI usable when optional host components are absent.

#### Scenario: Install on a clean machine
- **GIVEN** a supported OS and no Vibe or Rust toolchain
- **WHEN** the developer runs the documented install command
- **THEN** Vibe is available at the selected version and can report its installed components

### Requirement: MS-INSTL-02 — Provision and verify each host
Vibe SHALL provision MCP definitions, skills, hooks, plugins and instructions through supported surfaces for Claude Code, Codex, OpenCode, Factory Droid, Hermes Agent and OpenClaw. Doctor SHALL distinguish active, unsupported and awaiting-consent capabilities.

#### Scenario: A hook needs native consent
- **GIVEN** a supported host has loaded a plugin but is awaiting hook consent
- **WHEN** the developer runs doctor
- **THEN** it identifies the exact pending capability and the host action required before calling it active

### Requirement: MS-INSTL-03 — Preserve host ownership
Reinstall, upgrade and uninstall SHALL preserve user-owned configuration. Vibe SHALL report conflicts and remove only installation effects it can prove it owns.

#### Scenario: Uninstall after a user edits the host profile
- **GIVEN** a Vibe-managed adapter and unrelated user entries
- **WHEN** Vibe is uninstalled
- **THEN** Vibe-owned effects are removed where safe, and user entries and divergent edits are retained or reported

## Acceptance scope

Prove the reference installer first. Full milestone acceptance requires an executable host/version/runtime matrix and a clean-profile journey for each of the six hosts.

[Milestones](product-milestones.md) · [Technical contract](../tech/milestone-install-contract.md)
