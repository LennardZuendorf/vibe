"""Tests for doctor.py + commands/doctor_cmd.py (vibe-cli/13).

Contract (health report, not a parity unit — self-consistency is the bar):

- ``run_checks(root)`` reports one ``Check`` per skill / hook event / cursor /
  AGENTS.md / dependency, each ``ok`` or degraded (``warn``) with a fix hint.
- A missing dependency warns, names the dep + its degrade text + a fix command;
  the command still exits 0 by default and only ``--exit-code`` turns a warning
  into a nonzero exit.
- A missing settings.json hook entry, an absent AGENTS.md block, and an invalid
  cursor each degrade to a warning with a fix hint.
- A fully provisioned install reports every check ok and exits 0 in both modes.

Content assertions run against the structured ``Check`` objects (no rich
width-truncation risk); the CliRunner drives only exit-code behaviour.
"""

from __future__ import annotations

import json
from pathlib import Path

import typer
from typer.testing import CliRunner

from vibe import doctor, machine, markers
from vibe.commands import doctor_cmd
from vibe.provision import settings

runner = CliRunner()

_DEP_NAMES = ("superpowers", "feature-dev", "caveman")


def _by_id(checks: list[doctor.Check]) -> dict[str, doctor.Check]:
    return {c.id: c for c in checks}


def _write_agents_block(target: Path) -> None:
    body = markers.wrap(doctor._INSTRUCTIONS_MARKER, "managed instructions")
    (target / "AGENTS.md").write_text(f"# Project\n\n{body}\n", encoding="utf-8")


def _install_skills(target: Path) -> None:
    skills_dir = target / ".claude" / "skills"
    for name in doctor._skill_names():
        (skills_dir / name).mkdir(parents=True, exist_ok=True)
        (skills_dir / name / "SKILL.md").write_text("stub\n", encoding="utf-8")


def _install_deps(home: Path) -> None:
    for name in _DEP_NAMES:
        (home / ".claude" / "skills" / name).mkdir(parents=True, exist_ok=True)


def _make_healthy(target: Path) -> None:
    _install_skills(target)
    settings.merge(target)
    _write_agents_block(target)


def _local_app() -> typer.Typer:
    app = typer.Typer()
    doctor_cmd.register(app)
    return app


# --------------------------------------------------------------------------- #
# skill-name discovery reads the bundled asset tree
# --------------------------------------------------------------------------- #
def test_skill_names_from_assets() -> None:
    assert doctor._skill_names() == ["spec", "vibe"]


# --------------------------------------------------------------------------- #
# missing dependency -> warn + degrade + fix cmd, exit 0 / --exit-code nonzero
# --------------------------------------------------------------------------- #
def test_missing_dep_warns_with_degrade_and_fix(home_sandbox, target_project) -> None:
    _make_healthy(target_project)  # only deps are absent (empty sandbox home)

    checks = doctor.run_checks(target_project)
    by_id = _by_id(checks)

    dep = by_id["dep.superpowers"]
    assert dep.ok is False
    assert "superpowers" in dep.message
    assert "degrade:" in dep.message  # the deps.json degrade text rides along
    assert dep.fix and "vibe plugins install" in dep.fix


def test_missing_dep_exit_codes(home_sandbox, target_project) -> None:
    _make_healthy(target_project)

    default = runner.invoke(_local_app(), ["--root", str(target_project)])
    assert default.exit_code == 0  # warn-only by default

    gated = runner.invoke(
        _local_app(), ["--root", str(target_project), "--exit-code"]
    )
    assert gated.exit_code != 0  # a degraded dep gates CI


# --------------------------------------------------------------------------- #
# missing settings.json hook entry -> warn (discriminating: exactly the one)
# --------------------------------------------------------------------------- #
def test_missing_settings_hook_warns(home_sandbox, target_project) -> None:
    _make_healthy(target_project)
    _install_deps(home_sandbox)

    path = settings.settings_path(target_project)
    data = json.loads(path.read_text())
    data["hooks"].pop("Stop")  # drop exactly one wired event
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")

    by_id = _by_id(doctor.run_checks(target_project))
    stop = by_id["hook.Stop"]
    assert stop.ok is False
    assert "vibe-hook gate" in stop.message
    assert stop.fix and ("vibe init" in stop.fix or "vibe update" in stop.fix)
    # the untouched events still pass — a real per-event check, not a blanket miss
    assert by_id["hook.UserPromptSubmit"].ok is True
    assert by_id["hook.PreToolUse"].ok is True


def test_absent_settings_file_warns_all_hooks(home_sandbox, target_project) -> None:
    _install_skills(target_project)
    _write_agents_block(target_project)
    _install_deps(home_sandbox)

    by_id = _by_id(doctor.run_checks(target_project))
    for event in settings.commands():
        assert by_id[f"hook.{event}"].ok is False


# --------------------------------------------------------------------------- #
# absent AGENTS.md block -> warn
# --------------------------------------------------------------------------- #
def test_absent_agents_block_warns(home_sandbox, target_project) -> None:
    _install_skills(target_project)
    settings.merge(target_project)
    _install_deps(home_sandbox)
    # AGENTS.md exists but WITHOUT the managed marker block
    (target_project / "AGENTS.md").write_text("# Project\n\nno markers here\n", encoding="utf-8")

    check = _by_id(doctor.run_checks(target_project))["agents_md"]
    assert check.ok is False
    assert "vibe:instructions" in check.message
    assert check.fix and ("vibe init" in check.fix or "vibe update" in check.fix)


def test_absent_agents_file_warns(home_sandbox, target_project) -> None:
    _install_skills(target_project)
    settings.merge(target_project)
    _install_deps(home_sandbox)

    check = _by_id(doctor.run_checks(target_project))["agents_md"]
    assert check.ok is False


# --------------------------------------------------------------------------- #
# invalid cursor -> warn (absent cursor is ok — idle is normal)
# --------------------------------------------------------------------------- #
def test_invalid_cursor_warns(home_sandbox, target_project) -> None:
    _make_healthy(target_project)
    _install_deps(home_sandbox)

    from vibe import cursor as cursor_mod

    cpath = cursor_mod.path_for(target_project)
    cpath.parent.mkdir(parents=True, exist_ok=True)
    cpath.write_text(
        json.dumps({"flow": "bogus", "phase": "nope", "feature": None, "updated": ""}),
        encoding="utf-8",
    )

    check = _by_id(doctor.run_checks(target_project))["cursor"]
    assert check.ok is False
    assert check.fix


def test_malformed_cursor_json_warns(home_sandbox, target_project) -> None:
    _make_healthy(target_project)
    _install_deps(home_sandbox)

    from vibe import cursor as cursor_mod

    cpath = cursor_mod.path_for(target_project)
    cpath.parent.mkdir(parents=True, exist_ok=True)
    cpath.write_text("{not json", encoding="utf-8")

    check = _by_id(doctor.run_checks(target_project))["cursor"]
    assert check.ok is False


def test_absent_cursor_is_ok(home_sandbox, target_project) -> None:
    _make_healthy(target_project)
    _install_deps(home_sandbox)

    check = _by_id(doctor.run_checks(target_project))["cursor"]
    assert check.ok is True


def test_valid_cursor_is_ok(home_sandbox, target_project) -> None:
    _make_healthy(target_project)
    _install_deps(home_sandbox)

    from vibe import cursor as cursor_mod

    cursor_mod.write(cursor_mod.path_for(target_project), "feature.impl", "auth")

    check = _by_id(doctor.run_checks(target_project))["cursor"]
    assert check.ok is True
    assert "feature.impl" in check.message


# --------------------------------------------------------------------------- #
# healthy install -> all ok, exit 0 in both modes
# --------------------------------------------------------------------------- #
def test_healthy_all_ok(home_sandbox, target_project) -> None:
    _make_healthy(target_project)
    _install_deps(home_sandbox)

    checks = doctor.run_checks(target_project)
    assert checks  # non-empty
    assert all(c.ok for c in checks), [c.id for c in checks if not c.ok]
    assert doctor.has_failures(checks) is False


def test_healthy_exit_codes_both_modes(home_sandbox, target_project) -> None:
    _make_healthy(target_project)
    _install_deps(home_sandbox)

    default = runner.invoke(_local_app(), ["--root", str(target_project)])
    assert default.exit_code == 0

    gated = runner.invoke(
        _local_app(), ["--root", str(target_project), "--exit-code"]
    )
    assert gated.exit_code == 0


# --------------------------------------------------------------------------- #
# register(app) wiring contract + report renders
# --------------------------------------------------------------------------- #
def test_register_adds_doctor_command() -> None:
    app = _local_app()
    names = {c.name for c in app.registered_commands}
    assert "doctor" in names


def test_report_renders_and_returns_checks(home_sandbox, target_project) -> None:
    _make_healthy(target_project)
    _install_deps(home_sandbox)

    result = runner.invoke(_local_app(), ["--root", str(target_project)])
    assert result.exit_code == 0
    assert "vibe doctor" in result.output


def test_settings_command_lookup_matches_merge(home_sandbox, target_project) -> None:
    # the read-only presence check must agree with settings.merge's wiring
    settings.merge(target_project)
    data = json.loads(settings.settings_path(target_project).read_text())
    for event, command in settings.commands().items():
        assert doctor._hook_command_present(data, event, command)


def test_machine_states_drive_cursor_validity(home_sandbox, target_project) -> None:
    # a cursor at every legal target validates as ok
    from vibe import cursor as cursor_mod

    _make_healthy(target_project)
    _install_deps(home_sandbox)
    m = machine.default()
    for target in ("idle", "feature.impl"):
        assert target in m.states
        cursor_mod.write(cursor_mod.path_for(target_project), target, "x")
        check = _by_id(doctor.run_checks(target_project))["cursor"]
        assert check.ok is True
