"""Smoke tests: both entry points import and their ``--help`` exits 0."""

from __future__ import annotations

import pytest
from typer.testing import CliRunner

from vibe import __version__
from vibe.app import app
from vibe import hook

runner = CliRunner()


def test_version_is_set() -> None:
    assert __version__ == "0.1.0"


def test_app_help_exits_zero() -> None:
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0
    # The registry is empty at this stage: help runs but advertises no commands.
    assert "vibe" in result.output


def test_app_runs_with_empty_registry() -> None:
    # A group with only a callback is valid: bare invocation shows help and does
    # NOT raise "Could not get a command for this Typer instance". Both 0 and 2
    # mean "ran and showed help"; a crash would surface as a raised exception.
    result = runner.invoke(app, [])
    assert result.exit_code in (0, 2)
    assert not isinstance(result.exception, RuntimeError)


def test_hook_help_exits_zero() -> None:
    # argparse raises SystemExit(0) on --help rather than returning.
    with pytest.raises(SystemExit) as excinfo:
        hook.main(["--help"])
    assert excinfo.value.code in (0, None)


@pytest.mark.parametrize("sub", ["inject", "guard", "gate"])
def test_hook_subcommands_stub_exit_zero(sub: str) -> None:
    assert hook.main([sub]) == 0


def test_hook_no_subcommand_exits_zero() -> None:
    assert hook.main([]) == 0
