"""Load + validate the canonical ``state-machine.json`` (stdlib ``json`` only).

``state-machine.json`` stays the single source of truth for the flow: 15 compound
``<flow>.<phase>`` states, their ``next``/``writes``/``inject``/``caveman`` fields,
and the declared ``flows``/``phases``/``modifiers`` vocabularies. This module reads
and validates that file with the standard-library :mod:`json` parser — **never**
``pydantic`` — so it is safe to import on the ``vibe-hook`` per-Edit guard hot path
alongside ``policy``/``orders``/``cursor``. Stdlib-only; safe to import anywhere.

Behaviour ports the bash originals so the Python and bash layers cannot drift:

* :func:`split_key` / :func:`join_key` reproduce ``set-state.sh`` and
  ``validate-state.sh`` exactly — a single-token state (``idle``) maps
  ``flow == phase``; otherwise the key splits on the first dot.
* :func:`is_valid_target` rejects ``amend`` the way ``set-state.sh`` does, driven
  by the machine's ``modifiers`` list rather than a hardcoded name — ``amend`` is
  present as a state yet is a modifier, never a stored cursor target.
"""

from __future__ import annotations

import json
from functools import lru_cache
from pathlib import Path

from vibe.errors import VibeError

__all__ = [
    "Machine",
    "load",
    "default",
    "state",
    "next_of",
    "inject_of",
    "writes_of",
    "caveman_of",
    "is_valid_target",
    "valid_targets",
    "split_key",
    "join_key",
]

_ASSET = Path(__file__).resolve().parent / "_assets" / "state-machine.json"

_REQUIRED_TOP = ("version", "initial", "flows", "phases", "modifiers", "states")


class Machine:
    """A validated, in-memory view of ``state-machine.json``.

    Construct via :func:`load` (an explicit path or the bundled asset) rather
    than directly; the constructor assumes ``data`` has already been validated.
    """

    def __init__(self, data: dict) -> None:
        self.data = data
        self.version: str = data["version"]
        self.initial: str = data["initial"]
        self.flows: list[str] = data["flows"]
        self.phases: list[str] = data["phases"]
        self.modifiers: list[str] = data["modifiers"]
        self.states: dict[str, dict] = data["states"]

    def state(self, key: str) -> dict:
        """Return the raw state dict for compound ``key`` (e.g. ``feature.impl``).

        Raises :class:`~vibe.errors.VibeError` when ``key`` is not a known state.
        ``amend`` resolves (it is a present state) even though it is not a legal
        cursor target — see :meth:`is_valid_target`.
        """
        try:
            return self.states[key]
        except KeyError as exc:
            raise VibeError(f"'{key}' is not a known state") from exc

    def next_of(self, key: str) -> list[str]:
        """Return a copy of the legal ``next`` states for ``key``."""
        return list(self.state(key).get("next", []))

    def inject_of(self, key: str) -> str | None:
        """Return the inline ``inject`` for ``key`` (``None`` for skill-owned states)."""
        return self.state(key).get("inject")

    def writes_of(self, key: str) -> list[str]:
        """Return a copy of the declared write surface (``writes``) for ``key``."""
        return list(self.state(key).get("writes", []))

    def caveman_of(self, key: str) -> str:
        """Return the frozen caveman level for ``key``."""
        return self.state(key).get("caveman", "")

    def is_valid_target(self, key: str) -> bool:
        """Return True iff ``key`` is a legal ``set-state`` cursor target.

        A target must be a known state that is *not* a modifier; ``amend`` is a
        modifier, so it is rejected exactly as ``set-state.sh`` rejects it.
        """
        return key in self.states and key not in self.modifiers

    def valid_targets(self) -> list[str]:
        """Return the known states minus modifiers, in declaration order."""
        return [k for k in self.states if k not in self.modifiers]


def _validate(data: object, source: str) -> dict:
    if not isinstance(data, dict):
        raise VibeError(f"state machine at {source} is not a JSON object")
    missing = [k for k in _REQUIRED_TOP if k not in data]
    if missing:
        raise VibeError(
            f"state machine at {source} is missing required keys: {', '.join(missing)}"
        )
    states = data["states"]
    if not isinstance(states, dict) or not states:
        raise VibeError(f"state machine at {source} has no states")
    return data


def load(path: str | Path | None = None) -> Machine:
    """Load and validate a state machine from ``path`` (bundled asset when ``None``).

    Raises :class:`~vibe.errors.VibeError` on a missing file, malformed JSON, or a
    structurally incomplete machine (missing top-level keys / empty ``states``).
    """
    src = Path(path) if path is not None else _ASSET
    try:
        raw = src.read_text(encoding="utf-8")
    except OSError as exc:
        raise VibeError(f"state machine not found at {src}") from exc
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise VibeError(f"state machine at {src} is not valid JSON: {exc}") from exc
    return Machine(_validate(data, str(src)))


@lru_cache(maxsize=1)
def default() -> Machine:
    """Return the cached, validated view of the bundled ``state-machine.json``."""
    return load()


def split_key(key: str) -> tuple[str, str]:
    """Split a compound ``<flow>.<phase>`` key into ``(flow, phase)``.

    Ports ``set-state.sh``: a single-token key (``idle``) maps ``flow == phase``;
    otherwise the flow is everything before the first dot and the phase the rest.
    Pure string transform — it does not validate that the key exists.
    """
    flow, sep, phase = key.partition(".")
    return (flow, phase) if sep else (key, key)


def join_key(flow: str, phase: str) -> str:
    """Rebuild the compound key from ``flow``/``phase``.

    Ports ``validate-state.sh``: ``KEY == FLOW`` when ``flow == phase`` (the bare
    single-token state), else ``FLOW.PHASE``.
    """
    return flow if flow == phase else f"{flow}.{phase}"


def state(key: str) -> dict:
    """Look up ``key`` in the bundled machine (see :meth:`Machine.state`)."""
    return default().state(key)


def next_of(key: str) -> list[str]:
    """Legal ``next`` states for ``key`` in the bundled machine (copy)."""
    return default().next_of(key)


def inject_of(key: str) -> str | None:
    """Inline ``inject`` for ``key`` in the bundled machine."""
    return default().inject_of(key)


def writes_of(key: str) -> list[str]:
    """Declared write surface for ``key`` in the bundled machine (copy)."""
    return default().writes_of(key)


def caveman_of(key: str) -> str:
    """Frozen caveman level for ``key`` in the bundled machine."""
    return default().caveman_of(key)


def is_valid_target(key: str) -> bool:
    """Whether ``key`` is a legal cursor target in the bundled machine."""
    return default().is_valid_target(key)


def valid_targets() -> list[str]:
    """Legal cursor targets (states minus modifiers) in the bundled machine."""
    return default().valid_targets()
