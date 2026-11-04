---
type: concept
updated: 2026-11-04
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

> **Current source:** Rust `vibe-spec`, flow F5/F6 and the C0 `vibe` front door are implemented. The Node engine still provides flow injection. `install.sh` exposes `spec`, `using-spec` and `vibe` as sibling skills. Flow prefers `.vibe/run/flow/state.json` once present and keeps the legacy cursor as an absence-only fallback. The full three-tool carrier model and later CLI modules remain targets; delivery status is in [plan.md](../plan.md).

Each of the three tools also ships as its own Claude Code plugin and its own
OpenCode plugin — see Carrier model above.
