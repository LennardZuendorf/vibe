# Layered spec root

Read this before you add a note to a layered root. `vibe-spec root --json`
prints `"layout": "layered"` for it; `vibe-spec init` creates one.

## Layout

```
<root>/
  README.md  AGENTS.md          # entry and scoped guide
  product.md tech.md design.md plan.md   # the four hubs: type: hub, ≤80 lines, links only
  lessons.md                    # tagged lessons, no line budget
  areas/ flows/ concepts/ platform/ tech/ decisions/ questions/   # kind folders: typed notes
  parts/<part>/                 # a part hub plus its own kind folders
  features/<feature>/           # fallback feature layer only
  archive/ quick/               # exempt from the layout checks
```

## Rules

1. A hub holds no detail. It says what the area is in one or two lines and links notes.
2. A note sits in the kind folder that matches its `type:`, is at most 60 lines, and
   has `updated:`. Over budget: split it into a folder with its own `README.md` hub.
3. Note types by folder: `areas/` area, `flows/` flow, `concepts/` concept,
   `platform/` platform, `tech/` tech, `decisions/` decision, `questions/` question.
4. An area note lists the code it covers in `code:` (paths that exist). A tech note
   lists the files it governs in `applies:` (a glob).
5. File and folder names are lowercase kebab-case and unique across the tree.
6. Every note is linked from a hub, a part hub or its folder `README.md`.
7. A topic is a note in a kind folder, never a root `*-<topic>.md` file.

## Worked example: record a cross-cutting decision

Your human partner says: "We store data in SQLite." Write one concept note, one
decision note, and link both from the tech hub.

`<root>/concepts/storage.md`:

```markdown
---
type: concept
updated: 2026-10-24
---

# Storage

Application data lives in one SQLite file per workspace.
Decision: [use SQLite](../decisions/use-sqlite.md).
```

`<root>/decisions/use-sqlite.md`:

```markdown
---
type: decision
updated: 2026-10-24
---

# Use SQLite

**Decision:** store application data in SQLite.
**Why:** one file, no server, enough for one workspace.
**Instead of:** a client-server database.
```

Add to `<root>/tech.md` (bump its `updated:`):

```markdown
- [Storage](concepts/storage.md): where application data lives.
- [Use SQLite](decisions/use-sqlite.md): the storage decision.
```

Check: `vibe-spec validate --strict` exits 0 with no SF30, SF31 or SF33 finding.
