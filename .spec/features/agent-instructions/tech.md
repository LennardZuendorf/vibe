---
type: feature-tech
feature: agent-instructions
sibling: product.md
parent: ../../tech.md
updated: 2026-06-06
---

# Feature: Agent Instructions — Architecture

Template-driven provisioning of the canonical `AGENTS.md` engineering guide and
optional platform adapter symlinks during `vibe-setup` `setup.apply`. Mirrors the
`spec` skill's `reference/templates/` + `setup.sh` pattern.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)
**Related:** [../platform-adapters/tech.md](../platform-adapters/tech.md),
[../vibe-flow/tech.md](../vibe-flow/tech.md)

---

## Files

```text
.agents/skills/vibe/
├── SKILL.md                              # detect/apply steps updated
└── reference/
    ├── templates/
    │   └── AGENTS.md                     # canonical template (source of truth for init)
    └── adapters.json                     # manifest: filename → {runtime, default, notes}

# Target repo (after setup.apply)
AGENTS.md                                 # real file — merged from template
CLAUDE.md                                 # optional symlink → AGENTS.md
WARP.md                                   # optional symlink → AGENTS.md
```

`adapters.json` is data, not prose — keeps the adapter catalogue extensible
without editing skill body text.

---

## Managed blocks in `AGENTS.md`

Two separate managed regions; do not conflate them:

| Markers | Owner | Writable by |
|---|---|---|
| `<!-- vibe:instructions:start/end -->` | `vibe-setup` template merge | `setup.apply`, repair |
| `<!-- vibe:active-rules:start/end -->` | `regen-active-rules.sh` | `*.compound` only |

Everything **outside** both marker pairs is user-owned. `detect-context.sh`
already warns on hand-edits inside the active-rules block.

### Template body

The template at `reference/templates/AGENTS.md` is the repo's current engineering
guide (Prime Directive, flow routing, commands, boundaries, commits, spec layout)
wrapped in `vibe:instructions` markers. The `active-rules` block is appended
below instructions markers with an empty digest placeholder — same shape as the
live repo-root `AGENTS.md`.

On merge, `setup.apply`:
1. Reads template instructions section.
2. If target has `vibe:instructions` markers → replace content between them.
3. If target has only `vibe:constitution` markers → migrate to `vibe:instructions`.
4. If target lacks markers but body matches template (hash or normalized compare) →
   **wrap** existing content in markers (do not append duplicate block). Required for
   dogfood repo where Stage 1 delivered an unmarked engineering guide.
5. If target lacks markers and content diverges → append managed block (warn) or ask user.
6. Never touches content outside markers or inside active-rules markers.
7. Runs `regen-active-rules.sh` after merge **only when** symlink dedupe is landed, or
   before creating adapter symlinks — see [plan.md](plan.md) AI4.

---

## Merge algorithm (`setup.apply`)

```bash
# Pseudocode — implement as merge-agents.sh or inline in setup flow
TEMPLATE=".agents/skills/vibe/reference/templates/AGENTS.md"
TARGET="AGENTS.md"

if [[ ! -f "$TARGET" ]]; then
  cp "$TEMPLATE" "$TARGET"
else
  extract_between "$TEMPLATE" "vibe:instructions" → NEW_BODY
  if markers_present "$TARGET" "vibe:instructions"; then
    replace_between "$TARGET" "vibe:instructions" "$NEW_BODY"
  else
    append_managed_block "$TARGET" "$NEW_BODY"   # warn
  fi
fi
# constitution-block migration: if old vibe:constitution markers found,
# replace with vibe:instructions in same pass (one-time upgrade path)
```

Deterministic, idempotent: re-running on an already-merged file with an
up-to-date template is a no-op (byte compare inside markers).

---

## Adapter symlink provisioning

Manifest-driven from `reference/adapters.json`:

```json
{
  "canonical": "AGENTS.md",
  "adapters": [
    { "file": "CLAUDE.md", "target": "AGENTS.md", "runtime": "claude-code", "default": false },
    { "file": "WARP.md",   "target": "AGENTS.md", "runtime": "warp",        "default": false }
  ]
}
```

For each adapter the user selects:

```bash
link_adapter() {
  local adapter="$1" target="$2"
  if [[ -L "$adapter" ]] && [[ "$(readlink "$adapter")" == "$target" ]]; then
    echo "skip: $adapter already symlinks $target"; return 0
  fi
  if [[ -e "$adapter" ]] && [[ ! -L "$adapter" ]]; then
    echo "block: $adapter is a real file — show diff, ask user"; return 1
  fi
  ln -sf "$target" "$adapter"
}
```

Use **relative** symlinks (`ln -sf AGENTS.md CLAUDE.md`) so clones and moves
survive. Never create `WARP.md` unless requested — Warp already reads
`AGENTS.md`.

### Interaction with `regen-active-rules.sh`

That script currently lists both `CLAUDE.md` and `AGENTS.md` as write targets.
With symlinks, `mv -f` replaces the link with a real file. **Prerequisite**
(documented in [platform-adapters/plan.md](../platform-adapters/plan.md)):
dedupe targets by resolved path before writing. Until that lands, compound runs
after symlink creation will break the link.

---

## `setup.detect` audit surface

Report a row per item:

| Check | States |
|---|---|
| `AGENTS.md` | missing / present-no-markers / present-managed-ok / present-managed-stale |
| `vibe:instructions` block | absent / current / stale (differs from template) |
| `vibe:constitution` block (legacy) | absent / present-needs-migration |
| `vibe:active-rules` block | absent / empty / populated |
| Each adapter in manifest | absent / symlink-ok / symlink-wrong-target / real-file / broken-link |

---

## Deprecations

| Old | New |
|---|---|
| Constitution block merge into `CLAUDE.md` + `AGENTS.md` separately | Single `AGENTS.md` template merge; adapters are symlinks |
| Hand-maintained duplicate adapter prose | `CLAUDE.md` → symlink; content lives once in `AGENTS.md` |
| `vibe:constitution` markers | `vibe:instructions` markers (with one-time migration) |

Update `.agents/skills/vibe/SKILL.md` apply step 1 when this feature ships.
Step 4 (`regen-active-rules`) stays; step 1 replaces the constitution template block.

---

## Open Questions

1. **Script home** — `merge-agents.sh` under `.agents/skills/vibe/scripts/` (colocated with template).
   Resolved: colocated under `vibe/scripts/`; `install.sh` calls it.
2. **Template versioning** — embed `<!-- vibe:template-version: 1 -->` inside
   instructions markers so `setup.detect` can report stale without a full diff?
