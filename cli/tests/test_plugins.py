"""Tests for provision/plugins.py and commands/plugins_cmd.py (vibe-cli/12).

Contract this unit defines (not a parity unit — self-consistency is the bar):

- ``deps.json`` is the single source; ``load_deps`` parses it in order.
- A dependency is *installable* when its ``source`` yields a git marketplace
  (repo name = last path segment). ``superpowers`` -> ``superpowers@superpowers``,
  ``caveman`` -> ``caveman@caveman``; ``feature-dev`` (prose source) is NOT
  installable and produces no subprocess call, only an advisory note.
- ``install_deps`` runs ``marketplace add`` + ``install --scope project`` per
  installable dep; ``add_marketplaces`` runs only the ``marketplace add`` step.
- Graceful degrade: ``claude`` absent OR ``dry_run`` -> no invocation, manual
  commands emitted; a failing subprocess -> warn, record, continue (no crash).
"""

from __future__ import annotations

import subprocess

import typer
from typer.testing import CliRunner

from vibe.commands import plugins_cmd
from vibe.provision import plugins

runner = CliRunner()


class _FakeRun:
    """Records argv lists; returns/raises a chosen outcome per call."""

    def __init__(self, returncode: int = 0, raises: BaseException | None = None) -> None:
        self.calls: list[list[str]] = []
        self._returncode = returncode
        self._raises = raises

    def __call__(self, argv, *args, **kwargs):
        self.calls.append(list(argv))
        if self._raises is not None:
            raise self._raises
        return subprocess.CompletedProcess(list(argv), self._returncode, "", "")


def _present(monkeypatch, path: str = "/usr/local/bin/claude") -> None:
    monkeypatch.setattr(plugins, "claude_path", lambda: path)


def _absent(monkeypatch) -> None:
    monkeypatch.setattr(plugins, "claude_path", lambda: None)


# --------------------------------------------------------------------------- #
# deps.json parsing + derivation
# --------------------------------------------------------------------------- #
def test_load_deps_reads_bundled() -> None:
    names = [d.name for d in plugins.load_deps()]
    assert names == ["superpowers", "feature-dev", "caveman"]


def test_installable_derivation() -> None:
    by_name = {d.name: d for d in plugins.load_deps()}

    sp = by_name["superpowers"]
    assert sp.installable is True
    assert sp.marketplace == "superpowers"
    assert sp.plugin_ref == "superpowers@superpowers"

    cave = by_name["caveman"]
    assert cave.plugin_ref == "caveman@caveman"

    fd = by_name["feature-dev"]
    assert fd.installable is False
    assert fd.marketplace is None
    assert fd.plugin_ref is None


def test_argv_builders() -> None:
    by_name = {d.name: d for d in plugins.load_deps()}
    sp = by_name["superpowers"]

    assert plugins.marketplace_add_argv(sp) == [
        "claude", "plugin", "marketplace", "add", "https://github.com/obra/superpowers",
    ]
    assert plugins.install_argv(sp) == [
        "claude", "plugin", "install", "superpowers@superpowers", "--scope", "project",
    ]

    fd = by_name["feature-dev"]
    assert plugins.marketplace_add_argv(fd) is None
    assert plugins.install_argv(fd) is None


def test_manual_commands_installable_only() -> None:
    cmds = plugins.manual_commands()
    assert cmds == [
        "claude plugin marketplace add https://github.com/obra/superpowers",
        "claude plugin install superpowers@superpowers --scope project",
        "claude plugin marketplace add https://github.com/JuliusBrussee/caveman",
        "claude plugin install caveman@caveman --scope project",
    ]
    assert not any("feature-dev" in c for c in cmds)

    add_only = plugins.manual_commands(install=False)
    assert add_only == [
        "claude plugin marketplace add https://github.com/obra/superpowers",
        "claude plugin marketplace add https://github.com/JuliusBrussee/caveman",
    ]


# --------------------------------------------------------------------------- #
# install_deps — claude present
# --------------------------------------------------------------------------- #
def test_install_deps_present_correct_args_per_dep(monkeypatch) -> None:
    _present(monkeypatch)
    fake = _FakeRun()

    result = plugins.install_deps(run=fake)

    assert fake.calls == [
        ["claude", "plugin", "marketplace", "add", "https://github.com/obra/superpowers"],
        ["claude", "plugin", "install", "superpowers@superpowers", "--scope", "project"],
        ["claude", "plugin", "marketplace", "add", "https://github.com/JuliusBrussee/caveman"],
        ["claude", "plugin", "install", "caveman@caveman", "--scope", "project"],
    ]
    assert not any("feature-dev" in " ".join(argv) for argv in fake.calls)
    assert result.claude_present is True
    assert result.ran == fake.calls
    assert result.failed == []
    assert any("feature-dev" in note for note in result.notes)


def test_add_marketplaces_present_marketplace_only(monkeypatch) -> None:
    _present(monkeypatch)
    fake = _FakeRun()

    result = plugins.add_marketplaces(run=fake)

    assert fake.calls == [
        ["claude", "plugin", "marketplace", "add", "https://github.com/obra/superpowers"],
        ["claude", "plugin", "marketplace", "add", "https://github.com/JuliusBrussee/caveman"],
    ]
    assert not any("install" in argv for argv in fake.calls)
    assert result.ran == fake.calls


def test_install_deps_continues_on_nonzero(monkeypatch) -> None:
    _present(monkeypatch)
    fake = _FakeRun(returncode=1)

    result = plugins.install_deps(run=fake)

    # every installable command was still attempted despite failures
    assert len(fake.calls) == 4
    assert result.failed == result.ran == fake.calls


def test_install_deps_continues_on_raise(monkeypatch) -> None:
    _present(monkeypatch)
    fake = _FakeRun(raises=FileNotFoundError("claude vanished"))

    result = plugins.install_deps(run=fake)  # must not propagate

    assert len(fake.calls) == 4
    assert result.failed == fake.calls


# --------------------------------------------------------------------------- #
# install_deps — degrade paths (absent / dry-run)
# --------------------------------------------------------------------------- #
def test_install_deps_absent_prints_manual(monkeypatch, capsys) -> None:
    _absent(monkeypatch)
    fake = _FakeRun()

    result = plugins.install_deps(run=fake)

    assert fake.calls == []  # nothing invoked
    assert result.claude_present is False
    assert result.ran == []
    assert result.manual == plugins.manual_commands()

    out = capsys.readouterr().out
    for cmd in plugins.manual_commands():
        assert cmd in out
    assert "feature-dev" in out  # advisory note, never silently dropped


def test_install_deps_dry_run_no_invocation(monkeypatch, capsys) -> None:
    _present(monkeypatch)
    fake = _FakeRun()

    result = plugins.install_deps(dry_run=True, run=fake)

    assert fake.calls == []  # dry-run performs ZERO writes (init --dry-run relies on this)
    assert result.claude_present is True
    assert result.ran == []
    assert result.manual == plugins.manual_commands()

    out = capsys.readouterr().out
    assert "claude plugin install superpowers@superpowers --scope project" in out


# --------------------------------------------------------------------------- #
# commands/plugins_cmd.py — register(app) contract
# --------------------------------------------------------------------------- #
def _local_app() -> typer.Typer:
    app = typer.Typer()
    plugins_cmd.register(app)
    return app


def test_register_adds_plugins_group() -> None:
    app = _local_app()
    assert any(g.name == "plugins" for g in app.registered_groups)


def test_plugins_list_reads_deps() -> None:
    result = runner.invoke(_local_app(), ["plugins", "list"])
    assert result.exit_code == 0
    for name in ("superpowers", "feature-dev", "caveman"):
        assert name in result.output


def test_plugins_install_absent_prints_manual(monkeypatch) -> None:
    _absent(monkeypatch)
    result = runner.invoke(_local_app(), ["plugins", "install"])
    assert result.exit_code == 0
    assert "claude plugin install superpowers@superpowers --scope project" in result.output


def test_plugins_install_present_invokes(monkeypatch) -> None:
    _present(monkeypatch)
    fake = _FakeRun()
    monkeypatch.setattr(plugins.subprocess, "run", fake)

    result = runner.invoke(_local_app(), ["plugins", "install"])

    assert result.exit_code == 0
    assert ["claude", "plugin", "install", "superpowers@superpowers", "--scope", "project"] in fake.calls


def test_plugins_add_present_marketplace_only(monkeypatch) -> None:
    _present(monkeypatch)
    fake = _FakeRun()
    monkeypatch.setattr(plugins.subprocess, "run", fake)

    result = runner.invoke(_local_app(), ["plugins", "add"])

    assert result.exit_code == 0
    assert fake.calls == [
        ["claude", "plugin", "marketplace", "add", "https://github.com/obra/superpowers"],
        ["claude", "plugin", "marketplace", "add", "https://github.com/JuliusBrussee/caveman"],
    ]
