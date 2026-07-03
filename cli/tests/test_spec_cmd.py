"""Tests for ``vibe spec`` / ``vibe check`` / ``vibe orders`` (vibe-cli/15).

Each command is exercised through a LOCAL ``typer.Typer()`` with the module's
own ``register(app)`` — never the app.py wiring (that is unit 16).
"""

from __future__ import annotations

import shutil
import subprocess

import pytest
import typer
from typer.testing import CliRunner

from vibe.commands import spec_cmd

runner = CliRunner()


def _app() -> typer.Typer:
    app = typer.Typer()
    spec_cmd.register(app)
    return app


def _valid_spec(root) -> None:
    spec = root / ".spec"
    spec.mkdir(parents=True, exist_ok=True)
    (spec / "product.md").write_text(
        "---\n"
        "type: product\n"
        "updated: 2026-07-03\n"
        "---\n"
        "# Product\n"
        "\n"
        "A minimal but valid spec so validate.sh exits 0.\n",
        encoding="utf-8",
    )


@pytest.mark.skipif(shutil.which("bash") is None, reason="bash required for spec scripts")
def test_spec_validate_matches_direct_and_succeeds(tmp_path):
    project = tmp_path / "proj"
    project.mkdir()
    _valid_spec(project)

    script = spec_cmd._resolve_script(project.resolve(), "validate.sh")
    direct = subprocess.run(
        ["bash", str(script)],
        cwd=str(project.resolve()),
        capture_output=True,
        text=True,
    )

    result = runner.invoke(_app(), ["spec", "validate", "--root", str(project)])

    assert result.exit_code == 0
    assert direct.returncode == 0
    assert result.stdout == direct.stdout


@pytest.mark.skipif(shutil.which("bash") is None, reason="bash required for spec scripts")
def test_spec_validate_falls_back_to_bundled_script(tmp_path):
    project = tmp_path / "proj"
    project.mkdir()
    _valid_spec(project)

    resolved = spec_cmd._resolve_script(project.resolve(), "validate.sh")
    assert resolved == spec_cmd._BUNDLED_SPEC / "scripts" / "validate.sh"
    assert resolved.is_file()


@pytest.mark.skipif(shutil.which("bash") is None, reason="bash required for spec scripts")
def test_spec_setup_scaffolds_spec_dir(tmp_path):
    project = tmp_path / "proj"
    project.mkdir()

    result = runner.invoke(_app(), ["spec", "setup", "--root", str(project)])

    assert result.exit_code == 0
    for entrypoint in ("product.md", "tech.md", "design.md", "plan.md", "lessons.md"):
        assert (project / ".spec" / entrypoint).is_file()


def test_check_lessons_blocked_at_idle(tmp_path):
    project = tmp_path / "proj"
    project.mkdir()

    result = runner.invoke(
        _app(), ["check", ".spec/lessons.md", "--root", str(project)]
    )

    assert result.exit_code == 0
    assert "block:" in result.stdout
    assert ".spec/lessons.md" in result.stdout
    assert "*.compound" in result.stdout


def test_check_state_override_allows_lessons_in_compound(tmp_path):
    project = tmp_path / "proj"
    project.mkdir()

    result = runner.invoke(
        _app(),
        ["check", ".spec/lessons.md", "--state", "feature.compound", "--root", str(project)],
    )

    assert result.exit_code == 0
    assert result.stdout.strip() == "allow"


def test_orders_prints_current_idle_orders(tmp_path):
    project = tmp_path / "proj"
    project.mkdir()

    result = runner.invoke(_app(), ["orders", "--root", str(project)])

    assert result.exit_code == 0
    assert "state=idle" in result.stdout


def test_orders_state_override(tmp_path):
    project = tmp_path / "proj"
    project.mkdir()

    result = runner.invoke(_app(), ["orders", "--state", "idle", "--root", str(project)])

    assert result.exit_code == 0
    assert "state=idle" in result.stdout
