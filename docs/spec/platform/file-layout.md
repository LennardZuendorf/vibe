---
type: platform
updated: 2026-10-19
---

# File Layout

```text
vibe/
├── crates/{vibe-core,vibe,vibe-spec,vibe-flow,vibe-instruct,vibe-signals,vibe-parity}/
├── spec/        # plugin: .claude-plugin/ skills/spec/ commands/ agents/ hooks/ bin/ opencode/ reference/
├── flow/        # plugin: … + state-machine.json policy.json bin/fallback/
├── instruct/    # plugin: … + blocks/ (shipped defaults)
├── vibe/        # plugin (v0.5): front-door command surface only — no hooks
├── contracts/   # schemas + goldens + run.sh
├── oracles/{js,bash}/ + MANIFEST.sha256
├── tests/parity/ (corpus, GREEN.hard)   tests/run.sh
├── .claude-plugin/marketplace.json
├── install.sh
├── .vibe/       # this repo's own config + blocks (+ run/, local/ ignored)
├── docs/spec/   # this repo's memory
└── .agents/skills/{spec,flow,instruct} → ../../<tool>/skills/<tool>  (compat)
```

A target repo receives config (`.vibe/spec.json`, `.vibe/flow.json`,
`.vibe/instruct.json`), repo-shared blocks (`.vibe/blocks/*.md`), provider
manifests (`.vibe/providers/*.json`), the spec tree (`docs/spec/`), and each
tool's AGENTS.md managed block — all committed. Runtime (`.vibe/run/`) and
the personal tier (`.vibe/local/`) are never committed; a fresh clone
regenerates both.
