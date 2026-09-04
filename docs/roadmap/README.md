# v0.4 roadmap

The v0.4 re-architecture ("three tools, one language, no bundle") as a GitHub
issue set, authored from `.spec/plan.md` rows F1–F24. `issues/` holds one
Markdown file per issue (`00-epic.md`, `01-F1-*.md` … `24-F24-*.md`), each
with YAML frontmatter (`id`, `tool`, `phase`, `depends_on`, `parent`) and the
same body template GitHub issues would get: Deliverable, Scope, Acceptance,
Depends on, Decisions this implements, Notes.

## Why files instead of issues

GitHub Issues is currently disabled on this repo (Settings → General →
Features → Issues). These files are the fallback: everything is drafted and
ready to post the moment Issues is turned back on.

## Posting them

**Prerequisite:** enable Issues — repo Settings → General → Features →
check "Issues".

Then, either:

1. **Script:** `bash docs/roadmap/create-issues.sh --dry-run` to preview
   (needs no `gh`), then `bash docs/roadmap/create-issues.sh` to create the
   epic and all 24 children (each body ends with `Parent: #<epic>`), fill in
   the epic's checklist with real issue numbers, and print a final
   `F<n> → #<num>` table. Requires `gh` on PATH and `gh auth status` clean.
   Bash 3.2 compatible.
2. **Agent:** ask Claude to post them via the GitHub MCP `issue_write` tool,
   same content, same order.

## Source of truth

`.spec/plan.md` is canonical. If the plan changes, regenerate these files —
never hand-edit a posted GitHub issue without updating `.spec/plan.md` first,
and never hand-edit these files to drift from the plan.
