"""Unit vibe-cli/2 — stdlib load + validate of the bundled state-machine.json.

Pins: all 15 states load; compound-key split/join match the bash originals
(`set-state.sh` / `validate-state.sh`); `amend` is present yet not a legal
cursor target; a missing/malformed/incomplete file raises the typed error; and
importing ``vibe.machine`` pulls in no ``typer``/``rich``/``pydantic``.
"""

from __future__ import annotations

import json
import subprocess
import sys

import pytest

from vibe import machine
from vibe.errors import VibeError


def test_default_loads_all_fifteen_states(machine_data):
    m = machine.default()
    assert len(m.states) == 15
    assert m.states == machine_data["states"]
    assert m.version == machine_data["version"]
    assert m.initial == machine_data["initial"]


def test_top_level_vocabularies_present(machine_data):
    m = machine.default()
    assert m.flows == machine_data["flows"]
    assert m.phases == machine_data["phases"]
    assert m.modifiers == machine_data["modifiers"]
    assert "amend" in m.modifiers


def test_state_lookup_returns_dicts():
    assert machine.state("idle")["caveman"] == "lite"
    assert machine.state("feature.impl")["caveman"] == "full"
    # amend is present in the machine even though it is not a cursor target.
    assert machine.state("amend")["skill"] == "vibe"


def test_state_unknown_key_rejected():
    with pytest.raises(VibeError):
        machine.state("feature.bogus")


def test_next_inject_writes_caveman_helpers():
    assert machine.next_of("idle") == [
        "setup.detect",
        "strategy.brainstorm",
        "feature.design",
        "quick.triage",
    ]
    assert machine.writes_of("feature.impl") == ["src/**", "tests/**"]
    assert machine.caveman_of("feature.impl") == "full"
    assert machine.caveman_of("idle") == "lite"
    # idle keeps an inline inject; skill-owning states carry inject:null (D12).
    assert isinstance(machine.inject_of("idle"), str)
    assert machine.inject_of("feature.impl") is None


def test_list_helpers_return_copies():
    a = machine.next_of("idle")
    a.append("tampered")
    assert "tampered" not in machine.next_of("idle")
    b = machine.writes_of("feature.impl")
    b.append("tampered")
    assert "tampered" not in machine.writes_of("feature.impl")


def test_helpers_reject_unknown_key():
    for fn in (machine.next_of, machine.writes_of, machine.caveman_of, machine.inject_of):
        with pytest.raises(VibeError):
            fn("nope.nope")


def test_is_valid_target():
    assert machine.is_valid_target("idle") is True
    assert machine.is_valid_target("feature.impl") is True
    # amend is present but is a modifier, never a stored cursor target.
    assert machine.is_valid_target("amend") is False
    assert machine.is_valid_target("bogus") is False
    assert machine.is_valid_target("feature.bogus") is False


def test_valid_targets_excludes_amend():
    targets = machine.valid_targets()
    assert "idle" in targets
    assert "feature.impl" in targets
    assert "amend" not in targets
    assert len(targets) == 14


def test_split_key_matches_bash():
    # set-state.sh: single-token states map flow==phase; else split on first dot.
    assert machine.split_key("idle") == ("idle", "idle")
    assert machine.split_key("feature.impl") == ("feature", "impl")
    assert machine.split_key("strategy.brainstorm") == ("strategy", "brainstorm")


def test_join_key_matches_bash():
    # validate-state.sh: KEY=FLOW when FLOW==PHASE else FLOW.PHASE.
    assert machine.join_key("idle", "idle") == "idle"
    assert machine.join_key("feature", "impl") == "feature.impl"
    assert machine.split_key(machine.join_key("quick", "fix")) == ("quick", "fix")


def test_load_missing_file_raises(tmp_path):
    with pytest.raises(VibeError):
        machine.load(tmp_path / "does-not-exist.json")


def test_load_malformed_json_raises(tmp_path):
    bad = tmp_path / "machine.json"
    bad.write_text("{ not json ]")
    with pytest.raises(VibeError):
        machine.load(bad)


def test_load_missing_states_key_raises(tmp_path):
    incomplete = tmp_path / "machine.json"
    incomplete.write_text(
        json.dumps({"version": "1", "flows": [], "phases": [], "modifiers": []})
    )
    with pytest.raises(VibeError):
        machine.load(incomplete)


def test_load_empty_states_raises(tmp_path):
    empty = tmp_path / "machine.json"
    empty.write_text(
        json.dumps(
            {
                "version": "1",
                "initial": "idle",
                "flows": [],
                "phases": [],
                "modifiers": [],
                "states": {},
            }
        )
    )
    with pytest.raises(VibeError):
        machine.load(empty)


def test_load_explicit_path_roundtrips(machine_data, tmp_path):
    copy = tmp_path / "machine.json"
    copy.write_text(json.dumps(machine_data))
    m = machine.load(copy)
    assert len(m.states) == 15
    assert m.states == machine_data["states"]


def test_import_pulls_no_typer_rich_pydantic():
    code = (
        "import sys, vibe.machine\n"
        "leaked = {'typer', 'rich', 'pydantic'} & set(sys.modules)\n"
        "assert not leaked, leaked\n"
    )
    result = subprocess.run(
        [sys.executable, "-c", code],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
