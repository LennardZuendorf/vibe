"""``vibe status`` / ``vibe next`` / ``vibe go`` — orient and transition (vibe-cli/6).

Thin rich commands over the stdlib flow layer: :mod:`vibe.cursor` (read + atomic
set-state write) and :mod:`vibe.machine` (legal ``next`` lookups). ``status`` and
``next`` never mutate; ``go`` is legality-checked against ``machine.next_of`` for
the *current* state before it hands the write to ``cursor.write`` — a refused
transition names the legal options and leaves the cursor untouched, so an
illegal jump can never stamp a new state.

Exposes the ``register(app)`` wiring contract so ``app.py`` (unit 16) mounts the
commands without this module importing ``app.py``.
"""

from __future__ import annotations

from pathlib import Path

import typer
from rich.panel import Panel
from rich.table import Table

from vibe import cursor, machine
from vibe.errors import VibeError
from vibe.ui import theme
from vibe.ui.console import console

_ROOT_OPTION = typer.Option(
    None,
    "--root",
    help="Project root holding the flow cursor (default: current directory).",
    show_default=False,
    exists=False,
    file_okay=False,
    dir_okay=True,
)


def _cursor_path(root: Path | None) -> Path:
    """Resolve the cursor's ``state.json`` path under ``root`` (or the cwd)."""
    return cursor.path_for((root or Path.cwd()).resolve())


def _legal_next(state_key: str) -> list[str]:
    """Legal ``next`` states for ``state_key`` (empty for terminal/unknown)."""
    try:
        return machine.next_of(state_key)
    except VibeError:
        return []


def _current(path: Path) -> tuple[str, str | None]:
    """Return the ``(state_key, feature)`` for the cursor at ``path``.

    Degrades to the machine's initial state (``idle``) when the cursor is absent
    or malformed, mirroring ``detect-context.sh``'s current-state resolution.
    """
    try:
        cur = cursor.read(path)
    except VibeError:
        cur = None
    if cur is None:
        return machine.default().initial, None
    return cur.key, cur.feature


def status_command(root: Path = _ROOT_OPTION) -> None:
    """Show the current flow, phase, feature, and legal next transitions."""
    path = _cursor_path(root)
    state_key, feature = _current(path)
    flow, phase = machine.split_key(state_key)
    legal = _legal_next(state_key)

    grid = Table.grid(padding=(0, 2))
    grid.add_column(justify="right", style=theme.get_label_style(), no_wrap=True)
    grid.add_column(style=theme.get_value_style())
    grid.add_row("state", state_key)
    grid.add_row("flow", flow)
    grid.add_row("phase", phase)
    grid.add_row("feature", feature if feature else "—")
    grid.add_row("next", ", ".join(legal) if legal else "(none — terminal state)")

    console.print(
        Panel(
            grid,
            title="vibe status",
            title_align="left",
            border_style=theme.get_card_border_style(),
        )
    )


def next_command(root: Path = _ROOT_OPTION) -> None:
    """List the legal transitions from the current state."""
    path = _cursor_path(root)
    state_key, _ = _current(path)
    legal = _legal_next(state_key)

    if not legal:
        console.print(
            f"[{theme.get_warning_style()}]next:[/] "
            f"{state_key} is terminal — no legal transitions."
        )
        return

    console.print(
        f"Legal transitions from [{theme.get_accent_style()}]{state_key}[/]:",
        highlight=False,
    )
    for target in legal:
        console.print(f"  {target}", style=theme.get_value_style(), highlight=False)


def go_command(
    target: str = typer.Argument(
        ...,
        help="Target state (<flow>.<phase>, or 'idle') to transition to.",
    ),
    feature: str = typer.Option(
        None,
        "--feature",
        "-f",
        help="Feature to stamp; carries forward from the current cursor if omitted.",
        show_default=False,
    ),
    root: Path = _ROOT_OPTION,
) -> None:
    """Transition the cursor to ``target`` if it is a legal next state."""
    path = _cursor_path(root)
    current = cursor.current_state(path)
    legal = _legal_next(current)

    if target not in legal:
        console.print(
            f"[{theme.get_error_style()}]go:[/] "
            f"{target} is not a legal transition from {current}."
        )
        options = ", ".join(legal) if legal else "(none — terminal state)"
        console.print(
            f"[{theme.get_dim_style()}]legal next:[/] {options}",
            highlight=False,
        )
        raise typer.Exit(1)

    try:
        new = cursor.write(path, target, feature)
    except VibeError as exc:
        console.print(f"[{theme.get_error_style()}]go:[/] {exc}")
        raise typer.Exit(1) from exc

    line = f"[{theme.get_success_style()}]go:[/] {current} → {new.key}"
    if new.feature:
        line += f" [{theme.get_dim_style()}](feature: {new.feature})[/]"
    console.print(line, highlight=False)

    if new.flow == "feature" and not new.feature:
        console.print(
            f"[{theme.get_warning_style()}]go:[/] entering {new.key} with no feature set — "
            "pass --feature <name> to name it.",
            highlight=False,
        )


def register(app: typer.Typer) -> None:
    """Mount ``status`` / ``next`` / ``go`` onto ``app`` (wiring contract)."""
    app.command("status")(status_command)
    app.command("next")(next_command)
    app.command("go")(go_command)


__all__ = ["status_command", "next_command", "go_command", "register"]
