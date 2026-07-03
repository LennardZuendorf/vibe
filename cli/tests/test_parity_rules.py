"""Parity gate: the Python port equals ``regen-active-rules.sh`` byte-for-byte.

The origin self-locates its repo root from ``BASH_SOURCE`` (a ``.spec``/``.git``
marker walk), so running the real script from ``flow/scripts/`` would read the
real ``.spec/lessons.md`` and rewrite the real ``AGENTS.md``. To keep the parity
run isolated and fed by ``sample_lessons``, we copy the script into a throwaway
``<sandbox>/flow/scripts/`` — both branches of its root resolution then land on
the sandbox (the marker walk stops at ``<sandbox>/.spec``, and the fallback
``$SKILL_DIR/..`` is also the sandbox).

``bash_ref`` is requested purely as the availability gate: its body skips the
test when ``bash``/``jq`` are missing (the origin needs no ``jq``, so this only
over-skips harmlessly). The copied script is driven directly so it targets the
sandbox rather than the dev tree.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest

from vibe import markers, rules

_REPO_ROOT = Path(__file__).resolve().parent.parent.parent
_ORIGIN = _REPO_ROOT / "flow" / "scripts" / "regen-active-rules.sh"

_DUMMY_BLOCK = markers.wrap(rules.MARKER_NAME, "DUMMY — must be replaced")


def _extract_region(text: str) -> str:
    region = markers.find_region(text, rules.MARKER_NAME)
    assert region is not None
    lines = text.split("\n")
    return "\n".join(lines[region[0] : region[1] + 1])


def _run_origin(sandbox: Path, sample_lessons: str) -> str:
    """Copy + run the origin against ``sample_lessons``; return the written AGENTS.md."""
    scripts = sandbox / "flow" / "scripts"
    scripts.mkdir(parents=True)
    shutil.copy2(_ORIGIN, scripts / "regen-active-rules.sh")

    (sandbox / ".spec").mkdir()
    (sandbox / ".spec" / "lessons.md").write_text(sample_lessons, encoding="utf-8")

    agents = sandbox / "AGENTS.md"
    agents.write_text(
        f"# Sandbox guide\n\n{_DUMMY_BLOCK}\n\ntail prose\n", encoding="utf-8"
    )

    proc = subprocess.run(
        ["bash", str(scripts / "regen-active-rules.sh")],
        capture_output=True,
        text=True,
        env={**os.environ, "LC_ALL": "C"},
    )
    assert proc.returncode == 0, proc.stderr
    return agents.read_text(encoding="utf-8")


@pytest.mark.usefixtures("bash_ref")
def test_block_matches_bash_origin(tmp_path, sample_lessons):
    if not _ORIGIN.exists():
        pytest.skip(f"bash origin not found: {_ORIGIN}")

    written = _run_origin(tmp_path, sample_lessons)

    # Mislocation guard: if the script had escaped the sandbox, the dummy would
    # survive and this would fail rather than pass silently.
    assert "DUMMY — must be replaced" not in written
    assert "tail prose" in written

    bash_block = _extract_region(written)
    py_block = rules.build_block(tmp_path / ".spec" / "lessons.md")
    assert bash_block == py_block


@pytest.mark.usefixtures("bash_ref")
def test_digest_lines_match_bash_origin(tmp_path, sample_lessons):
    if not _ORIGIN.exists():
        pytest.skip(f"bash origin not found: {_ORIGIN}")

    written = _run_origin(tmp_path, sample_lessons)
    bash_block = _extract_region(written)

    # The digest lives between "### Active Rules\n\n" and the end marker.
    body_lines = bash_block.split("\n")
    end_marker = markers.marker_lines(rules.MARKER_NAME)[1]
    heading_idx = body_lines.index("### Active Rules")
    digest_lines = body_lines[heading_idx + 2 : body_lines.index(end_marker)]

    assert len(digest_lines) == rules.CAP
    assert digest_lines[0].startswith("- \U0001f4cc ")  # pinned first, 📌
    assert "\n".join(digest_lines) == rules.build_digest(
        tmp_path / ".spec" / "lessons.md"
    )
