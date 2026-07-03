"""Tests for ``vibe uninstall`` / ``vibe update`` (vibe-cli/14).

Exercised through a LOCAL ``typer.Typer()`` with the unit's own ``register`` — the
commands are not wired into ``vibe.app`` until unit 16. The two load-bearing pins port
standing lessons: a *discriminating* surgical uninstall (fails against a blanket ``rm``)
and a live cursor surviving ``update``.
"""

from __future__ import annotations

import typer
from typer.testing import CliRunner

from vibe import cursor, rules
from vibe.commands import lifecycle_cmd
from vibe.provision import agents_md, settings, skills

runner = CliRunner()


def _app() -> typer.Typer:
    app = typer.Typer()
    lifecycle_cmd.register(app)
    return app


def _install(root) -> None:
    """Provision a target with the same leaves ``vibe init`` uses."""
    skills.install(root)
    settings.merge(root)
    agents_md.merge(root)
    cursor.seed(cursor.path_for(root))
    rules.write_rules(root)


def test_register_adds_uninstall_and_update() -> None:
    app = _app()
    names = {c.name for c in app.registered_commands}
    assert {"uninstall", "update"} <= names


def test_discriminating_uninstall_preserves_user_content(tmp_path):
    root = tmp_path / "proj"
    root.mkdir()
    _install(root)

    # User content dropped into each shared surface.
    user_note = root / ".claude" / "skills" / "user_note.md"
    user_note.write_text("keep me", encoding="utf-8")
    user_cfg = root / ".claude" / "user_config.txt"
    user_cfg.write_text("mine", encoding="utf-8")
    spec_file = root / ".spec" / "product.md"
    spec_file.parent.mkdir(parents=True, exist_ok=True)
    spec_file.write_text("# my spec", encoding="utf-8")
    agents = root / "AGENTS.md"
    agents.write_text(agents.read_text(encoding="utf-8") + "\nUSER-PROSE-SENTINEL\n", encoding="utf-8")

    shipped = root / ".claude" / "skills" / "vibe" / "SKILL.md"
    assert shipped.is_file()  # provisioned
    cursor_path = cursor.path_for(root)
    assert cursor_path.is_file()  # seeded

    result = runner.invoke(_app(), ["uninstall", str(root)])
    assert result.exit_code == 0, result.output

    # Shipped file gone; every user artifact survives.
    assert not shipped.exists(), "shipped skill file should be removed"
    assert user_note.read_text(encoding="utf-8") == "keep me"
    assert user_cfg.read_text(encoding="utf-8") == "mine"
    assert spec_file.read_text(encoding="utf-8") == "# my spec"
    assert "USER-PROSE-SENTINEL" in agents.read_text(encoding="utf-8")
    # Cursor preserved without --yes.
    assert cursor_path.is_file(), "cursor must survive uninstall without --yes"


def test_uninstall_yes_removes_cursor(tmp_path):
    root = tmp_path / "proj"
    root.mkdir()
    _install(root)
    cursor_path = cursor.path_for(root)
    assert cursor_path.is_file()

    result = runner.invoke(_app(), ["uninstall", str(root), "--yes"])
    assert result.exit_code == 0, result.output
    assert not cursor_path.exists(), "--yes must remove the cursor"


def test_update_preserves_live_cursor_and_prose(tmp_path):
    root = tmp_path / "proj"
    root.mkdir()
    _install(root)

    # A live cursor mid-feature, plus user prose in AGENTS.md.
    cursor.write(cursor.path_for(root), "feature.impl", "foo")
    agents = root / "AGENTS.md"
    agents.write_text(agents.read_text(encoding="utf-8") + "\nUSER-PROSE-SENTINEL\n", encoding="utf-8")

    result = runner.invoke(_app(), ["update", str(root)])
    assert result.exit_code == 0, result.output

    kept = cursor.read(cursor.path_for(root))
    assert kept is not None
    assert kept.key == "feature.impl"
    assert kept.feature == "foo", "update must preserve the live cursor"
    assert (root / ".claude" / "skills" / "vibe" / "SKILL.md").is_file(), "managed files refreshed"
    assert "USER-PROSE-SENTINEL" in agents.read_text(encoding="utf-8"), "user prose preserved"
