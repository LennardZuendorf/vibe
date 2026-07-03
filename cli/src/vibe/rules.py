"""Active-rules digest — port of ``flow/scripts/regen-active-rules.sh``.

Projects a capped top-5 digest of ``.spec/lessons.md`` into the managed
``vibe:active-rules`` block of ``CLAUDE.md`` / ``AGENTS.md``. ``lessons.md`` stays
canonical; this block is *generated output* (like a committed lockfile), not a
second source of truth.

Selection mirrors the bash origin exactly: pinned entries first (those carrying a
``**Pinned-by:**`` line — pinning is deliberately expensive), then most recent by
``**Date:**``; hard cap of 5. Formatting is byte-for-byte identical to the origin
so the parity suite (``tests/test_parity_rules.py``) is a merge gate — including
the literal ``regen-active-rules.sh`` mention in the generated note (renaming is a
retirement concern owned by unit 16, not this port).

Stdlib-only import: uses the shared :mod:`vibe.markers` guard (also stdlib-only)
for the strict marker-pairing / reversal protection, never re-implementing the
laxer grep the bash origin used.
"""

from __future__ import annotations

import os
import re
import tempfile
from pathlib import Path

from vibe import markers

__all__ = [
    "MARKER_NAME",
    "CAP",
    "GENERATED_NOTE",
    "EMPTY_DIGEST",
    "build_digest",
    "build_block",
    "write_rules",
]

MARKER_NAME = "vibe:active-rules"
CAP = 5

# Byte-exact with the origin's block template (line 2 is indented five spaces and
# names the script). Kept verbatim for parity — do NOT "correct" the name here.
GENERATED_NOTE = (
    "<!-- Generated from .spec/lessons.md by regen-active-rules.sh. "
    "Do not edit by hand;\n"
    f"     edit lessons.md and re-run during compound. Top {CAP}, pinned first. -->"
)
EMPTY_DIGEST = "_No lessons recorded yet._"

# Mirror awk's [[:space:]] (a split-out line can never contain the newline).
_COMMENT_OPEN = re.compile(r"^[ \t\r\v\f]*<!--")
_RULE_PREFIX = re.compile(r"^\*\*Rule:\*\* *")
_DATE_PREFIX = re.compile(r"^\*\*Date:\*\* *")

# A parsed lesson row, mirroring the tab-separated record the origin's awk emits:
# (pinned, date, title, rule). ``date`` already carries the "0000-00-00" default.
_Row = tuple[int, str, str, str]


def _parse_entries(text: str) -> list[_Row]:
    """Parse lesson entries from ``lessons.md`` text (ports the origin's awk).

    A line that *starts* with ``<!--`` opens a comment block (so an inline
    ``<!-- ... -->`` token inside a Rule body is safe); everything until a line
    containing ``-->`` is skipped — this drops the trailing format template that
    real ``lessons.md`` files carry.
    """
    rows: list[_Row] = []
    in_comment = False
    have = False
    title = ""
    rule = ""
    date = ""
    pinned = 0

    def emit() -> None:
        if have:
            rows.append((pinned, date if date else "0000-00-00", title, rule))

    for line in text.split("\n"):
        if _COMMENT_OPEN.match(line):
            in_comment = True
        if in_comment:
            if "-->" in line:
                in_comment = False
            continue
        if line.startswith("### "):
            emit()
            have = True
            title = line[4:]
            rule = ""
            date = ""
            pinned = 0
            continue
        if line.startswith("**Rule:**"):
            rule = _RULE_PREFIX.sub("", line)
            continue
        if line.startswith("**Date:**"):
            date = _DATE_PREFIX.sub("", line)
            continue
        if line.startswith("**Pinned-by:**"):
            pinned = 1
            continue

    emit()
    return rows


def _select(rows: list[_Row]) -> list[_Row]:
    """Order rows as ``sort -k1,1nr -k2,2r`` (with the forward whole-line last
    resort) and cap at :data:`CAP`.

    Emulated with three stable passes, least-significant first: whole-line
    ascending (the tiebreak GNU sort uses when all keys are equal), then date
    descending, then pinned descending.
    """
    ordered = sorted(rows, key=lambda r: f"{r[0]}\t{r[1]}\t{r[2]}\t{r[3]}")
    ordered.sort(key=lambda r: r[1], reverse=True)
    ordered.sort(key=lambda r: r[0], reverse=True)
    return ordered[:CAP]


def _format_digest(rows: list[_Row]) -> str:
    if not rows:
        return EMPTY_DIGEST
    lines = []
    for pinned, _date, title, rule in rows:
        pin = "\U0001f4cc " if pinned == 1 else ""
        lines.append(f"- {pin}**{title}** — {rule}")
    return "\n".join(lines)


def build_digest(lessons_path: Path) -> str:
    """Build the digest body (markdown lines) from ``lessons.md``.

    Graceful degrade, matching the origin: a missing file or an empty parse
    yields the placeholder :data:`EMPTY_DIGEST`, never a raise.
    """
    if not lessons_path.is_file():
        return EMPTY_DIGEST
    rows = _select(_parse_entries(lessons_path.read_text(encoding="utf-8")))
    return _format_digest(rows)


def build_block(lessons_path: Path) -> str:
    """Return the full managed ``vibe:active-rules`` block (markers included)."""
    digest = build_digest(lessons_path)
    body = f"{GENERATED_NOTE}\n\n### Active Rules\n\n{digest}"
    return markers.wrap(MARKER_NAME, body)


def _resolve_target(file: Path) -> Path:
    """Follow a one-level symlink to the file the block is written into.

    Mirrors the origin's ``readlink`` handling: temp files (and the block) must
    land beside the real target so the final rename is a same-directory atomic
    replace, not a symlink clobber.
    """
    if file.is_symlink():
        link = os.readlink(file)
        if os.path.isabs(link):
            return Path(link)
        return file.parent.resolve() / link
    return file


def _atomic_write(path: Path, text: str) -> None:
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=f"{path.name}.", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(text)
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def _write_block(file: Path, block: str) -> Path | None:
    """Write ``block`` into ``file`` (following one symlink level).

    Returns the resolved path actually written, or ``None`` when the target does
    not exist (the origin warns and skips). Replaces an existing managed region
    in place; otherwise appends a fresh block after a separating newline. The
    strict :mod:`vibe.markers` guard raises on a reversed marker pair rather than
    mangling — the intended hardening over the origin's lax grep.
    """
    target = _resolve_target(file)
    if not target.is_file():
        return None

    text = target.read_text(encoding="utf-8")
    if markers.has_region(text, MARKER_NAME):
        new_text = markers.replace_region(text, MARKER_NAME, block)
    else:
        new_text = markers.append_block(text, block)
    if not new_text.endswith("\n"):
        new_text += "\n"

    _atomic_write(target, new_text)
    return target


def write_rules(root: Path, targets: list[Path] | None = None) -> list[Path]:
    """Regenerate the active-rules block in ``root``'s ``CLAUDE.md``/``AGENTS.md``.

    ``targets`` defaults to ``[root/CLAUDE.md, root/AGENTS.md]``. Symlink-aware
    dedup (by real path) collapses a ``CLAUDE.md -> AGENTS.md`` alias so the block
    is written exactly once. Returns the resolved paths written, in order.
    """
    root = Path(root)
    if targets is None:
        targets = [root / "CLAUDE.md", root / "AGENTS.md"]

    block = build_block(root / ".spec" / "lessons.md")

    written: list[Path] = []
    seen: set[str] = set()
    for target in targets:
        resolved = os.path.realpath(target)
        if resolved in seen:
            continue
        seen.add(resolved)
        result = _write_block(Path(target), block)
        if result is not None:
            written.append(result)
    return written
