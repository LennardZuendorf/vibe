"""Tests for the shared strict marker implementation (``vibe.markers``)."""

from __future__ import annotations

import pytest

from vibe import markers

NAME = "vibe:instructions"
START, END = markers.marker_lines(NAME)


def _doc(inner: str) -> str:
    """A document with user prose surrounding a managed ``NAME`` region."""
    return (
        "# Project\n"
        "\n"
        "User preamble that must survive.\n"
        f"{START}\n"
        f"{inner}\n"
        f"{END}\n"
        "\n"
        "User epilogue that must survive.\n"
    )


def test_find_region_returns_marker_line_indices() -> None:
    text = _doc("managed body")
    region = markers.find_region(text, NAME)
    assert region is not None
    start_idx, end_idx = region
    lines = text.split("\n")
    assert lines[start_idx] == START
    assert lines[end_idx] == END


def test_find_region_absent_returns_none() -> None:
    assert markers.find_region("no markers here\n", NAME) is None
    # Only one of the pair present is still "absent" (incomplete region).
    assert markers.find_region(f"{START}\nlonely\n", NAME) is None


def test_replace_region_round_trips() -> None:
    text = _doc("old body")
    new_block = markers.wrap(NAME, "new body\nsecond line")
    replaced = markers.replace_region(text, NAME, new_block)

    # Managed content swapped...
    assert "old body" not in replaced
    assert "new body" in replaced
    assert "second line" in replaced
    # ...and the surrounding user prose preserved byte-for-byte.
    assert "User preamble that must survive." in replaced
    assert "User epilogue that must survive." in replaced

    # Region still well-formed and re-extractable.
    region = markers.find_region(replaced, NAME)
    assert region is not None

    # Replacing with the identical current block is a no-op.
    current = markers.wrap(NAME, "new body\nsecond line")
    assert markers.replace_region(replaced, NAME, current) == replaced


def test_replace_region_missing_raises() -> None:
    with pytest.raises(markers.MarkerError):
        markers.replace_region("plain text\n", NAME, markers.wrap(NAME, "x"))


def test_append_block_adds_block_after_newline() -> None:
    text = "existing user content\n"
    block = markers.wrap(NAME, "appended body")
    out = markers.append_block(text, block)
    assert out == f"{text}\n{block}"
    assert markers.find_region(out, NAME) is not None
    assert "existing user content" in out


def test_unmerge_removes_block_and_preserves_prose() -> None:
    text = _doc("managed body")
    out = markers.unmerge(text, NAME)
    assert START not in out
    assert END not in out
    assert "managed body" not in out
    assert "User preamble that must survive." in out
    assert "User epilogue that must survive." in out


def test_unmerge_absent_is_noop() -> None:
    text = "no managed block here\n"
    assert markers.unmerge(text, NAME) == text


def test_reversed_markers_raise_not_mangle() -> None:
    # End marker appears BEFORE the start marker: refuse rather than drop content.
    reversed_text = (
        "keep me above\n"
        f"{END}\n"
        "trapped middle\n"
        f"{START}\n"
        "keep me below\n"
    )
    with pytest.raises(markers.MarkerError):
        markers.find_region(reversed_text, NAME)
    with pytest.raises(markers.MarkerError):
        markers.replace_region(reversed_text, NAME, markers.wrap(NAME, "x"))
    with pytest.raises(markers.MarkerError):
        markers.unmerge(reversed_text, NAME)


def test_has_region_true_false_and_reversed() -> None:
    assert markers.has_region(_doc("body"), NAME) is True
    assert markers.has_region("nothing\n", NAME) is False
    with pytest.raises(markers.MarkerError):
        markers.has_region(f"{END}\n{START}\n", NAME)
