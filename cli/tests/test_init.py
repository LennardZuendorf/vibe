"""Tests for commands/init_cmd.py — ``vibe init [PATH]`` (vibe-cli/11).

``init`` is ORCHESTRATION ONLY: every step delegates to a tested leaf
(``provision.skills`` / ``provision.settings`` / ``provision.agents_md`` /
``rules`` / ``provision.plugins`` / ``cursor``). These tests exercise the command
through a LOCAL ``typer.Typer()`` with the unit's own ``register`` — ``app.py``
wiring is unit 16, not tested here — against the ``target_project`` sandbox so the
dev tree is never touched.

``plugins.claude_path`` is forced absent in every test that runs the real plugin
step so a dev machine with ``claude`` on PATH never triggers a real marketplace
install; the spec-only test spies ``plugins.install_deps`` to prove it is not even
offered.

Load-bearing scenarios (R1, R6):

- fresh ``--yes`` fully provisioned: both skill trees, three settings.json hooks,
  an ``AGENTS.md`` block, a seeded + gitignored cursor, and a printed summary;
- **ephemeral** invocation (binaries not on PATH) warns — never a silent no-op;
- ``--only spec`` installs the spec tree ONLY (vibe skill absent, no hooks, no
  cursor, no gitignore line, no AGENTS.md, plugins never offered);
- ``--dry-run`` performs ZERO filesystem writes yet still prints the summary;
- the written settings.json command strings equal ``hook.py``'s own subcommands.
"""

from __future__ import annotations

import json
from pathlib import Path

import typer
from typer.testing import CliRunner

from vibe import cursor, hook
from vibe.commands import init_cmd
from vibe.provision import plugins, settings

runner = CliRunner()


def _local_app() -> typer.Typer:
    # A callback forces group behaviour so ``init`` is a subcommand (mirroring
    # app.py's wiring); without it Typer unwraps a lone command to the app root.
    app = typer.Typer()

    @app.callback()
    def _root() -> None:  # pragma: no cover - wiring shim
        ...

    init_cmd.register(app)
    return app


def _force_claude_absent(monkeypatch) -> None:
    """Force the plugin step onto its manual (no-subprocess) degrade path."""
    monkeypatch.setattr(plugins, "claude_path", lambda: None)


def _settings_commands(target: Path) -> set[str]:
    data = json.loads((target / ".claude" / "settings.json").read_text(encoding="utf-8"))
    found: set[str] = set()
    for groups in data["hooks"].values():
        for group in groups:
            for entry in group["hooks"]:
                found.add(entry["command"])
    return found


# --------------------------------------------------------------------------- #
# register(app) contract
# --------------------------------------------------------------------------- #
def test_register_adds_init_command() -> None:
    app = _local_app()
    assert any(c.name == "init" for c in app.registered_commands)


# --------------------------------------------------------------------------- #
# fresh --yes fully provisioned (R1-S1)
# --------------------------------------------------------------------------- #
def test_fresh_yes_fully_provisioned(target_project: Path, monkeypatch) -> None:
    _force_claude_absent(monkeypatch)

    result = runner.invoke(_local_app(), ["init", str(target_project), "--yes"])

    assert result.exit_code == 0, result.output

    # both skill trees under .claude/skills/
    assert (target_project / ".claude" / "skills" / "spec" / "SKILL.md").is_file()
    assert (target_project / ".claude" / "skills" / "vibe" / "SKILL.md").is_file()

    # three settings.json hook events
    data = json.loads(
        (target_project / ".claude" / "settings.json").read_text(encoding="utf-8")
    )
    assert set(data["hooks"]) == {"UserPromptSubmit", "PreToolUse", "Stop"}

    # AGENTS.md managed block (and the regenerated active-rules block)
    agents = (target_project / "AGENTS.md").read_text(encoding="utf-8")
    assert "<!-- vibe:instructions:start -->" in agents
    assert "<!-- vibe:active-rules:start -->" in agents

    # cursor seeded (idle) at the canonical relpath
    cursor_file = cursor.path_for(target_project)
    assert cursor_file == target_project / ".agents" / "skills" / "vibe" / "state.json"
    assert cursor_file.is_file()
    assert json.loads(cursor_file.read_text(encoding="utf-8"))["flow"] == "idle"

    # cursor gitignored
    gitignore = (target_project / ".gitignore").read_text(encoding="utf-8")
    assert ".agents/skills/vibe/state.json" in gitignore

    # a summary was printed
    assert "init" in result.output.lower()


# --------------------------------------------------------------------------- #
# ephemeral binaries warn, not silent (R6-S2)
# --------------------------------------------------------------------------- #
def test_ephemeral_binaries_warn_not_silent(target_project: Path, monkeypatch) -> None:
    _force_claude_absent(monkeypatch)
    monkeypatch.setattr(init_cmd, "_binary_on_path", lambda name: None)

    result = runner.invoke(_local_app(), ["init", str(target_project), "--yes"])

    assert result.exit_code == 0, result.output
    # the warning names the ephemeral trap and PATH — not a silent no-op
    lowered = result.output.lower()
    assert "ephemeral" in lowered
    assert "path" in lowered
    # ...and provisioning still happened (the anti-no-op guarantee)
    assert (target_project / ".claude" / "settings.json").is_file()
    assert (target_project / ".claude" / "skills" / "spec").is_dir()


def test_full_install_no_warning_when_binaries_resolve(
    target_project: Path, monkeypatch
) -> None:
    _force_claude_absent(monkeypatch)
    monkeypatch.setattr(
        init_cmd, "_binary_on_path", lambda name: f"/usr/local/bin/{name}"
    )

    result = runner.invoke(_local_app(), ["init", str(target_project), "--yes"])

    assert result.exit_code == 0, result.output
    assert "ephemeral" not in result.output.lower()


# --------------------------------------------------------------------------- #
# --only spec: spec tree ONLY (discriminating)
# --------------------------------------------------------------------------- #
def test_only_spec_installs_spec_tree_only(target_project: Path, monkeypatch) -> None:
    offered: list[dict] = []
    monkeypatch.setattr(
        plugins, "install_deps", lambda **kwargs: offered.append(kwargs)
    )

    result = runner.invoke(_local_app(), ["init", str(target_project), "--only", "spec"])

    assert result.exit_code == 0, result.output

    # spec skill present...
    assert (target_project / ".claude" / "skills" / "spec" / "SKILL.md").is_file()
    # ...vibe skill ABSENT (this is what discriminates spec-only from full)
    assert not (target_project / ".claude" / "skills" / "vibe").exists()
    # no flow wiring at all
    assert not (target_project / ".claude" / "settings.json").exists()
    assert not (target_project / ".agents").exists()  # no cursor
    assert not (target_project / ".gitignore").exists()
    assert not (target_project / "AGENTS.md").exists()
    # plugins never even offered
    assert offered == []


def test_unknown_only_value_errors(target_project: Path) -> None:
    result = runner.invoke(_local_app(), ["init", str(target_project), "--only", "flow"])
    assert result.exit_code != 0
    assert not (target_project / ".claude").exists()


# --------------------------------------------------------------------------- #
# --dry-run: ZERO writes, summary still printed
# --------------------------------------------------------------------------- #
def test_dry_run_writes_nothing_but_prints_summary(
    target_project: Path, monkeypatch
) -> None:
    _force_claude_absent(monkeypatch)

    result = runner.invoke(
        _local_app(), ["init", str(target_project), "--yes", "--dry-run"]
    )

    assert result.exit_code == 0, result.output
    # ZERO filesystem writes
    assert not (target_project / ".claude").exists()
    assert not (target_project / "AGENTS.md").exists()
    assert not (target_project / ".gitignore").exists()
    assert not (target_project / ".agents").exists()
    # ...but the summary still printed
    assert "skills" in result.output
    assert "init" in result.output.lower()


def test_dry_run_leaves_no_target_files(target_project: Path, monkeypatch) -> None:
    _force_claude_absent(monkeypatch)
    before = {p for p in target_project.rglob("*")}

    runner.invoke(_local_app(), ["init", str(target_project), "--yes", "--dry-run"])

    after = {p for p in target_project.rglob("*")}
    assert before == after


# --------------------------------------------------------------------------- #
# hook-contract proxy (R1-S2): written commands == hook.py subcommands
# --------------------------------------------------------------------------- #
def test_written_settings_commands_equal_hook_subcommands(
    target_project: Path, monkeypatch
) -> None:
    _force_claude_absent(monkeypatch)

    runner.invoke(_local_app(), ["init", str(target_project), "--yes"])

    written = _settings_commands(target_project)
    # tie to hook.py itself — not a restated literal — so verdicts cannot drift
    expected = {f"vibe-hook {sub}" for sub in hook._SUBCOMMANDS}
    assert written == expected
    # and the leaf that wrote them agrees on the same command set
    assert written == set(settings.commands().values())


# --------------------------------------------------------------------------- #
# idempotence: a second init is a safe no-op-ish re-provision
# --------------------------------------------------------------------------- #
def test_reinstall_is_idempotent(target_project: Path, monkeypatch) -> None:
    _force_claude_absent(monkeypatch)

    runner.invoke(_local_app(), ["init", str(target_project), "--yes"])
    settings_before = (
        target_project / ".claude" / "settings.json"
    ).read_text(encoding="utf-8")
    agents_before = (target_project / "AGENTS.md").read_text(encoding="utf-8")

    result = runner.invoke(_local_app(), ["init", str(target_project), "--yes"])

    assert result.exit_code == 0, result.output
    assert (
        target_project / ".claude" / "settings.json"
    ).read_text(encoding="utf-8") == settings_before
    assert (target_project / "AGENTS.md").read_text(encoding="utf-8") == agents_before


def test_reinstall_preserves_live_cursor(target_project: Path, monkeypatch) -> None:
    _force_claude_absent(monkeypatch)

    runner.invoke(_local_app(), ["init", str(target_project), "--yes"])
    # advance the cursor to a live feature.impl state
    cursor_file = cursor.path_for(target_project)
    cursor.write(cursor_file, "feature.impl", "demo")

    runner.invoke(_local_app(), ["init", str(target_project), "--yes"])

    cur = cursor.read(cursor_file)
    assert cur.key == "feature.impl"
    assert cur.feature == "demo"
