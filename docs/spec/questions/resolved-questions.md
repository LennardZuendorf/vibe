---
type: question
updated: 2026-10-19
---

# Resolved Questions

1. **Cursor git tracking.** Version the static state machine; gitignore the mutable cursor (installer seeds + ignores it).
2. **Skill count (v0.3).** Consolidated seven `vibe-*` shims into one `vibe` skill (router + phase files).
3. **Adapter installation.** `install.sh` copies the core + Claude adapter, merges `AGENTS.md` via markers, supports `--only`, `--dry-run`, `--uninstall`.
4. **Hook strictness.** Shipped warn-first; only the guard's hard blocks deny, every `Stop` predicate is warn-only.
5. **Skill count (v0.4).** Supersedes #2 — three skills, one per tool (`spec`, `flow`, `instruct`), each shipped by its own plugin.
6. **Engine language.** Rust, static binaries, zero runtime deps — supersedes the Node engine.
7. **Spec root.** Default `docs/spec/`, `.spec/` legacy still resolves; configurable via `.vibe/spec.json` or `$VIBE_SPEC_ROOT`. <!-- literal -->
8. **OpenSpec.** Adopt the Requirement/Scenario grammar for interchange; the strategic layout is vibe's layered vault (D44), the feature layer is superpowers-first (D45).
9. **Umbrella (v0.5).** The `vibe` CLI becomes the front door that ships all three tools plus worktree and global-spec capabilities. D18's core survives — the three tools stay independent and alone-installable — but "no umbrella ever" retires at v0.5.
