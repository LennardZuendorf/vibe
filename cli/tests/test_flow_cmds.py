"""Tests for commands/flow_cmds.py — status / next / go (vibe-cli/6).

Exercised through a LOCAL ``typer.Typer()`` with the unit's own ``register`` —
``app.py`` wiring is unit 16, not tested here. Each test drives a throwaway
cursor under a temp ``--root`` (``<root>/.agents/skills/vibe/state.json``) so the
dev tree is never touched. The load-bearing scenarios (R2):

- ``status`` at ``feature.impl`` shows the cursor and its legal ``next``;
- ``go feature.verify`` from ``feature.design`` is refused, names the legal
  option, and leaves the cursor byte-unchanged (legality is checked against
  ``machine.next_of`` *before* any write);
- a legal ``go`` transitions and stamps, carrying the feature forward.
"""

from __future__ import annotations

import typer
from typer.testing import CliRunner

from vibe import cursor, machine
from vibe.commands import flow_cmds

runner = CliRunner()


def _local_app() -> typer.Typer:
    app = typer.Typer()
    flow_cmds.register(app)
    return app


def _seed(root, target, feature=None):
    """Write the cursor directly (the low-level set-state writer, no gate)."""
    return cursor.write(cursor.path_for(root), target, feature)


# --------------------------------------------------------------------------- #
# register(app) contract
# --------------------------------------------------------------------------- #
def test_register_adds_all_three_commands() -> None:
    app = _local_app()
    names = {c.name for c in app.registered_commands}
    assert {"status", "next", "go"} <= names


# --------------------------------------------------------------------------- #
# status
# --------------------------------------------------------------------------- #
def test_status_shows_cursor_and_legal_next(tmp_path) -> None:
    _seed(tmp_path, "feature.impl", "demo")

    result = runner.invoke(_local_app(), ["status", "--root", str(tmp_path)])

    assert result.exit_code == 0
    assert "feature.impl" in result.output
    assert "demo" in result.output
    # legal next of feature.impl is exactly feature.verify
    assert "feature.verify" in result.output


def test_status_defaults_to_idle_without_cursor(tmp_path) -> None:
    result = runner.invoke(_local_app(), ["status", "--root", str(tmp_path)])

    assert result.exit_code == 0
    assert "idle" in result.output
    # idle's legal next set is advertised
    for target in machine.next_of("idle"):
        assert target in result.output


# --------------------------------------------------------------------------- #
# next
# --------------------------------------------------------------------------- #
def test_next_lists_legal_transitions(tmp_path) -> None:
    _seed(tmp_path, "feature.design")

    result = runner.invoke(_local_app(), ["next", "--root", str(tmp_path)])

    assert result.exit_code == 0
    assert "feature.plan" in result.output


def test_next_single_target_lists_it(tmp_path) -> None:
    _seed(tmp_path, "quick.verify")  # next: idle

    result = runner.invoke(_local_app(), ["next", "--root", str(tmp_path)])

    assert result.exit_code == 0
    assert "idle" in result.output


# --------------------------------------------------------------------------- #
# go — refusal (R2-S2)
# --------------------------------------------------------------------------- #
def test_go_illegal_transition_refused_cursor_unchanged(tmp_path) -> None:
    before = _seed(tmp_path, "feature.design")
    path = cursor.path_for(tmp_path)
    raw_before = path.read_text(encoding="utf-8")

    result = runner.invoke(
        _local_app(), ["go", "feature.verify", "--root", str(tmp_path)]
    )

    assert result.exit_code != 0
    # the legal option is named
    assert "feature.plan" in result.output
    # cursor is byte-unchanged
    assert path.read_text(encoding="utf-8") == raw_before
    after = cursor.read(path)
    assert after.key == before.key == "feature.design"


def test_go_unknown_target_refused(tmp_path) -> None:
    _seed(tmp_path, "feature.design")
    path = cursor.path_for(tmp_path)
    raw_before = path.read_text(encoding="utf-8")

    result = runner.invoke(
        _local_app(), ["go", "not.a.state", "--root", str(tmp_path)]
    )

    assert result.exit_code != 0
    assert path.read_text(encoding="utf-8") == raw_before


# --------------------------------------------------------------------------- #
# go — legal transitions (R2-S3)
# --------------------------------------------------------------------------- #
def test_go_legal_transition_stamps_and_preserves_feature(tmp_path) -> None:
    _seed(tmp_path, "feature.impl", "demo")
    path = cursor.path_for(tmp_path)

    result = runner.invoke(
        _local_app(), ["go", "feature.verify", "--root", str(tmp_path)]
    )

    assert result.exit_code == 0
    cur = cursor.read(path)
    assert cur.key == "feature.verify"
    assert cur.feature == "demo"  # carried forward
    assert cur.updated  # stamped


def test_go_from_idle_into_flow(tmp_path) -> None:
    _seed(tmp_path, "idle")
    path = cursor.path_for(tmp_path)

    result = runner.invoke(
        _local_app(),
        ["go", "feature.design", "--feature", "auth", "--root", str(tmp_path)],
    )

    assert result.exit_code == 0
    cur = cursor.read(path)
    assert cur.key == "feature.design"
    assert cur.feature == "auth"


def test_go_creates_cursor_when_absent(tmp_path) -> None:
    # No cursor seeded: current state resolves to idle, a legal jump is written.
    path = cursor.path_for(tmp_path)
    assert not path.exists()

    result = runner.invoke(
        _local_app(), ["go", "quick.triage", "--root", str(tmp_path)]
    )

    assert result.exit_code == 0
    assert path.exists()
    assert cursor.read(path).key == "quick.triage"
