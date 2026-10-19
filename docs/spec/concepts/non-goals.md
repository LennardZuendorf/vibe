---
type: concept
updated: 2026-10-19
---

# Non-Goals

- **Not a mandatory bundle.** No tool ever requires another, and nothing requires the `vibe` CLI; the CLI (v0.5) is an optional front door over independently installable tools.
- **Not the OpenSpec layout or CLI.** `spec` adopts OpenSpec's requirement/scenario grammar for interchange; it keeps its own root, document model, and commands.
- **Not a model-enforced gate.** Progress signals, including any future embedded model, are advisory; only the state-machine writer and the guard refuse a transition.
- **Not Claude-only or Codex-only.** Each tool ships a Claude Code plugin and an OpenCode plugin; committed memory reads plain for any agent or none.
- **Not a second context budget.** Injected text stays inside its trigger-class line budget so vibe never crowds out the work it serves.
- **Not a disk scanner.** Cross-repo visibility (v0.5) reads one registry of registered repos, never a filesystem sweep.
- **Not Mojo.** Rejected for no CLI/TUI ecosystem and no native Windows target.
- **Not a general-purpose framework.** The repo is three Rust binaries, Markdown memory, and file contracts — not a platform for arbitrary agent tooling.
