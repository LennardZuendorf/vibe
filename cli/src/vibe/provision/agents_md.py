"""Provision a target's ``AGENTS.md`` from the bundled template.

Port of ``flow/scripts/merge-agents.sh``. The managed region logic lives once in
:mod:`vibe.markers` (the strict pairing/reversal guard); this module orchestrates
the five deterministic merge cases, the inverse ``unmerge``, and adapter
symlinking driven by the bundled ``_assets/adapters.json`` catalogue.

Merge cases (idempotent):

1. No target file            -> copy the template.
2. ``vibe:instructions`` block -> replace the content between the markers.
3. ``vibe:constitution`` block -> migrate the legacy region to ``vibe:instructions``.
4. No markers, body == template body (normalized) -> wrap (copy the template).
5. No markers, divergent     -> append the managed block (warn; never clobber).

Content OUTSIDE the markers (user preamble, the ``vibe:active-rules`` block) is
never touched. A reversed marker pair is refused, not mangled — that guard is
inherited from :mod:`vibe.markers`.

Assets are located relative to this package, never the CWD. Stdlib-only imports
plus the two intra-package leaves; no ``typer``/``rich``/``pydantic``.
"""

from __future__ import annotations

import json
import os
import shutil
import tempfile
from dataclasses import dataclass
from pathlib import Path

from vibe import markers
from vibe.errors import VibeError

__all__ = [
    "MergeResult",
    "LinkResult",
    "merge",
    "unmerge",
    "link",
    "adapters",
    "canonical",
]

_ASSETS = Path(__file__).resolve().parent.parent / "_assets"
_TEMPLATE = _ASSETS / "templates" / "AGENTS.md"
_ADAPTERS = _ASSETS / "adapters.json"

_INSTRUCTIONS = "vibe:instructions"
_CONSTITUTION = "vibe:constitution"
_MANAGED_COMMENT_PREFIX = "<!-- Managed by vibe-setup"


@dataclass(frozen=True)
class MergeResult:
    """Outcome of a :func:`merge` / :func:`unmerge` call.

    ``action`` is one of ``created``/``merged``/``noop``/``migrated``/``wrapped``/
    ``appended`` (merge) or ``removed``/``absent``/``no-file`` (unmerge).
    """

    action: str
    path: Path
    warning: str | None = None


@dataclass(frozen=True)
class LinkResult:
    """Outcome of an adapter :func:`link` call.

    ``action`` is one of ``linked``/``noop``/``relinked``/``refused``.
    """

    action: str
    path: Path
    target: str
    warning: str | None = None


def _load_adapters() -> dict:
    return json.loads(_ADAPTERS.read_text(encoding="utf-8"))


def canonical() -> str:
    """Return the canonical instruction filename from the adapters catalogue."""
    return _load_adapters()["canonical"]


def adapters() -> list[dict]:
    """Return the adapter catalogue entries from ``_assets/adapters.json``."""
    return _load_adapters()["adapters"]


def _target_for(adapter: str) -> str:
    data = _load_adapters()
    for entry in data["adapters"]:
        if entry["file"] == adapter:
            return entry["target"]
    return data["canonical"]


def _atomic_write(path: Path, text: str) -> None:
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=path.name + ".", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as handle:
            handle.write(text)
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def _template_block() -> str:
    text = _TEMPLATE.read_text(encoding="utf-8")
    region = markers.find_region(text, _INSTRUCTIONS)
    if region is None:
        raise VibeError(
            f"template at {_TEMPLATE} is missing its {_INSTRUCTIONS} markers"
        )
    start_idx, end_idx = region
    return "\n".join(text.split("\n")[start_idx : end_idx + 1])


def _template_core(text: str) -> str:
    """Inner ``vibe:instructions`` body with the managed-comment stripped.

    Mirrors the awk in ``merge-agents.sh`` that builds the wrap-comparison core.
    """
    start, end = markers.marker_lines(_INSTRUCTIONS)
    out: list[str] = []
    inside = False
    skipping_comment = False
    for line in text.split("\n"):
        if line == start:
            inside = True
            continue
        if line == end:
            inside = False
            continue
        if not inside:
            continue
        if line.startswith(_MANAGED_COMMENT_PREFIX):
            skipping_comment = True
        if skipping_comment:
            if line.endswith("-->"):
                skipping_comment = False
            continue
        out.append(line)
    return "\n".join(out)


def _normalize(text: str) -> str:
    """Strip trailing whitespace, trim edge blanks, squeeze internal blank runs.

    Byte-for-byte port of the ``sed`` + ``awk`` normalize in ``merge-agents.sh``,
    used only for the case-4 wrap comparison.
    """
    lines = [line.rstrip() for line in text.split("\n")]
    start = 0
    end = len(lines) - 1
    while start <= end and lines[start] == "":
        start += 1
    while end >= start and lines[end] == "":
        end -= 1
    out: list[str] = []
    blank = False
    for i in range(start, end + 1):
        if lines[i] == "":
            if blank:
                continue
            blank = True
        else:
            blank = False
        out.append(lines[i])
    return "\n".join(out)


def merge(root: Path) -> MergeResult:
    """Provision ``root/AGENTS.md`` from the bundled template (5 cases)."""
    if not _TEMPLATE.is_file():
        raise VibeError(f"template not found at {_TEMPLATE}")

    target = root / canonical()

    if not target.exists():
        shutil.copyfile(_TEMPLATE, target)
        return MergeResult("created", target)

    template_text = _TEMPLATE.read_text(encoding="utf-8")
    block = _template_block()
    text = target.read_text(encoding="utf-8")

    if markers.find_region(text, _INSTRUCTIONS) is not None:
        new_text = markers.replace_region(text, _INSTRUCTIONS, block)
        if new_text == text:
            return MergeResult("noop", target)
        _atomic_write(target, new_text)
        return MergeResult("merged", target)

    if markers.find_region(text, _CONSTITUTION) is not None:
        new_text = markers.replace_region(text, _CONSTITUTION, block)
        _atomic_write(target, new_text)
        return MergeResult("migrated", target)

    if _normalize(_template_core(template_text)) == _normalize(text):
        shutil.copyfile(_TEMPLATE, target)
        return MergeResult("wrapped", target)

    _atomic_write(target, markers.append_block(text, block))
    return MergeResult(
        "appended",
        target,
        warning=(
            f"{target} has no {_INSTRUCTIONS} markers and diverges from the "
            "template; appended the managed block — review and move your content"
        ),
    )


def unmerge(root: Path) -> MergeResult:
    """Remove the managed ``vibe:instructions`` region, preserving all prose."""
    target = root / canonical()
    if not target.exists():
        return MergeResult("no-file", target)

    text = target.read_text(encoding="utf-8")
    new_text = markers.unmerge(text, _INSTRUCTIONS)
    if new_text == text:
        return MergeResult("absent", target)
    _atomic_write(target, new_text)
    return MergeResult("removed", target)


def link(adapter: str, root: Path) -> LinkResult:
    """Symlink ``root/adapter`` -> its catalogue target (relative).

    Fresh -> create; already-correct -> no-op; wrong link -> relink (warn); a real
    file -> refuse (warn, never clobber).
    """
    target = _target_for(adapter)
    link_path = root / adapter

    if link_path.is_symlink():
        current = os.readlink(link_path)
        if current == target:
            return LinkResult("noop", link_path, target)
        link_path.unlink()
        link_path.symlink_to(target)
        return LinkResult(
            "relinked",
            link_path,
            target,
            warning=f"{adapter} symlinked {current}, not {target} — relinked",
        )

    if link_path.exists():
        return LinkResult(
            "refused",
            link_path,
            target,
            warning=(
                f"{adapter} is a real file — not replacing; show a diff and "
                "confirm before symlinking"
            ),
        )

    link_path.symlink_to(target)
    return LinkResult("linked", link_path, target)
