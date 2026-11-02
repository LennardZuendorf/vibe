---
type: area
code:
  - "install.sh"
  - "flow/tests/adapters/plugin-install.sh"
  - "flow/tests/adapters/plugin-isolation.sh"
updated: 2026-11-02
---

# Installer

## Product piece

| Piece | What It Owns | Lives In |
|---|---|---|
| installer | One POSIX `install.sh`: subsets, platforms, dry-run, uninstall | `install.sh` |

## Host plugin acceptance

Host-install tests clone the committed payload and run each host from a temporary
home with an explicit environment allowlist. Safe CLI doubles check inherited
config paths, credentials, failure branches, and cache corruption before live
probes run. Live probes compare both cached skill files with the clone's canonical
source. Claude and Codex caches pin the manifest version; Droid local caches pin
the clone's source revision. The local marketplace source directory is named
`vibe` so Droid registers the name used by the install command.
