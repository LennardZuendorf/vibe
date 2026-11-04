---
type: area
code:
  - "install.sh"
  - "flow/tests/adapters/plugin-install.sh"
  - "flow/tests/adapters/plugin-isolation.sh"
  - "flow/tests/adapters/global-install.sh"
  - "flow/tests/adapters/opencode-discovery.mjs"
updated: 2026-11-04
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

## Global installation

`install.sh --global` installs `spec`, `using-spec` and `vibe` into each selected host’s
user skill directory: Claude `~/.claude/skills`, Codex `~/.codex/skills`,
Droid `~/.factory/skills`, OpenCode `~/.config/opencode/skills`.
Host config environment overrides apply. `--host` selects hosts; otherwise
a host CLI or existing config directory enables detection. No host CLI is
required for copying skills. `--plugin` optionally installs the Claude plugin.
One marker-delimited block in `~/.agents/AGENTS.md` preserves surrounding
user content; each host still reads its own instruction file.

## Spec binary availability

Global install reports whether `vibe-spec` is available. `--build-spec-bin`
builds it and copies it to `~/.local/bin`; `--bin-dir` overrides that directory.
The installer leaves shell profiles untouched and reports a missing PATH entry.
Build failures warn and preserve the installed skills. Without the binary,
commands such as `root`, `validate`, `promote` and `init` are unavailable;
bundled scripts validate flat legacy roots only. Doctor reports this condition.

## Skill discovery

Project and global installs expose `spec`, `using-spec` and `vibe` as sibling
skill directories. Materialized host plugin packages use the same flat shape
and omit the nested `spec/using-spec` copy. In project installs, `--only flow`
excludes both spec skills; `--only spec` includes the router.

Native acceptance isolates every CLI call, including help, in a fresh home.
Legacy uses `debug skill --pure`; v2 retains one private localhost server and
awaits `model.list` before `skill.list`. Both assert all three exact installed paths.
Node doubles cover isolation, failures and cleanup even when OpenCode is absent.
