"""``vibe rules`` — regenerate the active-rules digest (vibe-cli/10).

Thin rich wrapper over :mod:`vibe.rules`: regenerates the managed
``vibe:active-rules`` block in the target project's ``CLAUDE.md`` / ``AGENTS.md``
from ``.spec/lessons.md``. Exposes the ``register(app)`` wiring contract so
``app.py`` (unit 16) can mount it without this module importing ``app.py``.
"""

from __future__ import annotations

from pathlib import Path

import typer

from vibe import rules
from vibe.markers import MarkerError
from vibe.ui import theme
from vibe.ui.console import console


def rules_command(
    root: Path = typer.Option(
        None,
        "--root",
        help="Project root holding .spec/lessons.md (default: current directory).",
        show_default=False,
        exists=False,
        file_okay=False,
        dir_okay=True,
    ),
) -> None:
    """Regenerate the active-rules digest from ``.spec/lessons.md``."""
    target_root = (root or Path.cwd()).resolve()
    lessons = target_root / ".spec" / "lessons.md"

    if not lessons.is_file():
        console.print(
            f"[{theme.get_warning_style()}]rules:[/] "
            f"{lessons} not found; writing an empty digest."
        )

    try:
        written = rules.write_rules(target_root)
    except MarkerError as exc:
        console.print(f"[{theme.get_error_style()}]rules:[/] {exc}")
        raise typer.Exit(1) from exc

    if not written:
        console.print(
            f"[{theme.get_warning_style()}]rules:[/] "
            f"no CLAUDE.md or AGENTS.md to update in {target_root}."
        )
        return

    for path in written:
        console.print(
            f"[{theme.get_success_style()}]rules:[/] updated {path.name}"
        )


def register(app: typer.Typer) -> None:
    """Mount ``vibe rules`` onto ``app`` (the shared wiring contract)."""
    app.command("rules")(rules_command)
