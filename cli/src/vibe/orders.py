"""Resolve the per-turn orders (D12) — stdlib only, hot-path safe.

Ports ``flow/scripts/orders.sh``: the orders for a state resolve in three tiers,
highest first, and resolution never fails —

1. the linked skill's ``<!-- vibe:orders:<state> -->`` block in its ``SKILL.md``
   (the machine's ``skill`` field names the skill; the block is read from
   ``skills_dir/<skill>/SKILL.md``);
2. the machine's inline ``inject`` string (``idle`` and any skill-less state);
3. a generic one-line fallback (:data:`GENERIC_FALLBACK`).

``<feature>`` is the only interpolation and only when a feature is carried, so
the output stays byte-stable per state (prompt-cache safe).

**Call-site read location.** :func:`resolve` reads the block from the bundled
``_assets/skills/<skill>/SKILL.md`` by default — the copy used for parity and for
the ``vibe-hook inject`` path. The ``skills_dir`` seam lets the hook (unit 7)
point at the *target's* ``.claude/skills/vibe/SKILL.md`` at hook time instead.

Stdlib-only at import (only the stdlib plus the stdlib-only :mod:`vibe.machine`
and :mod:`vibe.cursor`): safe on the ``vibe-hook`` per-Edit guard hot path — no
``typer``/``rich``/``pydantic``.
"""

from __future__ import annotations

import os
from pathlib import Path

from vibe import cursor, machine
from vibe.errors import VibeError

__all__ = ["GENERIC_FALLBACK", "resolve", "resolve_current"]

#: Last-resort one-liner when neither a skill block nor a machine inject exists.
#: Byte-identical to ``orders.sh``'s ``GENERIC_FALLBACK``.
GENERIC_FALLBACK = (
    "state=unknown · read .agents/skills/vibe/state-machine.json and pick the "
    "matching vibe phase · transition via set-state.sh"
)

_SKILLS_DIR = Path(__file__).resolve().parent / "_assets" / "skills"

_OPEN = "<!-- vibe:orders:{state} -->"
_CLOSE = "<!-- /vibe:orders -->"


def _extract_block(skill_md: Path, state: str) -> str | None:
    """Return the orders block for ``state`` from ``skill_md``, else ``None``.

    Mirrors ``orders.sh``'s ``awk`` extractor: collect the lines strictly between
    ``<!-- vibe:orders:<state> -->`` and ``<!-- /vibe:orders -->`` (exact,
    whole-line matches), joined by ``\\n``. Returns ``None`` when the opening
    marker is absent or the file cannot be read; an empty block yields ``""``
    (falsy), which the caller treats as "no block" exactly as the bash ``-n`` test
    does.
    """
    try:
        text = skill_md.read_text(encoding="utf-8")
    except OSError:
        return None
    records = text.split("\n")
    if records and records[-1] == "":
        records.pop()
    want = _OPEN.format(state=state)
    grabbing = False
    lines: list[str] = []
    for line in records:
        if not grabbing:
            if line == want:
                grabbing = True
            continue
        if line == _CLOSE:
            break
        lines.append(line)
    if not grabbing:
        return None
    return "\n".join(lines)


def _interpolate(text: str, feature: str | None) -> str:
    """Replace every ``<feature>`` with ``feature`` when one is carried.

    Ports ``orders.sh``'s ``interpolate``: with no feature the literal
    ``<feature>`` placeholder is left intact (still valid, still byte-stable).
    """
    if feature:
        return text.replace("<feature>", feature)
    return text


def _machine() -> machine.Machine | None:
    try:
        return machine.default()
    except VibeError:
        return None


def resolve(
    state: str,
    feature: str | None = None,
    *,
    skills_dir: str | os.PathLike[str] | None = None,
) -> str:
    """Resolve the orders string for ``state`` (never raises).

    Tiers: linked-skill block → machine ``inject`` → :data:`GENERIC_FALLBACK`.
    ``feature`` interpolates ``<feature>`` when truthy. ``skills_dir`` overrides
    where the skill block is read from (defaults to the bundled ``_assets/skills``);
    the skill name comes from the machine's ``skill`` field for ``state``.
    """
    m = _machine()
    skills = Path(skills_dir) if skills_dir is not None else _SKILLS_DIR

    skill: str | None = None
    inject: str | None = None
    if m is not None:
        try:
            data = m.state(state)
        except VibeError:
            data = None
        if data is not None:
            skill = data.get("skill")
            inject = data.get("inject")

    if skill:
        block = _extract_block(skills / skill / "SKILL.md", state)
        if block:
            return _interpolate(block, feature)

    if inject:
        return _interpolate(inject, feature)

    return GENERIC_FALLBACK


def resolve_current(
    project_root: str | os.PathLike[str],
    *,
    skills_dir: str | os.PathLike[str] | None = None,
) -> str:
    """Resolve orders for the cursor at ``project_root`` (missing → ``idle``).

    Reads ``{flow, phase, feature}`` from the project's cursor via
    :mod:`vibe.cursor`; a missing or malformed cursor degrades to ``idle`` with no
    feature, matching ``orders.sh``'s ``current_state``/``current_feature``.
    """
    cur_path = cursor.path_for(project_root)
    try:
        cur = cursor.read(cur_path)
    except VibeError:
        cur = None
    state = cur.key if cur is not None else "idle"
    feature = cur.feature if cur is not None else None
    return resolve(state, feature, skills_dir=skills_dir)
