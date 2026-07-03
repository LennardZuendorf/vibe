"""Regression pins for the feature.verify fixes (vibe-cli).

Each test locks a behavior an adversarial review found missing from the first
implementation pass: graceful-degrade on a malformed ``settings.json`` (init +
uninstall), the ``vibe orders --state`` cursor-feature interpolation, the
``go feature.*`` no-feature nudge, and the ``init`` next-step guidance.
"""

from __future__ import annotations

import typer
from typer.testing import CliRunner

from vibe import cursor
from vibe.commands import flow_cmds, init_cmd, lifecycle_cmd, spec_cmd
from vibe.provision import plugins, skills

runner = CliRunner()


def _app(module) -> typer.Typer:
    app = typer.Typer()
    module.register(app)
    return app


def _no_claude(monkeypatch) -> None:
    """Force plugins to the manual path so init never shells out to `claude`."""
    monkeypatch.setattr(plugins, "claude_path", lambda: None)


def test_init_degrades_on_malformed_settings(tmp_path, monkeypatch):
    _no_claude(monkeypatch)
    root = tmp_path / "proj"
    (root / ".claude").mkdir(parents=True)
    (root / ".claude" / "settings.json").write_text("{ bad json here\n", encoding="utf-8")

    result = runner.invoke(_app(init_cmd), [str(root), "--yes"])
    assert result.exit_code == 0, result.output
    # The malformed file is left INTACT (never overwritten — user data behind the
    # syntax error is recoverable) ...
    assert "bad json here" in (root / ".claude" / "settings.json").read_text(encoding="utf-8")
    # ... and init did not partial-crash: AGENTS.md + cursor were still provisioned.
    assert (root / "AGENTS.md").exists()
    assert cursor.path_for(root).exists()


def test_uninstall_degrades_on_malformed_settings(tmp_path):
    root = tmp_path / "proj"
    root.mkdir()
    skills.install(root)
    (root / ".claude" / "settings.json").write_text("{ nope\n", encoding="utf-8")

    result = runner.invoke(_app(lifecycle_cmd), ["uninstall", str(root)])
    assert result.exit_code == 0, result.output
    assert "nope" in (root / ".claude" / "settings.json").read_text(encoding="utf-8")
    # Skills were still surgically removed despite the bad settings.
    assert not (root / ".claude" / "skills" / "vibe" / "SKILL.md").exists()


def test_orders_state_uses_cursor_feature(tmp_path):
    root = tmp_path / "proj"
    root.mkdir()
    cursor.write(cursor.path_for(root), "feature.impl", "payments")

    result = runner.invoke(
        _app(spec_cmd), ["orders", "--state", "feature.impl", "--root", str(root)]
    )
    assert result.exit_code == 0, result.output
    assert "<feature>" not in result.output, "the literal placeholder must be interpolated"
    assert "payments" in result.output


def test_go_feature_without_feature_warns(tmp_path):
    root = tmp_path / "proj"
    root.mkdir()
    cursor.seed(cursor.path_for(root))  # idle → feature.design is legal

    result = runner.invoke(_app(flow_cmds), ["go", "feature.design", "--root", str(root)])
    assert result.exit_code == 0, result.output
    assert "no feature set" in result.output


def test_init_prints_next_step(tmp_path, monkeypatch):
    _no_claude(monkeypatch)
    root = tmp_path / "p2"
    root.mkdir()

    result = runner.invoke(_app(init_cmd), [str(root), "--yes"])
    assert result.exit_code == 0, result.output
    assert "next" in result.output.lower()
    assert "vibe go" in result.output
