---
type: concept
updated: 2026-10-19
---

# Shared feature sequence

Rows of the Feature Sequence that span all three tools; delivery order and binary gates are in [the feature sequence](feature-sequence.md).

| Order | Feature | Tool | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|---|
| F3 | oracles-parity | all | Frozen `oracles/js`+`oracles/bash` + `MANIFEST.sha256`; generic `vibe-parity` differential harness; spec corpus classes with floors (flow classes land with F5) | Oracle byte change reddens CI; planted mutant confirmed via `git diff --numstat` also reddens it | DONE | — |
| F4 | workspace-core | all | Cargo workspace with `vibe-core`, `vibe-spec`, `vibe-parity` only (flow/instruct/signals crates arrive with their rows); MSRV 1.88; `cargo-deny` allowlist | Green matrix (3 OS × stable+MSRV); `cargo metadata` graph assertion | DONE | F2 |
| F21 | packaging | all | Three plugin dirs (manifests, skills, commands, agents, hooks, bin, opencode); `marketplace.json`; `doctor` peer checks — spec slice in the spec program | `claude plugin validate --strict` × 3; shim exits 0 without binary | NOT STARTED | F8, F14, F19 |
| F22 | release-install | all | `cargo-dist` 5 targets, per-tool tags, `SHA256SUMS`; `install.sh` (subsets, platforms, uninstall, cargo fallback) — spec slice in the spec program | 5 verified artifacts; containerised subset × platform matrix; uninstall restores | NOT STARTED | F21 |
| F23 | retire-legacy | all | Delete `flow/engine/`, node tests, `build-plugin.sh`, old `install.sh` paths; `vibe-flow migrate`; spec slice deletes `spec/scripts/*.sh` | Full parity green; no `node` anywhere in hooks | NOT STARTED | F15, F20, F22 |
| F24 | doc-truth | all | READMEs (umbrella + per tool), CHANGELOG, root specs truth sweep; drift check extended to three tools | `check-drift` green; stranger eval from a fresh non-git target | NOT STARTED | F23 |
