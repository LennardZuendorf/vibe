"""Managed HTML-comment marker blocks — the single shared implementation.

Ported from ``flow/scripts/merge-agents.sh`` (the STRICT variant, not the laxer
``regen-active-rules.sh`` grep). A managed region is delimited by a matched pair
of exact-line HTML comments::

    <!-- vibe:instructions:start -->
    ...managed content...
    <!-- vibe:instructions:end -->

Content OUTSIDE the pair is user-owned and never touched. The cardinal
invariant: if the markers are present but reversed (the end marker's line is at
or before the start marker's), we RAISE rather than mangle the file — a naive
region replace would silently drop trailing content. This guard lives once, in
``find_region``, so ``replace_region`` and ``unmerge`` both inherit it.

The functions operate on text (``str``) and preserve exact bytes: splitting on
``"\\n"`` and re-joining round-trips any input, including trailing-newline state.
Stdlib-only — safe to import anywhere, including the ``vibe-hook`` hot path.

The API is generic over the marker *name* (the token between ``vibe:`` prefixes
in practice, e.g. ``"vibe:instructions"`` or ``"vibe:active-rules"``). A block
passed to :func:`replace_region` / :func:`append_block` is expected to carry its
own start/end marker lines, matching the inclusive-region semantics of the bash
original (``extract_region`` yields the markers together with their content).
"""

from __future__ import annotations

__all__ = [
    "MarkerError",
    "marker_lines",
    "wrap",
    "find_region",
    "has_region",
    "replace_region",
    "append_block",
    "unmerge",
]


class MarkerError(ValueError):
    """Raised on a reversed/overlapping marker pair (refuse rather than mangle)."""


def marker_lines(name: str) -> tuple[str, str]:
    """Return the exact ``(start, end)`` marker lines for a managed ``name``."""
    return f"<!-- {name}:start -->", f"<!-- {name}:end -->"


def wrap(name: str, body: str) -> str:
    """Wrap ``body`` in the ``name`` marker pair, returning the full block.

    The result is suitable to pass to :func:`replace_region` or
    :func:`append_block`. ``body`` is inserted verbatim between the markers.
    """
    start, end = marker_lines(name)
    return f"{start}\n{body}\n{end}"


def find_region(text: str, name: str) -> tuple[int, int] | None:
    """Locate the managed region for ``name`` in ``text``.

    Returns ``(start_index, end_index)`` — the 0-based line indices of the first
    exact-match start marker and the first exact-match end marker (both
    inclusive of the marker lines themselves). Returns ``None`` when either
    marker is absent (there is no complete region to act on).

    Raises :class:`MarkerError` when both markers are present but reversed or
    overlapping (``start_index >= end_index``).
    """
    start, end = marker_lines(name)
    lines = text.split("\n")

    start_idx: int | None = None
    end_idx: int | None = None
    for i, line in enumerate(lines):
        if start_idx is None and line == start:
            start_idx = i
        if end_idx is None and line == end:
            end_idx = i

    if start_idx is None or end_idx is None:
        return None
    if start_idx >= end_idx:
        raise MarkerError(
            f"reversed {name} markers (start line {start_idx + 1}, "
            f"end line {end_idx + 1}) — refusing to mangle; fix by hand"
        )
    return start_idx, end_idx


def has_region(text: str, name: str) -> bool:
    """Return True when ``text`` holds a complete, well-ordered ``name`` region.

    Raises :class:`MarkerError` on a reversed pair, exactly like
    :func:`find_region` — a caller must not treat a reversed pair as "absent".
    """
    return find_region(text, name) is not None


def replace_region(text: str, name: str, block: str) -> str:
    """Replace the inclusive ``name`` region in ``text`` with ``block``.

    ``block`` is expected to carry its own start/end marker lines (see
    :func:`wrap`). Everything outside the region is preserved byte-for-byte.

    Raises :class:`MarkerError` if the existing region is missing (nothing to
    replace) or reversed.
    """
    region = find_region(text, name)
    if region is None:
        raise MarkerError(f"no {name} region to replace in target text")
    start_idx, end_idx = region

    lines = text.split("\n")
    new_lines = lines[:start_idx] + block.split("\n") + lines[end_idx + 1 :]
    return "\n".join(new_lines)


def append_block(text: str, block: str) -> str:
    """Append ``block`` to ``text`` after a single separating newline.

    Mirrors the bash ``printf '\\n'; cat block >> file`` fallback used when a
    target has no managed markers and diverges from the template.
    """
    return f"{text}\n{block}"


def unmerge(text: str, name: str) -> str:
    """Remove the inclusive ``name`` region from ``text``, preserving the rest.

    Returns ``text`` unchanged when the region is absent (mirrors the bash
    "left untouched" behaviour). Raises :class:`MarkerError` on a reversed pair.
    """
    region = find_region(text, name)
    if region is None:
        return text
    start_idx, end_idx = region

    lines = text.split("\n")
    new_lines = lines[:start_idx] + lines[end_idx + 1 :]
    return "\n".join(new_lines)
