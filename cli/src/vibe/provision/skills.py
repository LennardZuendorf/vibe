"""Copy engine for the bundled skill trees.

``vibe init`` provisions the two portable skill trees vendored under
``src/vibe/_assets/skills/{spec,vibe}/`` into a target project's
``.claude/skills/{spec,vibe}/`` for Claude Code auto-discovery. The canonical
``.agents/`` copy in the source repo is never touched here.

Public API (consumed by the ``init`` and ``uninstall``/``update`` commands):

``install(target, *, dry_run=False) -> list[Path]``
    Copy both trees into ``<target>/.claude/skills/``. Idempotent: a re-copy
    overwrites shipped files in place and leaves co-located user files alone.

``remove(target, *, dry_run=False) -> list[Path]``
    Surgical inverse — delete only the files this module ships (the per-file
    inverse of the copy), then prune emptied directories. Co-located user files
    and any non-empty directory survive.

``target`` is the project root (the directory that holds ``.claude/``), not the
``.claude`` directory itself. Both functions return the absolute paths acted on
(the files that were, or under ``dry_run`` would be, written/removed).

Stdlib-only: no typer/rich/pydantic at import.
"""

from __future__ import annotations

import contextlib
import shutil
from collections.abc import Iterator
from pathlib import Path

SKILL_NAMES: tuple[str, ...] = ("spec", "vibe")


def _assets_skills_root() -> Path:
    return Path(__file__).resolve().parent.parent / "_assets" / "skills"


def _target_skills_root(target: Path) -> Path:
    return Path(target) / ".claude" / "skills"


def _iter_shipped(src: Path) -> Iterator[Path]:
    for path in sorted(src.rglob("*"), key=lambda p: p.parts):
        if path.is_file():
            yield path.relative_to(src)


def install(target: Path, *, dry_run: bool = False) -> list[Path]:
    """Copy both bundled skill trees into ``<target>/.claude/skills/``."""
    src_root = _assets_skills_root()
    dst_root = _target_skills_root(target)
    written: list[Path] = []
    for name in SKILL_NAMES:
        src = src_root / name
        if not src.is_dir():
            continue
        for rel in _iter_shipped(src):
            dst = dst_root / name / rel
            written.append(dst)
            if dry_run:
                continue
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src / rel, dst)
    return written


def remove(target: Path, *, dry_run: bool = False) -> list[Path]:
    """Delete only the files ``install`` ships, then prune emptied dirs."""
    src_root = _assets_skills_root()
    dst_root = _target_skills_root(target)
    removed: list[Path] = []
    for name in SKILL_NAMES:
        src = src_root / name
        if not src.is_dir():
            continue
        skill_dir = dst_root / name
        for rel in _iter_shipped(src):
            dst = skill_dir / rel
            if dst.is_file() or dst.is_symlink():
                removed.append(dst)
                if not dry_run:
                    dst.unlink()
        if not dry_run:
            _prune_empty_dirs(skill_dir)
    if not dry_run:
        _prune_if_empty(dst_root)
        _prune_if_empty(dst_root.parent)
    return removed


def _prune_empty_dirs(root: Path) -> None:
    if not root.is_dir():
        return
    subdirs = (p for p in root.rglob("*") if p.is_dir())
    for directory in sorted(subdirs, key=lambda p: len(p.parts), reverse=True):
        _prune_if_empty(directory)
    _prune_if_empty(root)


def _prune_if_empty(directory: Path) -> None:
    with contextlib.suppress(OSError):
        directory.rmdir()


__all__ = ["install", "remove", "SKILL_NAMES"]
