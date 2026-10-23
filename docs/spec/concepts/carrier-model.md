---
type: concept
updated: 2026-10-19
---

# Carrier model and front door

**Carrier model (v0.4 target).** Each tool — `spec`, `flow`, `instruct` — is a
static Rust binary shipping as its own Claude Code plugin and its own OpenCode
plugin, installable and useful alone. `contracts/` holds the versioned schemas
the three tools exchange as files under `.vibe/`; no tool imports another. One
shared `install.sh` and one marketplace list all three; each keeps its own
release train and version.

**Front door (v0.5 target).** One `vibe` CLI ships all three tools together and
adds the cross-tool capabilities no single tool can own — worktree lifecycle
(`vibe worktree`), global spec management (`vibe space`), and global
instruction sharing — while each tool stays independently installable; the CLI
is the convenience, never the requirement.

> **Shipped today:** vibe is still one repo running a Node engine (`flow/engine/`), with `install.sh` copying `.agents/skills/{spec,vibe}` and `.claude/**` into a target repo. The cursor lives at `.agents/skills/vibe/state.json`; specs live in `docs/spec/`. Everything three-binary and Rust above is the v0.4 target — build order and gates are in [plan.md](../plan.md).

Each of the three tools also ships as its own Claude Code plugin and its own
OpenCode plugin — see Carrier model above.
