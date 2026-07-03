"""``vibe uninstall`` / ``vibe update`` — surgical, cursor-safe lifecycle (vibe-cli/14).

Both commands compose the same tested provisioning leaves that :mod:`vibe.commands.init_cmd`
uses, run in reverse (uninstall) or re-applied idempotently (update):

* **uninstall** removes only the artifacts ``init`` created — the per-file inverse via
  :func:`vibe.provision.skills.remove`, :func:`vibe.provision.settings.unmerge`, and
  :func:`vibe.provision.agents_md.unmerge` (which preserves user prose outside the managed
  markers), plus the managed ``vibe:active-rules`` digest block. It never touches ``.spec/``
  and, unless ``--yes`` is given, leaves the flow cursor in place.
* **update** re-provisions the managed files idempotently (skills refreshed, hook + AGENTS.md
  merges are no-ops when current, digest regenerated) while :func:`vibe.cursor.seed` preserves
  a live cursor.

Exposes the ``register(app)`` wiring contract.
"""

from __future__ import annotations

from pathlib import Path

import typer
from rich.panel import Panel
from rich.table import Table

from vibe import cursor, markers, rules
from vibe.errors import VibeError
from vibe.markers import MarkerError
from vibe.provision import agents_md, settings, skills
from vibe.ui import theme
from vibe.ui.console import console

_MANAGED_INSTRUCTION_FILES = ("CLAUDE.md", "AGENTS.md")


def _warn(action: str, message: str) -> None:
    console.print(f"[{theme.get_warning_style()}]{action}:[/] {message}", highlight=False)


def _render_summary(action: str, rows: list[tuple[str, str]]) -> None:
    grid = Table.grid(padding=(0, 2))
    grid.add_column(justify="right", style=theme.get_label_style(), no_wrap=True)
    grid.add_column(style=theme.get_value_style())
    for label, detail in rows:
        grid.add_row(label, detail)
    console.print(
        Panel(
            grid,
            title=f"vibe {action} — done",
            title_align="left",
            border_style=theme.get_card_border_style(),
        )
    )


def _strip_active_rules(root: Path) -> bool:
    """Remove the managed ``vibe:active-rules`` block from the instruction files.

    Operates only on the marker region (via the shared strict guard); any prose the
    user placed outside the markers is untouched. A ``CLAUDE.md`` symlinked onto
    ``AGENTS.md`` resolves to one file and is edited once.
    """
    changed = False
    seen: set[Path] = set()
    for name in _MANAGED_INSTRUCTION_FILES:
        f = root / name
        if not f.exists():
            continue
        real = f.resolve()
        if real in seen:
            continue
        seen.add(real)
        text = real.read_text(encoding="utf-8")
        if markers.has_region(text, rules.MARKER_NAME):
            real.write_text(markers.unmerge(text, rules.MARKER_NAME), encoding="utf-8")
            changed = True
    return changed


def uninstall_command(
    path: Path = typer.Argument(
        None, help="Project root to clean (default: current directory).", show_default=False
    ),
    yes: bool = typer.Option(
        False, "--yes", "-y", help="Also remove the flow cursor (state.json)."
    ),
) -> None:
    """Surgically remove the vibe artifacts, preserving user content and ``.spec/``."""
    root = (path or Path.cwd()).resolve()
    console.print(
        f"[{theme.get_heading_style()}]vibe uninstall[/] "
        f"[{theme.get_dim_style()}]{root}[/]",
        highlight=False,
    )

    rows: list[tuple[str, str]] = []

    removed = skills.remove(root)
    rows.append(("skills", f"{len(removed)} shipped files removed"))

    try:
        rows.append(("settings.json", "vibe hooks removed" if settings.unmerge(root) else "no vibe hooks"))
    except ValueError as exc:
        _warn("uninstall", f"settings.json left intact (unparseable JSON): {exc}")
        rows.append(("settings.json", "left intact (see warning)"))

    try:
        result = agents_md.unmerge(root)
        rows.append(("AGENTS.md", f"{result.action} (prose preserved)"))
    except (VibeError, MarkerError) as exc:
        _warn("uninstall", f"AGENTS.md block not removed: {exc}")
        rows.append(("AGENTS.md", "left in place (see warning)"))

    rows.append(("active-rules", "block removed" if _strip_active_rules(root) else "absent"))

    cursor_path = cursor.path_for(root)
    if yes:
        if cursor_path.exists():
            cursor_path.unlink()
            _prune_cursor_dirs(cursor_path)
        rows.append(("cursor", "removed (--yes)"))
    else:
        rows.append(("cursor", "preserved (pass --yes to remove)"))

    rows.append((".spec/", "untouched"))
    _render_summary("uninstall", rows)


def _prune_cursor_dirs(cursor_path: Path) -> None:
    """Prune the now-empty ``.agents/skills/vibe`` chain after cursor removal."""
    directory = cursor_path.parent
    for _ in range(3):  # vibe -> skills -> .agents
        try:
            directory.rmdir()
        except OSError:
            break
        directory = directory.parent


def update_command(
    path: Path = typer.Argument(
        None, help="Project root to update (default: current directory).", show_default=False
    ),
) -> None:
    """Re-provision the managed files idempotently, preserving the live cursor."""
    root = (path or Path.cwd()).resolve()
    console.print(
        f"[{theme.get_heading_style()}]vibe update[/] "
        f"[{theme.get_dim_style()}]{root}[/]",
        highlight=False,
    )

    rows: list[tuple[str, str]] = []

    skills.install(root)
    rows.append(("skills", "refreshed"))

    try:
        rows.append(("settings.json", "merged" if settings.merge(root) else "current"))
    except ValueError as exc:
        _warn("update", f"settings.json left intact (unparseable JSON): {exc}")
        rows.append(("settings.json", "left intact (see warning)"))

    try:
        result = agents_md.merge(root)
        if result.warning:
            _warn("update", result.warning)
        rows.append(("AGENTS.md", result.action))
    except (VibeError, MarkerError) as exc:
        _warn("update", f"AGENTS.md not merged: {exc}")
        rows.append(("AGENTS.md", "left in place (see warning)"))

    try:
        rules.write_rules(root)
        rows.append(("active-rules", "regenerated"))
    except MarkerError as exc:
        _warn("update", f"active-rules not regenerated: {exc}")

    kept = cursor.seed(cursor.path_for(root))
    rows.append(("cursor", f"preserved ({kept.key})"))

    _render_summary("update", rows)


def register(app: typer.Typer) -> None:
    """Mount ``vibe uninstall`` and ``vibe update`` (the shared wiring contract)."""
    app.command("uninstall")(uninstall_command)
    app.command("update")(update_command)


__all__ = ["uninstall_command", "update_command", "register"]
