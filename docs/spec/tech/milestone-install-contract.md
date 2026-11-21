---
type: tech
applies: ["install.sh", "crates/vibe/**"]
updated: 2026-11-21
---

# One-command installer and host adapter contract

The installer makes a versioned Vibe CLI available and applies the selected
host components. A host adapter owns host-specific files and capability checks.

## Contracts

- Resolve platform and architecture. Verify expected artifact identity before
  publication to the install location. Report unsupported targets and
  preserve a working install on failed upgrades.
- Keep stable config entry points and use existing
  [projection ownership](cli/cli-store.md). Keep `vibe config` and `vibe hook session-start` compatible. Register owned files, configuration
  keys, links, hooks and plugin registrations. Recheck ownership before
  updating or removing them. Preserve user changes and explain conflicts.
- Track capabilities by host, runtime, version and distribution channel.
  Distinguish installed configuration, loaded runtime behavior and consent.
  Never convert install success into a claim that hooks can enforce Flow.
- Each adapter verifies MCP invocation, skill and instruction discovery,
  plugin load, and applicable prompt or tool hook behavior in a clean profile.
  Verify an upgrade/reinstall and show uninstall leaves unrelated settings.
- Credential references are resolved for the target host. Keep credentials
  out of release artifacts and remote configuration discovery.

## Open design

[#113](https://github.com/LennardZuendorf/vibe/issues/113) chooses the reference
host, publication method and verified artifact format. Host issues own native
support profiles. [Remote config discovery](https://github.com/LennardZuendorf/vibe/issues/118)
does not gate installation. Independent release-train changes require a separate
decision.

[Milestones](../concepts/product-milestones.md) · [Product requirements](../concepts/milestone-cli-installation.md)
