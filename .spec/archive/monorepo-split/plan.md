---
type: feature-plan
feature: monorepo-split
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-03
---

# Feature: Monorepo Split — Implementation Plan

Move first, link second, sweep third, compound last. The move must be green
(all three suites) before the sweep touches any prose, so path breakage is
never masked by doc churn.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** Starts immediately (no upstream).

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | Canonical split | monorepo-split/1 |
| R2 | Runtime paths unchanged | monorepo-split/1, monorepo-split/2 |
| R3 | Installer materializes | monorepo-split/3 |
| R4 | Truth sweep | monorepo-split/4 |
| R5 | Orphans compounded | monorepo-split/5 |
| R6 | Metadata correct | monorepo-split/4 |

---

## Key Technical Decisions

1. **Symlinks point root-ward** — `.agents/skills/*` is the portable runtime interface; `spec/`+`flow/` are storage. See tech.md.
2. **Script audit before move lands** — physical-path self-location is the one real breakage vector; test both invocation paths.
3. **Sweep after green move** — doc edits never mixed with path-breaking commits.

---

### monorepo-split/1 — Move + symlinks

**Goal:** `spec/` + `flow/` real dirs, `.agents/skills/*` + `.claude/skills/spec` symlinks, gitignore cursor path, live cursor migrated.

**Requirements:** R1, R2

**Dependencies:** —

**Files:**

```
spec/  flow/  .agents/skills/{spec,vibe}  .claude/skills/spec  .gitignore
```

**Test scenarios:**

- `bash tests/spec/run.sh` + `tests/flow/run.sh` green post-move
- `orders.sh`, `set-state.sh`, `detect-context.sh` produce identical output invoked via `.agents/skills/vibe/scripts/` and `flow/scripts/`

**Verification:** all three suites + `git ls-files -s .agents/skills` shows mode 120000 links.

---

### monorepo-split/2 — Script self-location audit

**Goal:** every script resolves repo root correctly from both real and symlinked invocation; add regression assertions.

**Requirements:** R2

**Dependencies:** monorepo-split/1

**Files:**

```
flow/scripts/*.sh  spec/scripts/*.sh  .claude/hooks/*.sh  tests/flow/run.sh
```

**Test scenarios:**

- Each script invoked via both paths → byte-identical output
- Hook scripts resolve `${CLAUDE_PLUGIN_ROOT}`-relative paths unchanged

**Verification:** new assertions in `tests/flow/run.sh`, suite green.

---

### monorepo-split/3 — Installer dereference

**Goal:** `install.sh` copies from `spec/`+`flow/`, materializes real dirs in target `.agents/skills/`, cursor preservation intact.

**Requirements:** R3

**Dependencies:** monorepo-split/1

**Files:**

```
install.sh  tests/adapters/run.sh
```

**Test scenarios:**

- Fresh install: target `.agents/skills/vibe` is real dir, zero symlinks in copied tree
- Re-install with live `feature.impl` cursor → cursor survives (existing regression stays green)

**Verification:** `bash tests/adapters/run.sh` green incl. new dereference assertions.

---

### monorepo-split/4 — Truth sweep

**Goal:** zero stale `.agents/flow` refs in `.spec/**` + template; root plan counts/sequence current; AGENTS.md regenerated (dup line fixed); plugin.json homepage fixed.

**Requirements:** R4, R6

**Dependencies:** monorepo-split/1

**Files:**

```
.spec/{product,tech,plan}.md  .spec/features/{platform-adapters,agent-instructions}/*  spec/reference/plan.md  flow/reference/templates/AGENTS.md  AGENTS.md  .claude-plugin/plugin.json
```

**Test scenarios:**

- `grep -r "\.agents/flow" .spec/ AGENTS.md flow/ spec/` → only the intentional test assertion
- `merge-agents.sh` regen leaves user content untouched (existing tests)

**Verification:** grep evidence + `validate.sh` + `tests/adapters/run.sh` green.

---

### monorepo-split/5 — Compound orphan features

**Goal:** `vibe-flow-collapse`, `vibe-skill-consolidation`, `spec-skill-improvements` compounded: promote durable bits, DONE rows + delivered notes in root plan, archive, prompt-delete.

**Requirements:** R5

**Dependencies:** monorepo-split/4

**Files:**

```
.spec/plan.md  .spec/tech.md  .spec/features/<three>/  .spec/archive/
```

**Test scenarios:**

- `validate.sh` clean after archive
- Root plan "Active focus" and drift table truthful

**Verification:** `validate.sh` output + folder absence + delivered notes present.

---

## Progress

| Unit | Status |
|---|---|
| monorepo-split/1 | DONE (606dd5e) |
| monorepo-split/2 | DONE (c44dc23) |
| monorepo-split/3 | DONE (8758720) |
| monorepo-split/4 | DONE (9f5e671) |
| monorepo-split/5 | DONE |
