"""Unit tests for orders resolution (vibe-cli/5).

Deterministic, bash-free coverage of the three-tier resolution and the
``<feature>`` interpolation. The bundled ``_assets`` copy is the read source;
expected block content is recomputed here with an independent index-based
extractor so the assertions cross-check ``orders.py``'s line-iteration extractor
rather than restate it.
"""

from __future__ import annotations

import sys
from pathlib import Path

from vibe import cursor, machine, orders

_ASSETS = Path(orders.__file__).resolve().parent / "_assets"
_VIBE_SKILL = _ASSETS / "skills" / "vibe" / "SKILL.md"


def _block(state: str) -> str:
    """Independent extractor: substring-slice the block from the bundled SKILL.md."""
    text = _VIBE_SKILL.read_text(encoding="utf-8")
    opener = f"<!-- vibe:orders:{state} -->\n"
    start = text.index(opener) + len(opener)
    end = text.index("\n<!-- /vibe:orders -->", start)
    return text[start:end]


def test_skill_block_extracted_and_interpolated():
    expected = _block("feature.impl").replace("<feature>", "myfeat")
    result = orders.resolve("feature.impl", "myfeat")
    assert result == expected
    assert "myfeat" in result
    assert "<feature>" not in result


def test_no_feature_leaves_placeholder():
    result = orders.resolve("feature.impl", None)
    assert result == _block("feature.impl")
    assert "<feature>" in result


def test_multiple_feature_occurrences_all_interpolated():
    result = orders.resolve("feature.plan", "acme")
    assert "<feature>" not in result
    assert result.count("acme") == _block("feature.plan").count("<feature>") >= 2


def test_skill_block_without_placeholder_unchanged_by_feature():
    with_feature = orders.resolve("feature.verify", "myfeat")
    without = orders.resolve("feature.verify", None)
    assert with_feature == without == _block("feature.verify")


def test_setup_and_amend_blocks_extracted():
    assert orders.resolve("setup.detect") == _block("setup.detect")
    assert orders.resolve("amend") == _block("amend")


def test_machine_inject_fallback_for_idle():
    m = machine.default()
    assert orders.resolve("idle") == m.inject_of("idle")


def test_generic_fallback_for_unknown_state():
    assert orders.resolve("bogus.state") == orders.GENERIC_FALLBACK
    assert orders.resolve("totally-unknown") == orders.GENERIC_FALLBACK


def test_skills_dir_override_reads_target_copy(tmp_path):
    target_skill = tmp_path / "vibe" / "SKILL.md"
    target_skill.parent.mkdir(parents=True)
    target_skill.write_text(
        "<!-- vibe:orders:feature.impl -->\n"
        "OVERRIDDEN orders for <feature>\n"
        "<!-- /vibe:orders -->\n",
        encoding="utf-8",
    )
    result = orders.resolve("feature.impl", "zed", skills_dir=tmp_path)
    assert result == "OVERRIDDEN orders for zed"


def test_override_missing_block_falls_through_to_generic(tmp_path):
    (tmp_path / "vibe").mkdir()
    (tmp_path / "vibe" / "SKILL.md").write_text("no orders here\n", encoding="utf-8")
    assert orders.resolve("feature.impl", "z", skills_dir=tmp_path) == orders.GENERIC_FALLBACK


def test_resolve_current_reads_cursor(tmp_path):
    root = tmp_path / "proj"
    root.mkdir()
    cursor.write(cursor.path_for(root), "feature.impl", "widget")
    assert orders.resolve_current(root) == orders.resolve("feature.impl", "widget")


def test_resolve_current_missing_cursor_is_idle(tmp_path):
    root = tmp_path / "empty"
    root.mkdir()
    assert orders.resolve_current(root) == orders.resolve("idle")


def test_import_is_stdlib_only():
    # Must run in a FRESH interpreter: sys.modules is process-global, so a
    # sibling command test importing typer earlier in the same suite would
    # otherwise contaminate an in-process check.
    import subprocess

    code = (
        "import vibe.orders, sys; "
        "bad=[m for m in ('typer','rich','pydantic') if m in sys.modules]; "
        "assert not bad, bad"
    )
    proc = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True
    )
    assert proc.returncode == 0, proc.stderr
