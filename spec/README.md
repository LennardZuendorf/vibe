# The spec framework

> **For humans.** This README is the standalone guide to the spec half of vibe kit.
> Agents read [SKILL.md](SKILL.md) instead — it is the router they route through.

The **spec framework** is a durable planning layer for a codebase. It teaches an
agent (or a human) to keep design docs that are *current*, not a backlog: what you
are building (product), how it is built (tech), the visual / interaction language
(design), and the order of work (plan). It ships as the `spec` skill, the `using-spec` session router, two agents and
hooks. The central `vibe spec` command and compatibility `vibe-spec` share one
Rust application layer for queries, authoring and validation. The current branch
also implements registered storage and local stdio MCP; shipping remains gated.
Installed skills and hooks still need the compatibility binary they invoke.

It is one of two halves. The other is [the vibe flow](https://github.com/LennardZuendorf/vibe/blob/main/flow/README.md); the
[root README](https://github.com/LennardZuendorf/vibe#readme) explains the split and
holds the [Install section](https://github.com/LennardZuendorf/vibe#install). This half stands entirely alone.

## Quickstart

```bash
/spec setup            # write the config and seed a spec root (vibe-spec init)
/spec strategy         # author the root layer: product / tech / design / plan
/spec feature <name>   # route and scope one named feature
/spec validate         # check structural consistency
```

`/spec …` invokes the skill on hosts that expose it as a command. Direct binary
use is available through `vibe spec` or the compatibility `vibe-spec`. A central
source build needs no peer executable or Node for spec operations. See the root
Install section for the separate installed-skill and hook requirements.

```bash
vibe spec root                # show the selected root
vibe spec init                # seed a layered root, preserving existing files
vibe spec validate            # check consistency
vibe spec validate --strict   # promote warnings to errors
vibe spec check-drift         # compare current specs and feature lifecycle
```

For management commands (`project`, `context`, `branch`, `document`, `snapshot`
and `reconcile`), put any supported `--project`, `--checkout` or `--branch` selector
after the command or subcommand. Prefix context selectors refuse before effects.
Existing spec operations and MCP launch use selectors before their command.
Prefix `--root` retains its legacy override behavior where supported. Hooks
reject the new context selectors and keep their legacy no-selector/`--root` behavior.

### Choose storage and context

```bash
# From the intended checkout, choose one mode:
vibe spec project init --mode repo-first
# Or, without existing repo specs:
vibe spec project init --mode global-first
vibe spec project list
vibe spec project show --project "$PROJECT"
vibe spec project attach --project "$PROJECT" --checkout "$OTHER_CHECKOUT"
vibe spec context --project "$PROJECT"
```

Set `PROJECT` to the returned opaque project ID; `OTHER_CHECKOUT` names a matching
clone/worktree. Init registers the initial main checkout; attach `--main` replaces
that choice. Repository identity recognizes matching clones/worktrees. Ambiguous
matches need explicit project selection. Setup chooses one mode and does not
migrate existing repository specs to global storage.

Repo-first stores durable specs and feature designs/plans in the code repository.
Reads/writes follow the selected checkout and its checked-out branch. Inside a
matching checkout, omitted checkout/branch selectors use that checkout. Explicit project selection ignores an unrelated ambient
checkout; an explicit mismatched checkout refuses. Outside the selected project,
project-only access uses its registered main checkout's
actual branch, not a hardcoded `main`. Selecting context never switches or merges
code branches.

Global-first stores the entire durable spec and feature designs/plans outside the
code repository. Matching clones/worktrees share one stored branch; other branches
stay isolated. Initialize a stored branch from an explicit existing spec base:

```bash
vibe spec branch init "$FEATURE_BRANCH" --from "$BASE_BRANCH" --project "$PROJECT"
vibe spec context --project "$PROJECT" --branch "$FEATURE_BRANCH"
```

`BASE_BRANCH` is the actual existing stored code-ref name. Branch init copies the
complete stored spec/artifact tree; it does not create or check out a code branch.
It resolves the exact stored base and target. Its returned code context uses only
the validated caller checkout when already on that target; otherwise checkout and
code_head are null and code_available is false. It chooses no other clone or main
checkout. Ordinary context selection still rejects ambiguous matching checkouts
and explicit checkout/branch mismatches. An explicit global stored branch can be
selected outside code; resolve checkout ambiguity explicitly when present.
Operations needing matching code files fail when code is unavailable.
`code_available` means matching files exist; it does not promise Git history.

Context reports separate code, spec and artifact anchors plus an opaque
`revision_token`. Use the returned token for an expected revision; do not use a
Git OID or a document hash. Re-read after a context change or stale-token refusal.
Writes never silently refresh and substitute newer authority.

`project list` returns the registry revision; `project show` also returns the
project metadata revision. Project init/attach accept `--expect` for the registry;
branch init also accepts `--expect-project`. Omitting these CLI flags captures
current metadata once; MCP requires the explicit metadata tokens. These tokens
are separate from document/context mutation tokens.

Project setup does not install a workflow harness. The existing
`setup --git-hooks|--opencode|--agents-md` command installs marked hook/router
files. `init` seeds a layered tree and preserves existing files. Neither command
is a storage-mode migration.

### Read and edit logical documents

```bash
vibe spec document list --project "$PROJECT" --branch "$FEATURE_BRANCH"
vibe spec document read spec/product.md --project "$PROJECT" --branch "$FEATURE_BRANCH"
vibe spec document write spec/product.md --input "$MARKDOWN_FILE" --expect "$REVISION" --project "$PROJECT" --branch "$FEATURE_BRANCH"
vibe spec document write spec/product.md --expect "$REVISION" --project "$PROJECT" --branch "$FEATURE_BRANCH" < "$MARKDOWN_FILE"
vibe spec document remove spec/concepts/obsolete.md --expect "$REVISION" --project "$PROJECT" --branch "$FEATURE_BRANCH"
```

`REVISION` is the current selected context's `revision_token`; obtain a fresh one
before each independent mutation. Use logical Markdown paths under `spec/` or
`superpowers/` (`superpowers/specs/…` and `superpowers/plans/…` for features).
Write reads replacement UTF-8 Markdown from `--input FILE`, or stdin when omitted.
The two write lines are alternatives; do not reuse the old token after a write.
The remove example assumes that document exists and its removal is intended.

Global authoring records safe drafts and reports semantic diagnostics. A recorded
write is not whole-tree validation. Run `vibe spec --project "$PROJECT" --branch
"$FEATURE_BRANCH" validate` after completing related edits. Raw edits to a global tree
need an explicit snapshot before managed authoring can proceed:

```bash
vibe spec snapshot --expect "$REVISION" --project "$PROJECT" --branch "$FEATURE_BRANCH"
```

### Reconcile global branches

Save an exact preview, inspect it, and apply that saved plan:

```bash
vibe spec reconcile --from "$FEATURE_BRANCH" --into "$BASE_BRANCH" --dry-run --project "$PROJECT" > "$PLAN_FILE"
vibe spec reconcile --input "$PLAN_FILE" --expect "$TARGET_REVISION"
```

Set `TARGET_REVISION` to that plan's `target_revision`. Saved-plan application
accepts no selection/source/target overrides; it refuses changed captured inputs.
Preview does not refresh, recover or write. Explicit reconcile operates on the
selected target/record; it does not run a broader refresh.

For a conflict, the canonical target stays unchanged. Use the returned record ID:

```bash
vibe spec reconcile status "$RECORD" --project "$PROJECT"
vibe spec reconcile read "$RECORD" spec/product.md --project "$PROJECT"
vibe spec reconcile resolve "$RECORD" --input "$RESOLUTION_FILE" --expect "$TARGET_REVISION" --expect-resolution "$WORKSPACE_REVISION" --project "$PROJECT"
vibe spec reconcile status "$RECORD" --project "$PROJECT"
```

Read returns base, source, target and proposed text (null for absent files), plus
the current target and resolution-workspace tokens. Read every manifest conflict
and submit exactly one entry per conflict in a structured bundle:

```json
{"files":[{"path":"spec/product.md","content":"replacement Markdown\n"},{"path":"spec/concepts/obsolete.md","content":null}]}
```

This bundle illustrates text and deletion; use the actual record's full conflict
set. Resolve requires both returned tokens and validates the completed tree
before publication. Validation errors prevent finalization; warnings remain
visible. Records/conflict workspaces persist across client restarts: reopen with
status/read and current tokens. Do not edit unrelated workspace files.

Only explicit global snapshots capture code/spec bindings for automatic delayed
reconciliation. Ordinary writes and branch setup do not. Snapshot each source
and target while its matching code checkout/head is available; a clean snapshot
can capture metadata without a new content commit. Establish target observation
with a context/read before the code merge; first observation only baselines.

Later context/document/query reads can reconcile a newly observed direct
two-parent code merge only with unique, clean, unchanged snapshot evidence for
both sides and the exact observed parent tips. Squash/rebase, intervening commits,
missing/deleted refs and ambiguous or edited evidence require explicit reconcile.
No daemon or network fetch runs, and code merges never wait. Repo-first relies on
the code repository's normal Git merge; it does not use global reconciliation.

### Local MCP

```bash
vibe spec --project "$PROJECT" mcp --stdio
# Or pin a stored branch for this server's defaults:
vibe spec --project "$PROJECT" --branch "$FEATURE_BRANCH" mcp --stdio
```

Selectors precede `mcp`. Launch selectors are immutable defaults; individual calls
can select context without changing other calls' defaults. The 28 tools expose typed
project, context, document, lifecycle, validation and reconciliation operations.
Use discovery for the required token classes and full input/output schemas.
Only protocol messages use stdout; diagnostics use stderr. This launch does not
configure a harness, start a remote server or synchronize another machine.

The server bounds admitted application operations at 32, including project
waiters and cancelled workers until completion. Each UTF-8 document is bounded at
8 MiB; each encoded frame/envelope is bounded at 8 MiB including its newline.
Oversized output is refused without truncation. Malformed or oversized frames
may be dropped; if an error cannot fit its original ID, the transport refuses that write. Cancellation after publication preserves the durable result and
finishes bookkeeping; the SDK may suppress its reply. Inspect current context,
document or record state after reopening. No historical request replay is promised.

### Practical limits

Storage and MCP are local-only. Storage-mode migration is unsupported. Explicit
reconcile handles uncertain code history; it does not block or reverse a code
merge. Locks are never stolen by age: after a crash, independently verify the
holder stopped before manually removing its abandoned lock, then recover.
An interrupted merge workspace without durable manifest proof is preserved as
`needs-input` / `unrecorded_merge_workspace` for inspection; automatic reclamation
is not promised.

## The two-layer model

The logical durable layer has two layouts. `vibe spec root` prints the selected
physical root; repo-first defaults to `docs/spec/`, while global-first stores it
outside code. `init` creates the layered layout shown below. Logical feature
artifacts use the selected artifact root in either mode.

```
<root>/
├── product.md, tech.md, design.md, plan.md   ← hubs: short, link only
├── lessons.md                                 ← tagged lessons, append-only
├── areas/ flows/ concepts/ platform/ tech/    ← typed notes the hubs link to
│   decisions/ questions/ parts/<part>/
└── features/<feature>/   fallback feature layer only; see below
```

- **Root** answers project-level questions and carries no backlog or archaeology.
  The layered guide is `reference/layered.md`.
- **Feature layer**, branch-scoped: by default the superpowers artifacts under
  `docs/superpowers/specs/` and `docs/superpowers/plans/` in repo-first. Global-first
  keeps the same `specs/` and `plans/` contents in its external artifact root. The fallback folder
  `<root>/features/<feature>/` applies only when superpowers is absent, your human
  partner says *self*, `.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec
  interchange is required. Wrap-up promotes the cross-cutting parts to the root and
  deletes the feature artifacts **before the branch merges**. Code is truth.
- **Flat (legacy)** roots keep `product.md`, `tech.md`, `design.md`, `plan.md`,
  `lessons.md` and `{product,tech}-{topic}.md` topic docs side by side. The
  templates in [reference/templates/](reference/templates/) are flat-layout only;
  the `scripts/` validator reads a flat legacy root only.

## Day-to-day authoring

**Bootstrap (strategy).** Write the root layer in order:
`product.md → tech.md → design.md → plan.md`. The root plan holds the feature map
and a Feature Sequence with binary whole-feature gates — never unit-level detail.
The row for a fallback feature exists, with status `IN PROGRESS`, before its design.

**A new feature** routes through [SKILL.md](SKILL.md) § Routing. The default is
`superpowers:brainstorming` then `superpowers:writing-plans`. The fallback follows
[feature.md](feature.md): step 0 sizes the work, then Locate and name, Interview for
WHAT, Rigor gate, Sketch HOW, Plan units. Cite unit IDs in commits and tests during
implementation. Bump each edited spec's `updated:` date, then run `/spec validate`.

**Wrap-up** has one sequence: [SKILL.md](SKILL.md) § Wrap-up.

**Strict separation** is the core discipline: product holds *what & why* (no code
or paths), tech holds *how* (no UX opinions), plan holds *units & order* (no code
or requirement prose), design holds tokens and interaction patterns. Read before
you write; keep parent ↔ child links alive both ways.

**Superpowers are optional.** Each authoring step names a delegate the skill
suggests unless `.vibe/spec.json` sets `suggest` to false. Decline any and the skill
does the step itself from its constraint documents. They enhance; they never gate.

## With or without the flow

The spec skill is self-sufficient. Drive it directly with `/spec …` on any host,
or let [the vibe flow](https://github.com/LennardZuendorf/vibe/blob/main/flow/README.md) drive its authoring states
(`strategy.spec`, `feature.design`, `feature.plan`, `feature.compound`). Either way
the docs, templates, and validator are identical.

## Degrade behavior

- **Installed skill/hook has no `vibe-spec`:** it reports the missing compatibility
  binary and points to Install. The central source-built `vibe spec` still works
  independently. Flat legacy fallback scripts cannot validate a layered root;
  unavailable validation never counts as a pass.
- **Other `AGENTS.md` readers:** agents follow the written authoring flow manually.
- **Missing superpowers:** the fallback feature layer applies.

## File map

Everything below is the `spec` skill, addressed at runtime under
`.agents/skills/spec/` (and `.agents/skills/using-spec/`).

| Path | What it is |
|---|---|
| [SKILL.md](SKILL.md) | Skill router: routing table, layers, wrap-up, roles |
| `using-spec` | Session-start router: source `spec/using-spec/SKILL.md`; packaged/runtime sibling `../using-spec/SKILL.md` |
| [strategy.md](strategy.md) | Root-layer authoring guide |
| [feature.md](feature.md) | Fallback feature-layer authoring flow |
| [reference/](reference/) | Per-doc writing guides (`product`, `tech`, `design`, `plan`) and [templates/](reference/templates/) |
| [agents/](agents/) | Claude Code agents: `spec-promoter`, `spec-health` |
| [hooks/](hooks/) | Fail-open shim that runs `vibe-spec hook <event>` |
| `scripts/` | Bash scripts, legacy (flat root only) |

## More

- [the root README](https://github.com/LennardZuendorf/vibe#readme) — the umbrella: the spec/flow split and install.
- [`flow/README.md`](https://github.com/LennardZuendorf/vibe/blob/main/flow/README.md) — the other half: the state-machine flow.
- [SKILL.md](SKILL.md) — the canonical rules agents follow.
