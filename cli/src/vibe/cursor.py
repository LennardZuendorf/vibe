"""Read + atomic-write of the flow cursor ``state.json`` (stdlib only).

The cursor ``{flow, phase, feature, updated}`` is writer-only through this
module — the CLI's sanctioned replacement for ``flow/scripts/set-state.sh``.
:func:`write` ports that script's semantics exactly:

* the target must be a known, non-modifier state — ``amend`` and unknown keys
  are rejected (validation is driven by :mod:`vibe.machine`, never a hardcoded
  list);
* ``feature`` carries forward from the current cursor unless a new one is given,
  and moving to ``idle`` clears it to ``null``;
* ``updated`` is stamped ``date -u +%Y-%m-%dT%H:%M:%SZ`` (UTC, seconds, literal
  ``Z``); and
* the write is atomic (temp file in the target dir + :func:`os.replace`), so a
  crash mid-write never leaves a partial cursor.

:func:`current_state` mirrors ``detect-context.sh``'s ``current_state`` — a
missing or malformed cursor resolves to ``idle``, and a JSON-null ``flow``/
``phase`` coalesces to ``idle`` — so the policy layer (unit 4) can resolve the
cursor through this module and stay byte-for-byte parity with the bash origin.

Stdlib-only at import (``json``/``os``/``tempfile``/``datetime``/``dataclasses``
plus the stdlib-only :mod:`vibe.machine`): safe on the ``vibe-hook`` per-Edit
guard hot path — no ``typer``/``rich``/``pydantic``.
"""

from __future__ import annotations

import json
import os
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

from vibe import machine
from vibe.errors import VibeError

__all__ = [
    "Cursor",
    "RELPATH",
    "path_for",
    "read",
    "write",
    "seed",
    "current_state",
]

#: Project-relative location of the cursor — the exact path the policy layer
#: hard-blocks against direct edits. This module is that path's only writer.
RELPATH = Path(".agents") / "skills" / "vibe" / "state.json"


@dataclass(frozen=True)
class Cursor:
    """An immutable view of the cursor's ``{flow, phase, feature, updated}``."""

    flow: str
    phase: str
    feature: str | None
    updated: str

    @property
    def key(self) -> str:
        """The compound ``<flow>.<phase>`` state key (``idle`` when equal)."""
        return machine.join_key(self.flow, self.phase)

    def to_dict(self) -> dict:
        """Ordered dict for serialization (matches ``set-state.sh`` field order)."""
        return {
            "flow": self.flow,
            "phase": self.phase,
            "feature": self.feature,
            "updated": self.updated,
        }


def path_for(project_root: str | os.PathLike[str]) -> Path:
    """Return the cursor path for ``project_root`` (``root/`` + :data:`RELPATH`)."""
    return Path(project_root) / RELPATH


def read(path: str | os.PathLike[str]) -> Cursor | None:
    """Read the cursor at ``path``; return ``None`` when the file is absent.

    A JSON-null or absent ``flow``/``phase`` coalesces to ``idle`` (matching
    ``detect-context.sh``'s ``// "idle"``). Raises :class:`~vibe.errors.VibeError`
    on malformed JSON or a non-object document.
    """
    p = Path(path)
    if not p.exists():
        return None
    try:
        raw = p.read_text(encoding="utf-8")
    except OSError as exc:
        raise VibeError(f"cannot read cursor at {p}") from exc
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise VibeError(f"cursor at {p} is not valid JSON: {exc}") from exc
    if not isinstance(data, dict):
        raise VibeError(f"cursor at {p} is not a JSON object")
    return Cursor(
        flow=data.get("flow") or "idle",
        phase=data.get("phase") or "idle",
        feature=data.get("feature") or None,
        updated=data.get("updated") or "",
    )


def _read_lenient(path: str | os.PathLike[str]) -> Cursor | None:
    """Like :func:`read` but never raises — a corrupt cursor reads as absent.

    Ports ``set-state.sh``'s ``jq ... 2>/dev/null || echo null`` tolerance and
    ``detect-context.sh``'s degrade-to-idle behaviour.
    """
    try:
        return read(path)
    except VibeError:
        return None


def _now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _atomic_write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=path.name + ".", suffix=".tmp")
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


def write(
    path: str | os.PathLike[str],
    target: str,
    feature: str | None = None,
) -> Cursor:
    """Atomically write the cursor to ``target``, porting ``set-state.sh``.

    ``target`` must be a legal cursor state — a modifier (``amend``) or an
    unknown key raises :class:`~vibe.errors.VibeError` and leaves any existing
    cursor untouched. ``feature`` wins when given; otherwise it carries forward
    from the current cursor, and moving to ``idle`` clears it. Returns the new
    :class:`Cursor`.
    """
    p = Path(path)
    m = machine.default()
    if not m.is_valid_target(target):
        if target in m.states:
            raise VibeError(
                f"'{target}' is a modifier, not a cursor state; run it as a "
                "targeted scope edit and continue in your current state — the "
                "cursor is unchanged"
            )
        legal = ", ".join(m.valid_targets())
        raise VibeError(f"'{target}' is not a known state. Known states: {legal}")

    flow, phase = machine.split_key(target)

    if feature:
        new_feature: str | None = feature
    elif flow == "idle":
        new_feature = None
    else:
        current = _read_lenient(p)
        new_feature = current.feature if current is not None else None

    result = Cursor(flow=flow, phase=phase, feature=new_feature, updated=_now())
    _atomic_write(p, json.dumps(result.to_dict(), indent=2) + "\n")
    return result


def seed(path: str | os.PathLike[str]) -> Cursor:
    """Return the existing cursor, else write and return a fresh ``idle`` one.

    Seeds only when genuinely absent so a live cursor survives re-provisioning;
    snapshot/restore across a managed-file refresh is the installer's job.
    """
    existing = _read_lenient(path)
    if existing is not None:
        return existing
    return write(path, "idle")


def current_state(path: str | os.PathLike[str]) -> str:
    """Resolve the current compound state key, defaulting to ``idle``.

    Mirrors ``detect-context.sh``'s ``current_state``: a missing or malformed
    cursor — or one whose ``flow``/``phase`` is null — resolves to ``idle``.
    """
    cur = _read_lenient(path)
    return cur.key if cur is not None else "idle"
