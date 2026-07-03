"""``vibe doctor`` — install health report (vibe-cli/13).

Thin rich wrapper over :mod:`vibe.doctor`: it runs the health checks against the
target project, renders the report, and — only when ``--exit-code`` is passed —
turns any degraded check into a nonzero exit for CI gating. The default is
warn-only (exit 0) so a broken install can still describe itself. Exposes the
``register(app)`` wiring contract so ``app.py`` (unit 16) mounts the command
without this module importing ``app.py``.
"""

from __future__ import annotations

from pathlib import Path

import typer

from vibe import doctor


def doctor_command(
    root: Path = typer.Option(
        None,
        "--root",
        help="Project root to inspect (default: current directory).",
        show_default=False,
        exists=False,
        file_okay=False,
        dir_okay=True,
    ),
    exit_code: bool = typer.Option(
        False,
        "--exit-code",
        help="Exit nonzero when any check is degraded (for CI gating).",
    ),
) -> None:
    """Report on the vibe install: skills, hooks, cursor, AGENTS.md, dependencies."""
    target_root = (root or Path.cwd()).resolve()
    checks = doctor.report(target_root)
    if exit_code and doctor.has_failures(checks):
        raise typer.Exit(1)


def register(app: typer.Typer) -> None:
    """Mount ``vibe doctor`` onto ``app`` (the shared wiring contract)."""
    app.command("doctor")(doctor_command)


__all__ = ["doctor_command", "register"]
