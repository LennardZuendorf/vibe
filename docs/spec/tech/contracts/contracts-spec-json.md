---
type: tech
applies:
  - "contracts/spec-json.schema.json"
  - "crates/vibe-spec/**"
updated: 2026-10-19
---

# Spec JSON v1

Nine queries, each `--json`, each exit 0 with a JSON object on success.
`promote` and `import` write files; their JSON reports what was written.

| Query | Answers |
|---|---|
| `root` | absolute spec root + how it was found (`source`) |
| `lessons-for --tag <state>` | lessons tagged for the given flow state |
| `plan --feature <f>` | that feature's plan rows |
| `feature <f>` | that feature's product/tech/plan summary |
| `validate` | layout, strictness, examined count, error and warning counts, `checks` (`{id, level}`) and `findings` (`{check, severity, file?, message}`) |
| `trace` | unit and citation counts, `history` (`all\|since\|none`), `uncited` units, `orphans` |
| `check-drift` | drift findings and counts |
| `promote` | what the promotion wrote or would write, and the checklist |
| `openspec` | what `export` or `import` wrote (both directions) |

Exit 3 = no spec tree found at any point in the discovery order — the one
query-specific exit code (see the exit-code contract below). Degrade path:
if `vibe-spec` is absent from PATH, a peer checks only that
`<specroot>/features/<f>/` exists, using its own copy of the discovery
order, and falls back to a bare path pointer instead of the summarized
answer — never a hard failure.

Fields are defined in `contracts/spec-json.schema.json`. Required sets are
minimal: `root {v, root, source}`, `lessons-for {v, tags, lessons}`,
`plan {v, feature, rows}`, `feature {v, feature, layer, docs}`.
`root.source` names the discovery step that hit: `flag`, `env`, `config`,
`docs-spec`, `dot-spec`. Spec JSON is a new `--json` output; the legacy
`lessons-for.sh --format json` array is a different shape and stays
byte-identical except that the port emits valid JSON: it escapes quotes,
backslashes and control characters and replaces invalid UTF-8 with U+FFFD
(a pinned divergence).

`doc.kind` is `design`, `plan`, `product`, `tech` or `research`.
`contracts/run.mjs` pins every required set and the `root.source` order
exactly, so dropping or adding a required field fails the gate.

`plan` rows come from the newest `docs/superpowers/plans/YYYY-MM-DD-<f>.md`:
one row per `### Task N:`, with status `done`, `in-progress` or `todo` from
its checkboxes. Without that plan, they come from the `| Unit | Status |`
table of `<root>/features/<f>/plan.md`. `feature` reports the layer
`superpowers` when a design or plan artifact exists. Otherwise it reports
`fallback`, with the folder's `product|tech|design|plan|research` docs.
