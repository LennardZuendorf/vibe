# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-03

First public release of vibe — a self-hosting bash/Markdown/JSON workflow harness.

### Added

- **Spec framework** — the `.spec/` four-layer memory model (product, tech,
  design, plan, plus lessons and branch-scoped `features/`), the bundled `spec`
  skill, and `validate.sh` consistency checks.
- **Vibe flow** — a 15-state workflow state machine with the `vibe` skill
  router, per-turn order injection (decisions D8–D12), and graceful degradation
  when optional skills are absent.
- **AGENTS.md provisioning** — instruction template, marker-bounded merge that
  preserves user content, and adapter symlinks (`CLAUDE.md`, `WARP.md`).
- **Claude Code adapter** — a plugin plus three hooks: inject
  (UserPromptSubmit), guard (PreToolUse), and gate (Stop).
- **Monorepo split** — canonical `spec/` and `flow/` halves with compatibility
  symlinks under `.agents/skills/`.
- **Install tooling** — `install.sh` with `--only`, `--dry-run`, and
  `--uninstall`, a `doctor.sh` health check, and a `deps.json` manifest.

[0.1.0]: https://github.com/LennardZuendorf/vibe/releases/tag/v0.1.0
