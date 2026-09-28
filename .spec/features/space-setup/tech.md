---
type: feature-tech
feature: space-setup
sibling: product.md
parent: ../../tech.md
updated: 2026-09-28
---

# Feature: space-setup — Architecture

The `space/` half: repo holds templates and adapters, the machine holds materialized
instances. `apply.sh` is the single fan-out engine (materialize → link → register →
record); `install.sh --space` delegates to it so the curl bootstrap keeps working;
`doctor.sh` verifies per runtime. No second sync engine: source fetch and block sync
stay with instruct (C5's rule); space only provisions.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## Files

```
space/AGENTS.md                            # global prompt source (moved from ~/.config/opencode/AGENTS.md)   ~140 LOC
space/mcp.json                            # canonical MCP template; ${ENV} placeholders; serena flagged optional  ~40 LOC
space/plugins.json                        # per-runtime plugin/skill declarations (superpowers, context7, …)  ~30 LOC
space/adapters/opencode/vibe.ts            # hook-trio port, moved from ~/.config/opencode/plugins/vibe.ts   ~90 LOC
space/adapters/opencode/shared-mcp.ts      # shared-MCP plugin, moved; reads the materialized instance        ~45 LOC
space/adapters/opencode/omp-parity/mcp.ts  # MCP format library, moved with it (unchanged consumers)          ~60 LOC
space/adapters/droid/hooks.json.template  # user-scope hook entries (guard/gate/orders) for ~/.factory/hooks.json
space/adapters/droid/project-hooks.json   # project-scope template written per repo by install.sh --local
space/adapters/codex/agents-import.md     # prompt import fix for ~/.codex/AGENTS.md (replaces broken @RTK.md)
space/apply.sh                            # idempotent fan-out: materialize, link, register, record          ~300 LOC
space/doctor.sh                           # per-runtime verification with population assertions               ~120 LOC
space/tests/run.sh                        # behaviour suite (source-only, never ships)
install.sh                                 # gains --space: delegates to space/apply.sh; --local gains the droid hook step
```

## Contract / API

```jsonc
// space/mcp.json — canonical template. Credential fields carry env placeholders;
// "optional" servers materialize disabled unless requested.
{
  "mcpServers": {
    "exa":      { "type": "http", "url": "https://mcp.exa.ai/mcp?exaApiKey=${EXA_API_KEY}" },
    "context7": { "type": "http", "url": "https://mcp.context7.com/mcp",
                  "headers": { "Authorization": "Bearer ${CONTEXT7_API_KEY}" } },
    "serena":   { "type": "stdio", "command": "uvx",
                  "args": ["--from", "git+https://github.com/oraios/serena", "serena",
                           "start-mcp-server", "--project-from-cwd"],
                  "optional": true }
  }
}
```

```jsonc
// ~/.config/vibe/space/manifest.json — written by apply, read by uninstall + doctor.
// Records every path the apply created (files, symlinks, merged-hook entries).
{ "version": 1, "applied": "<iso>", "paths": ["…"], "symlinks": ["…"], "hookEntries": ["…"] }
```

- **Runtime registry** (inside apply/doctor): opencode, claude, codex, droid, omp —
  each detected by a stable marker (binary on PATH or config dir present). Detection
  result feeds doctor's population assertion; an empty registry is a finding.
- **`plugins.json`** maps a logical plugin set to per-runtime install facts:
  `{ name, source, perRuntime: { opencode: "git+…", claude: "name@marketplace", droid: "user-plugin" } }`.

## Implementation Detail

- **Template/instance split.** Every credential-bearing output is materialized from
  the repo template with env fill; the shared instance lands at
  `~/.config/agent-stack/mcp.json` with mode `0600`, keeping `shared-mcp.ts` and
  omp-parity working unchanged. Repo files never hold literal tokens; a scanner in
  `space/tests/run.sh` asserts no token-shaped literals across `space/**` **and
  asserts its own population** (file count floor), per the vacuous-check lesson.
- **Prompt distribution.** Symlink where the runtime resolves symlinks
  (`~/.claude/AGENTS.md`, `~/.config/opencode/AGENTS.md`); import/at-syntax where a
  symlink cannot sit (codex). A differing regular file at the target is backed up
  beside the new indirection with one warning — never silently destroyed.
- **opencode.** Plugin files move into `space/adapters/opencode/`; apply symlinks
  each into `~/.config/opencode/plugins/`. Apply never writes `opencode.json`:
  MCP reach flows through the shared-MCP plugin, so omp-parity's keys (providers,
  model roles, its plugin list) stay byte-untouched. If opencode proves not to
  auto-load symlinked plugin files, fall back to copy + a doctor drift check.
- **droid.** User scope: merge guard/gate/orders entries into `~/.factory/hooks.json`
  using the same marker-pairing discipline as `merge-agents.sh` (validate pairing
  before rewriting; refuse, never mangle). Project scope: `install.sh --local` writes
  `.factory/hooks.json` per repo as a sibling of the existing `.claude/settings.json`
  step; hook commands call the same `.agents/skills/vibe` scripts via
  `"$FACTORY_PROJECT_DIR"` absolute paths. Droid's `PreToolUse` permissionDecision
  (deny) carries the block; droid's UserPromptSubmit additionalContext carries orders.
  Hooks stay thin shells over the shared resolvers — no policy duplication.
- **codex.** Prompt: `~/.codex/AGENTS.md` gains an import of the shared prompt
  (replacing the broken `@RTK.md`-only import; RTK import itself is repaired). MCP:
  TOML emission is additive per `[mcp_servers.<name>]` block from the canonical
  template; foreign blocks are never rewritten. Codex TOML has no env expansion, so
  tokens are baked at apply time; token rotation means re-run apply (documented).
- **claude.** Existing `.claude` wiring is untouched; apply repoints the
  `~/.claude/AGENTS.md` symlink at the repo source, adds user-scope MCP servers
  idempotently, and only verifies already-installed plugins (superpowers et al).
- **Idempotency and uninstall.** Apply re-run with unchanged inputs rewrites
  byte-identical outputs and produces no backups or warnings. Uninstall reads the
  manifest and removes exactly the recorded paths — the surgical inverse tested with
  a discriminating test (user file planted in each shared target must survive while
  the shipped entry is removed).
- **Bootstrap.** `install.sh --space` passes the space flag through the existing
  network bootstrap; a stranger eval runs the whole path from a bare `mktemp -d`
  home, exercising the install-target reality the in-repo suite cannot.

<!-- merge -->
## The space/instruct seam

Space provisions; instruct authors. Space materializes config (MCP, plugins, hook
registration, prompt indirection, workspaces) and never writes instruction content;
instruct authors and syncs instruction content (blocks, tiers, sources) and never
writes config outside its injection surfaces. The global prompt's authoring home
moves to instruct's global tier when C5 lands; space's distribution step then
consumes what instruct syncs. Source sync is instruct's alone — space has no fetch
loop ("no second sync engine").
<!-- /merge -->
