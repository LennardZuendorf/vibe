---
type: feature-tech
feature: spec-cli-migration
sibling: product.md
parent: ../../tech.md
updated: 2026-07-04
---

# Feature: spec-CLI migration — Architecture

Reimplement the six `spec/scripts/*.sh` scripts as native Python behind the
existing `vibe spec` Typer group, backed by importable logic modules, and pin
each against its bash origin with a byte-parity suite. The bash scripts stay in
place (canonical for the skill context) until a separate decision retires them.
This mirrors the vibe-cli flow port: Python parallel implementation, bash
retained, parity as the merge gate.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## What each script does (the port surface)

| Script | Behavior | Port difficulty |
|---|---|---|
| `setup.sh` | Scaffold `.spec/` from `reference/templates/`; skip existing; emit lessons.md stub; print writing-order guide | Low — file copies + fixed stdout |
| `list-specs.sh` | List `.spec/*.md` + `features/*/` with `type`/`scope`/`area` parsed from frontmatter | Low — glob + line parse |
| `lessons-for.sh` | Extract lessons whose `**Tags:**` match args; emit `markdown`/`inject`/`json` | Medium — block parsing + 3 formats |
| `scan-merges.sh` | Report `<!-- merge -->` blocks across feature `tech.md`; `table`/`json`/`plain`; nonzero on unclosed | Medium — stateful scan + formats |
| `promote.sh` | Extract `<!-- merge -->` blocks from a feature `tech.md`, append to a target; `--dry-run`/`--target` | Medium — extract + atomic append |
| `validate.sh` | ~640 lines: frontmatter, links, naming, feature folders, design tokens (SF3), requirement/scenario blocks (SF10), plan structure/traceability (SF11–12), stale links (SF13), scope conflicts (SF14), root length (SF15), lessons tags (SF16) | **High** — heavy `awk`; the whole risk of the feature |

The two that already exist in `spec_cmd.py` (`validate`, `setup`) do so by
`subprocess.run(["bash", script])` and streaming output verbatim
([`spec_cmd.py`](../../../cli/src/vibe/commands/spec_cmd.py) `_run_spec_script`).
Migration replaces that shell-out with a native implementation.

---

## Files

```
cli/src/vibe/spec/__init__.py           # new package: importable spec logic (rich-path OK)
cli/src/vibe/spec/model.py              # SpecDoc/frontmatter parse, area/type/scope helpers
cli/src/vibe/spec/validate.py           # port of validate.sh (SF checks) -> findings list
cli/src/vibe/spec/setup.py              # scaffold logic (template copy, lessons stub)
cli/src/vibe/spec/listing.py            # list-specs logic
cli/src/vibe/spec/lessons.py            # lessons-for: block parse + markdown/inject/json
cli/src/vibe/spec/merges.py            # scan-merges + promote (shared marker scan)
cli/src/vibe/commands/spec_cmd.py       # EXTEND: native subcommands + keep --root; drop subprocess
cli/tests/test_spec_validate_parity.py  # byte-parity vs spec/scripts/validate.sh
cli/tests/test_spec_setup_parity.py     # byte-parity vs setup.sh
cli/tests/test_spec_listing_parity.py   # byte-parity vs list-specs.sh
cli/tests/test_spec_lessons_parity.py   # byte-parity vs lessons-for.sh (x3 formats)
cli/tests/test_spec_merges_parity.py    # byte-parity vs scan-merges.sh + promote.sh
```

Unchanged and load-bearing: `spec/scripts/*.sh` (canonical for skill context),
`cli/src/vibe/_assets/skills/spec/scripts/*.sh` (bundled copies), and
[`test_assets_sync.py`](../../../cli/tests/test_assets_sync.py) which pins them
byte-identical.

---

## Contract / API

Each logic module exposes a pure function returning data + a render step, so
tests assert on data and the parity tests assert on rendered stdout:

```python
# vibe/spec/validate.py
def run(root: Path) -> ValidationReport: ...      # errors/warnings, exit_code
def render(report: ValidationReport) -> str: ...  # byte-identical to validate.sh stdout

# vibe/spec/lessons.py
def extract(root: Path, tags: list[str]) -> list[Lesson]: ...
def render(lessons: list[Lesson], fmt: Literal["markdown","inject","json"]) -> str: ...

# vibe/spec/merges.py
def scan(root: Path, feature: str | None) -> ScanResult: ...   # blocks + unclosed flag
def promote(root: Path, feature: str, target: Path, dry_run: bool) -> PromoteResult: ...
```

`spec_cmd.register(app)` keeps its current signature; the new subcommands mount
under the same `spec` group. `--root` stays the way to target a project.

---

## Implementation Detail

- **Byte-parity is the bar, not "equivalent".** The current wrappers already
  guarantee byte-identical stdout by streaming the script's output
  ([`test_spec_cmd.py`](../../../cli/tests/test_spec_cmd.py) asserts
  `result.stdout == direct.stdout`). The native port must preserve that,
  including ANSI color codes (`\033[31m …`), the exact ` ERROR: `/` WARN: `/
  ` OK: ` prefixes, the `Errors:`/`Warnings:` footer, and setup's writing-order
  block. This constrains formatting precisely and is what the parity suite pins.
- **`validate.sh` is the hard unit.** Its `awk` programs encode subtle rules
  (empty design-token detection, requirement/scenario counting, R-ID
  traceability, scope-conflict dedup on a temp file). Port check-by-check, each
  with its own fixtures reused from `tests/spec/run.sh`, and diff against the
  script continuously. Environment knobs (`VIBE_DESIGN_LINT`, `SPEC_DIR`,
  `SPEC_ROOT_MAX_LINES`) must be honored identically. The optional `design.md`
  network lint (SF4, `npx @google/design.md`) should shell out unchanged or stay
  gated off by default — reproducing it in Python is out of scope and risky.
- **Parity test harness already exists to copy.** The flow parity tests
  (`test_parity_policy.py`, `test_parity_orders.py`) and the `bash_ref` conftest
  fixture (locates repo `*.sh`, skips without `jq`/`bash`) are the template;
  point new fixtures at `spec/scripts/*.sh` and the bundled copy resolved by
  `spec_cmd._resolve_script`.
- **No hot-path constraint.** Unlike `vibe-hook`, spec commands never run on the
  per-Edit guard path, so `rich`/`pydantic` are fine here. Keep logic in
  `vibe/spec/` importable (no Typer import) so parity tests call `render()`
  directly and a future skill-repoint can reuse it.

<!-- merge -->
Migration pattern for a bash→CLI port in this repo: reimplement natively behind
the Typer command, keep the `.sh` as the canonical fallback, and gate the swap
on a byte-for-byte parity suite (skip-with-message without `bash`). Retire the
bash only in a separate, explicitly-approved step once every context that
invokes it is guaranteed to have the CLI on `PATH`. The spec half stayed bash by
design (vibe-cli R5/D1); reversing that is a decision, not a refactor.
<!-- /merge -->

## Open Questions

1. **`validate.sh` color/ordering fidelity.** The script interleaves per-file
   `--- name ---` headers with colored findings in file-glob order, then runs
   the SF13–16 global checks last. Byte-parity requires reproducing that exact
   interleaving and glob order (`shopt -s nullglob`, `.spec/*.md` sorted by the
   shell). Confirm the shell's glob order (locale-sensitive) is stable enough to
   pin, or normalize both sides.
2. **SF4 network lint.** Keep as a bash shell-out, or drop from the CLI path and
   document the divergence? It is advisory and already graceful-degrades.
