"""Unit vibe-cli/3 — read + atomic-write of the flow cursor (state.json).

Pins the ported ``set-state.sh`` semantics: feature carry-forward across a
``feature.*`` move, ``idle`` clearing the feature to ``null``, an explicit
feature overriding the carry, ``amend`` (a modifier) and unknown states
rejected, an ISO-stamped ``updated``, and a crash-safe atomic write that leaves
no partial file behind. ``current_state`` mirrors ``detect-context.sh`` so the
unit-4 parity suite can resolve the cursor through this layer. Importing
``vibe.cursor`` must pull in no ``typer``/``rich``/``pydantic``.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys

import pytest

from vibe import cursor
from vibe.errors import VibeError

_ISO = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")


def _state_file(tmp_path):
    return tmp_path / "state.json"


def test_write_returns_cursor_and_writes_idle(tmp_path):
    path = _state_file(tmp_path)
    result = cursor.write(path, "idle")
    assert result.flow == "idle"
    assert result.phase == "idle"
    assert result.feature is None
    assert result.key == "idle"
    on_disk = json.loads(path.read_text())
    assert on_disk == {
        "flow": "idle",
        "phase": "idle",
        "feature": None,
        "updated": result.updated,
    }


def test_write_splits_compound_key(tmp_path):
    path = _state_file(tmp_path)
    result = cursor.write(path, "feature.impl", "auth")
    assert result.flow == "feature"
    assert result.phase == "impl"
    assert result.key == "feature.impl"
    assert result.feature == "auth"


def test_feature_carries_forward(tmp_path):
    path = _state_file(tmp_path)
    cursor.write(path, "feature.impl", "auth-tokens")
    later = cursor.write(path, "feature.verify")
    assert later.feature == "auth-tokens"
    assert later.key == "feature.verify"
    assert json.loads(path.read_text())["feature"] == "auth-tokens"


def test_idle_clears_feature(tmp_path):
    path = _state_file(tmp_path)
    cursor.write(path, "feature.impl", "auth-tokens")
    cleared = cursor.write(path, "idle")
    assert cleared.feature is None
    assert json.loads(path.read_text())["feature"] is None


def test_explicit_feature_overrides_carry(tmp_path):
    path = _state_file(tmp_path)
    cursor.write(path, "feature.design", "old")
    switched = cursor.write(path, "feature.impl", "new")
    assert switched.feature == "new"


def test_no_feature_stays_null_without_carry(tmp_path):
    path = _state_file(tmp_path)
    result = cursor.write(path, "feature.design")
    assert result.feature is None


def test_amend_target_rejected(tmp_path):
    path = _state_file(tmp_path)
    cursor.write(path, "feature.impl", "keep")
    before = path.read_text()
    with pytest.raises(VibeError) as exc:
        cursor.write(path, "amend")
    assert "modifier" in str(exc.value)
    assert path.read_text() == before


def test_unknown_target_rejected(tmp_path):
    path = _state_file(tmp_path)
    with pytest.raises(VibeError) as exc:
        cursor.write(path, "feature.bogus")
    msg = str(exc.value)
    assert "not a known state" in msg
    assert "feature.impl" in msg
    assert not path.exists()


def test_updated_is_iso_stamped(tmp_path):
    path = _state_file(tmp_path)
    result = cursor.write(path, "feature.impl", "auth")
    assert _ISO.match(result.updated)
    assert _ISO.match(json.loads(path.read_text())["updated"])


def test_read_missing_returns_none(tmp_path):
    assert cursor.read(_state_file(tmp_path)) is None


def test_read_roundtrips_written_cursor(tmp_path):
    path = _state_file(tmp_path)
    written = cursor.write(path, "feature.impl", "auth")
    got = cursor.read(path)
    assert got == written


def test_current_state_defaults_idle_when_absent(tmp_path):
    assert cursor.current_state(_state_file(tmp_path)) == "idle"


def test_current_state_reflects_cursor(tmp_path):
    path = _state_file(tmp_path)
    cursor.write(path, "feature.impl", "auth")
    assert cursor.current_state(path) == "feature.impl"


def test_current_state_degrades_on_malformed(tmp_path):
    path = _state_file(tmp_path)
    path.write_text("{ not json ]")
    assert cursor.current_state(path) == "idle"


def test_current_state_coalesces_null_flow(tmp_path):
    # detect-context uses `.flow // "idle"` — a JSON null coalesces to idle.
    path = _state_file(tmp_path)
    path.write_text(json.dumps({"flow": None, "phase": None, "feature": None}))
    assert cursor.current_state(path) == "idle"


def test_write_tolerates_malformed_existing_cursor(tmp_path):
    path = _state_file(tmp_path)
    path.write_text("{ not json ]")
    result = cursor.write(path, "feature.verify")
    assert result.feature is None
    assert result.key == "feature.verify"


def test_relpath_matches_the_hard_blocked_path():
    assert cursor.RELPATH.as_posix() == ".agents/skills/vibe/state.json"
    root = "/tmp/proj"
    assert cursor.path_for(root).as_posix() == "/tmp/proj/.agents/skills/vibe/state.json"


def test_atomic_write_no_partial_on_failure(tmp_path, monkeypatch):
    path = _state_file(tmp_path)
    cursor.write(path, "feature.impl", "safe")
    before = path.read_text()

    def boom(*_a, **_k):
        raise OSError("simulated replace failure")

    monkeypatch.setattr(os, "replace", boom)
    with pytest.raises(OSError):
        cursor.write(path, "idle")

    assert path.read_text() == before
    leftovers = list(tmp_path.glob("state.json.*"))
    assert leftovers == []


def test_atomic_write_absent_stays_absent_on_failure(tmp_path, monkeypatch):
    path = _state_file(tmp_path)

    def boom(*_a, **_k):
        raise OSError("simulated replace failure")

    monkeypatch.setattr(os, "replace", boom)
    with pytest.raises(OSError):
        cursor.write(path, "idle")

    assert not path.exists()
    assert list(tmp_path.glob("state.json.*")) == []


def test_write_creates_missing_parent_dirs(tmp_path):
    path = tmp_path / "nested" / "deep" / "state.json"
    cursor.write(path, "idle")
    assert path.exists()


def test_seed_writes_idle_when_absent(tmp_path):
    path = _state_file(tmp_path)
    seeded = cursor.seed(path)
    assert seeded.key == "idle"
    assert path.exists()


def test_seed_preserves_existing_cursor(tmp_path):
    path = _state_file(tmp_path)
    cursor.write(path, "feature.impl", "live")
    original = path.read_text()
    kept = cursor.seed(path)
    assert kept.key == "feature.impl"
    assert kept.feature == "live"
    assert path.read_text() == original


def test_import_pulls_no_typer_rich_pydantic():
    code = (
        "import sys, vibe.cursor\n"
        "leaked = {'typer', 'rich', 'pydantic'} & set(sys.modules)\n"
        "assert not leaked, leaked\n"
    )
    result = subprocess.run(
        [sys.executable, "-c", code],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
