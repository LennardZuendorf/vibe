---
type: entrypoint
scope: technical
children: []
updated: 2026-03-13
---

# Engineering Agent — Technical Architecture

## Design Philosophy

1. **File-based communication.** All state lives in `.spec/` files. No in-memory state survives session boundaries. Phases read files, not conversation history.
2. **Shell over code.** Prefer bash scripts over complex application code. The framework is glue, not a product.
3. **Hooks enforce, skills orchestrate.** PreToolUse hooks prevent writes in wrong phases. Skills handle the workflow logic.
4. **Subagents are disposable.** Each subagent gets a focused task + minimal context. They return compact summaries, not raw data.

## Architecture Overview

```
.agents/skills/
├── spec/                           # CORE: Spec system (existing, unchanged)
│   ├── SKILL.md                    # Spec navigation and management
│   ├── scripts/
│   │   ├── setup.sh                # Initialize .spec/
│   │   ├── validate.sh             # Validate spec consistency
│   │   └── list-specs.sh           # List current specs
│   └── reference/                  # Templates and writing guides
│
├── develop/                        # CORE: Phase orchestrator (to be updated)
│   ├── SKILL.md                    # Reads .framework.json, routes phases
│   └── scripts/
│       └── phase-gate.sh           # Phase enforcement logic
│
├── setup-framework/                # NEW: Interactive installer
│   ├── SKILL.md                    # Plugin detection + config generation
│   └── scripts/
│       ├── detect-plugins.sh       # Scan for installed plugins
│       └── generate-config.sh      # Write .framework.json
│
└── simplify/                       # BUNDLED: Multi-agent review (existing)
    └── SKILL.md

.claude/
├── settings.json                   # Hook configuration
└── hooks/
    └── check-phase.sh              # PreToolUse phase enforcement

.spec/                              # Per-project (generated)
├── .framework.json                 # Plugin routing config
├── .phase                          # Current phase state
├── product.md                      # Product entrypoint
├── tech.md                         # Tech entrypoint
├── plan.md                         # Implementation plan
├── lessons.md                      # Accumulated mistakes
└── research/                       # Research artifacts
    └── *.md                        # Per-topic research docs
```

## Tech Stack

**Inherited:**
- Claude Code CLI (skills, hooks, subagents, worktrees)
- Bash (scripts, hooks, detection)
- Markdown (all specs and docs)

**Added:**
- JSON (`.framework.json` config)
- No new runtime dependencies

## What We Build vs Inherit

| Source | What |
|--------|------|
| **Claude Code** (inherited) | Skill system, hooks, subagents, worktrees, Agent tool |
| **Spec skill** (existing, ours) | `.spec/` management, templates, validation |
| **Simplify skill** (existing, ours) | Multi-agent code review |
| **Plugin adapters** (new) | Routing logic for superpowers, feature-dev |
| **Setup-framework skill** (new) | Interactive installer, config generation |
| **Updated develop skill** (modified) | Config-aware phase orchestrator |

## Key Patterns

### Plugin Config Schema

```json
{
  "version": 1,
  "phases": {
    "research": {
      "provider": "feature-dev | superpowers | built-in",
      "config": {}
    },
    "discuss": {
      "provider": "superpowers | built-in",
      "config": {}
    },
    "spec": {
      "provider": "spec"
    },
    "plan": {
      "provider": "feature-dev | built-in",
      "config": {
        "competing_proposals": true,
        "wave_grouping": true
      }
    },
    "implement": {
      "provider": "built-in",
      "config": {
        "tdd": false,
        "parallel_waves": true
      }
    },
    "review": {
      "providers": ["simplify", "feature-dev"],
      "config": {
        "confidence_threshold": 80
      }
    }
  },
  "installed_plugins": ["simplify", "feature-dev"]
}
```

### Phase Routing

The `/develop` skill reads `.spec/.framework.json` at startup. For each phase:

1. Read the configured `provider` for that phase
2. Check if the provider's skill is installed (exists in `.agents/skills/`)
3. If installed: delegate to that skill's specific workflow
4. If not installed: fall back to built-in behavior
5. Always enforce phase gates regardless of provider

### Provider Mapping

| Phase | Provider | Delegates To |
|-------|----------|-------------|
| RESEARCH | `feature-dev` | Spawn code-explorer agents per feature-dev patterns |
| RESEARCH | `superpowers` | Use brainstorm skill for requirements discovery |
| RESEARCH | `built-in` | Spawn Explore subagents (current behavior) |
| DISCUSS | `superpowers` | `/superpowers:brainstorm` |
| DISCUSS | `built-in` | AskUserQuestion with structured prompts |
| SPEC | `spec` | Always `/spec` (non-negotiable) |
| PLAN | `feature-dev` | Spawn competing code-architect agents |
| PLAN | `built-in` | Write plan directly (current behavior) |
| IMPLEMENT | `built-in` | Wave-based execution, optional TDD |
| REVIEW | `simplify` | `/simplify` multi-agent review |
| REVIEW | `feature-dev` | Code-reviewer agent with confidence scoring |

### Built-in Defaults

Every phase has a built-in provider that requires zero external plugins:

- **RESEARCH:** Parallel Explore agents (Glob + Grep + WebSearch)
- **DISCUSS:** Structured AskUserQuestion prompts
- **SPEC:** `/spec` skill (always)
- **PLAN:** Direct plan writing with wave grouping
- **IMPLEMENT:** Wave-based subagent execution
- **REVIEW:** Self-review checklist + spec compliance check

### GSD-Inspired Patterns (Built Into Core)

These patterns from GSD are good enough to be built-in defaults, not plugin-dependent:

- **Wave-based task grouping:** Tasks grouped by dependency, waves execute sequentially
- **Plan immutability during implementation:** Plan changes require explicit plan-update step
- **Gap closure:** When implementation reveals plan gaps, stop and update plan first
- **Fresh subagent per task:** Each implementation task gets a clean context

### Superpowers-Inspired Patterns (Built Into Core)

- **Phase gate enforcement:** Hooks prevent file writes in wrong phases
- **Pressure resistance:** Framework resists user pressure to skip phases (polite but firm)

## Plugin Detection

Plugins are detected by checking for their skill directories:

```bash
# detect-plugins.sh
check_plugin() {
  local name=$1
  local path="$HOME/.agents/skills/$name/SKILL.md"
  if [[ -f "$path" ]]; then
    echo "$name:installed"
  else
    echo "$name:not-installed"
  fi
}
```

Supported plugins to detect:
- `superpowers` — `.agents/skills/superpowers/SKILL.md`
- `feature-dev` — `.agents/skills/feature-dev/SKILL.md`
- `simplify` — `.agents/skills/simplify/SKILL.md` (bundled with this framework)

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Plugin skill interfaces change | Pin to known patterns, validate at setup time |
| Config becomes stale (plugin uninstalled) | Validate config at `/develop` startup, warn if provider missing |
| Too many subagents overwhelm context | Each subagent returns compact summary (~95% reduction) |
| Framework overhead discourages use | Built-in defaults make zero-plugin setup instant |

## Branch Documents

| Document | Covers |
|----------|--------|
| **[tech-plugin-system.md](tech-plugin-system.md)** | Plugin adapter interfaces, routing logic, fallback chain |
| **[tech-installer.md](tech-installer.md)** | Detection scripts, config generation, validation |
