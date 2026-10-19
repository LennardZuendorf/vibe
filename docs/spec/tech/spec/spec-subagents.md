---
type: tech
applies:
  - "spec/agents/**"
  - ".claude/agents/**"
  - ".opencode/agents/**"
updated: 2026-10-19
---

# Subagents

Real platform agents on Claude Code and OpenCode. The sources live in
`spec/agents/`. `build-plugin.sh` maintains the symlink `plugin/agents ->
../spec/agents`. This repo registers them as `.claude/agents/<name>.md`
symlinks into `spec/agents/`. OpenCode files come from `vibe-spec setup
--opencode`.

| Agent | Model | Tools | Writes |
|---|---|---|---|
| spec-promoter | sonnet | Read, Write, Edit, Bash | only under `vibe-spec root`; runs `vibe-spec promote <feature> --dry-run`, then the real `promote` and `--done` only after an explicit yes |
| spec-health | haiku | Read, Bash | nothing: `Read-only: never write files.` |

`spec-interviewer` and `spec-tracer` are retired (D36): feature
interviews and plans run through `superpowers:brainstorming` and
`superpowers:writing-plans`.

Each agent file is at most 40 lines: frontmatter `name/description/tools/model`,
a fallback rule (`vibe-spec` missing: reply "vibe-spec binary missing" and
stop), and for the promoter the line `Write only under the directory printed
by \`vibe-spec root\`.`
