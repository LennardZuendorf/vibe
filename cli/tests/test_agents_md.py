"""Tests for ``vibe.provision.agents_md`` (port of merge-agents.sh)."""

from __future__ import annotations

import os
from pathlib import Path

import pytest

from vibe import markers
from vibe.provision import agents_md

_INSTRUCTIONS = "vibe:instructions"
_CONSTITUTION = "vibe:constitution"
I_START, I_END = markers.marker_lines(_INSTRUCTIONS)
C_START, C_END = markers.marker_lines(_CONSTITUTION)

_TEMPLATE = Path(agents_md.__file__).resolve().parent.parent / "_assets" / "templates" / "AGENTS.md"


def _template_text() -> str:
    return _TEMPLATE.read_text(encoding="utf-8")


# ── case 1: no file -> copy template ────────────────────────────────────────────
def test_merge_no_file_copies_template(target_project) -> None:
    result = agents_md.merge(target_project)
    agents = target_project / "AGENTS.md"
    assert result.action == "created"
    assert result.path == agents
    assert agents.read_text(encoding="utf-8") == _template_text()
    assert markers.has_region(agents.read_text(encoding="utf-8"), _INSTRUCTIONS)


# ── case 2: markers present -> replace, prose preserved ─────────────────────────
def test_merge_replaces_marker_block_and_preserves_prose(target_project) -> None:
    agents = target_project / "AGENTS.md"
    agents.write_text(
        "# My Project\n"
        "\n"
        "USER PREAMBLE that must survive.\n"
        f"{I_START}\n"
        "STALE managed content to be replaced\n"
        f"{I_END}\n"
        "\n"
        "USER EPILOGUE that must survive.\n",
        encoding="utf-8",
    )
    result = agents_md.merge(target_project)
    out = agents.read_text(encoding="utf-8")

    assert result.action == "merged"
    assert "STALE managed content" not in out
    assert "Prime Directive" in out  # from the template's managed block
    assert "USER PREAMBLE that must survive." in out
    assert "USER EPILOGUE that must survive." in out
    assert markers.find_region(out, _INSTRUCTIONS) is not None


def test_merge_is_idempotent(target_project) -> None:
    agents_md.merge(target_project)  # created
    first = agents_md.merge(target_project)
    second = agents_md.merge(target_project)
    assert first.action == "noop"
    assert second.action == "noop"


def test_merge_appended_block_is_then_idempotent(target_project) -> None:
    agents = target_project / "AGENTS.md"
    agents.write_text("# Wholly different doc\n\nUnrelated prose.\n", encoding="utf-8")
    first = agents_md.merge(target_project)
    second = agents_md.merge(target_project)
    assert first.action == "appended"
    assert second.action == "noop"


# ── case 3: legacy vibe:constitution -> migrate ─────────────────────────────────
def test_merge_migrates_legacy_constitution_block(target_project) -> None:
    agents = target_project / "AGENTS.md"
    agents.write_text(
        "# Legacy Project\n"
        "\n"
        "USER PREAMBLE survives migration.\n"
        f"{C_START}\n"
        "old constitution body\n"
        f"{C_END}\n"
        "\n"
        "USER EPILOGUE survives migration.\n",
        encoding="utf-8",
    )
    result = agents_md.merge(target_project)
    out = agents.read_text(encoding="utf-8")

    assert result.action == "migrated"
    assert C_START not in out
    assert C_END not in out
    assert "old constitution body" not in out
    assert markers.find_region(out, _INSTRUCTIONS) is not None
    assert "USER PREAMBLE survives migration." in out
    assert "USER EPILOGUE survives migration." in out


# ── case 4: no markers, body matches template body (normalized) -> wrap ──────────
def test_merge_wraps_normalized_equivalent_body(target_project) -> None:
    # Build a no-markers target from the template's managed body, then inject
    # cosmetic noise that normalize() is required to absorb: trailing whitespace
    # (stripped), a doubled blank run (squeezed to one), and edge blanks
    # (trimmed). If normalize is wrong, this diverges and would append (fail).
    core = agents_md._template_core(_template_text())
    # Pin _template_core independently of the wrap comparison: the multi-line
    # managed comment must be fully stripped, the markers gone, real body kept.
    assert "Managed by vibe-setup" not in core
    assert I_START not in core and I_END not in core
    assert "**Repository:**" in core
    noisy: list[str] = []
    added_ws = False
    doubled_blank = False
    for line in core.split("\n"):
        if line.strip():
            if not added_ws:
                noisy.append(line + "   ")  # trailing whitespace normalize must strip
                added_ws = True
            else:
                noisy.append(line)
        else:
            noisy.append(line)
            if not doubled_blank:
                noisy.append("")  # double an existing blank -> normalize squeezes
                doubled_blank = True
    assert added_ws and doubled_blank  # both noise kinds were actually injected
    noisy_body = "\n".join(["", "  "] + noisy + ["   ", ""])  # edge blanks + ws

    agents = target_project / "AGENTS.md"
    agents.write_text(noisy_body, encoding="utf-8")

    result = agents_md.merge(target_project)
    out = agents.read_text(encoding="utf-8")

    assert result.action == "wrapped"
    assert out == _template_text()  # copied full template, markers restored
    assert markers.find_region(out, _INSTRUCTIONS) is not None


# ── case 5: no markers, divergent -> append + warn ──────────────────────────────
def test_merge_appends_divergent_body_with_warning(target_project) -> None:
    agents = target_project / "AGENTS.md"
    original = "# Totally Different\n\nSome unrelated project notes.\n"
    agents.write_text(original, encoding="utf-8")

    result = agents_md.merge(target_project)
    out = agents.read_text(encoding="utf-8")

    assert result.action == "appended"
    assert result.warning is not None
    assert "Some unrelated project notes." in out  # not clobbered
    assert out.startswith(original)  # original preserved at the head
    assert markers.find_region(out, _INSTRUCTIONS) is not None


# ── unmerge ─────────────────────────────────────────────────────────────────────
def test_unmerge_removes_block_and_preserves_prose(target_project) -> None:
    agents = target_project / "AGENTS.md"
    agents.write_text(
        "# Project\n"
        "\n"
        "USER PREAMBLE.\n"
        f"{I_START}\n"
        "managed body\n"
        f"{I_END}\n"
        "\n"
        "USER EPILOGUE.\n",
        encoding="utf-8",
    )
    result = agents_md.unmerge(target_project)
    out = agents.read_text(encoding="utf-8")

    assert result.action == "removed"
    assert I_START not in out
    assert I_END not in out
    assert "managed body" not in out
    assert "USER PREAMBLE." in out
    assert "USER EPILOGUE." in out


def test_unmerge_no_file(target_project) -> None:
    result = agents_md.unmerge(target_project)
    assert result.action == "no-file"


def test_unmerge_absent_block_untouched(target_project) -> None:
    agents = target_project / "AGENTS.md"
    original = "# Project\n\nNo managed block here.\n"
    agents.write_text(original, encoding="utf-8")
    result = agents_md.unmerge(target_project)
    assert result.action == "absent"
    assert agents.read_text(encoding="utf-8") == original


# ── reversed markers refuse ─────────────────────────────────────────────────────
def _reversed_doc() -> str:
    return (
        "keep above\n"
        f"{I_END}\n"
        "trapped middle\n"
        f"{I_START}\n"
        "keep below\n"
    )


def test_merge_reversed_markers_raises(target_project) -> None:
    (target_project / "AGENTS.md").write_text(_reversed_doc(), encoding="utf-8")
    with pytest.raises(markers.MarkerError):
        agents_md.merge(target_project)


def test_unmerge_reversed_markers_raises(target_project) -> None:
    (target_project / "AGENTS.md").write_text(_reversed_doc(), encoding="utf-8")
    with pytest.raises(markers.MarkerError):
        agents_md.unmerge(target_project)


# ── adapter link: three outcomes (+ relink) ─────────────────────────────────────
def test_link_creates_fresh_symlink(target_project) -> None:
    agents_md.merge(target_project)  # ensure AGENTS.md exists as the target
    result = agents_md.link("CLAUDE.md", target_project)
    link_path = target_project / "CLAUDE.md"

    assert result.action == "linked"
    assert result.target == "AGENTS.md"  # resolved from adapters.json
    assert link_path.is_symlink()
    assert os.readlink(link_path) == "AGENTS.md"


def test_link_already_correct_is_noop(target_project) -> None:
    agents_md.merge(target_project)
    agents_md.link("CLAUDE.md", target_project)
    result = agents_md.link("CLAUDE.md", target_project)

    assert result.action == "noop"
    assert (target_project / "CLAUDE.md").is_symlink()
    assert os.readlink(target_project / "CLAUDE.md") == "AGENTS.md"


def test_link_real_file_refused_not_clobbered(target_project) -> None:
    real = target_project / "CLAUDE.md"
    real.write_text("hand-written user instructions\n", encoding="utf-8")

    result = agents_md.link("CLAUDE.md", target_project)

    assert result.action == "refused"
    assert result.warning is not None
    assert not real.is_symlink()  # still a real file
    assert real.read_text(encoding="utf-8") == "hand-written user instructions\n"


def test_link_relinks_wrong_target(target_project) -> None:
    (target_project / "CLAUDE.md").symlink_to("SOMETHING_ELSE.md")
    result = agents_md.link("CLAUDE.md", target_project)

    assert result.action == "relinked"
    assert result.warning is not None
    assert os.readlink(target_project / "CLAUDE.md") == "AGENTS.md"


def test_link_resolves_warp_adapter_target(target_project) -> None:
    result = agents_md.link("WARP.md", target_project)
    assert result.action == "linked"
    assert result.target == "AGENTS.md"
    assert os.readlink(target_project / "WARP.md") == "AGENTS.md"
