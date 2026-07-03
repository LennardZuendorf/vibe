"""``vibe plugins`` command group: list / install / add dependency plugins.

Thin rich wrapper over :mod:`vibe.provision.plugins`; the orchestration and
graceful-degrade logic live there. Exposes ``register(app)`` per the wiring
contract so ``app.py`` (unit 16) can mount the group without importing here
directly.
"""

from __future__ import annotations

import typer

from vibe.provision import plugins
from vibe.ui.console import console
from vibe.ui.theme import get_heading_style, get_value_style

plugins_app = typer.Typer(
    name="plugins",
    help="Manage the flow's Claude Code plugin dependencies.",
    no_args_is_help=True,
    add_completion=False,
)


@plugins_app.command("list")
def list_plugins() -> None:
    """List the plugin dependencies declared in deps.json."""
    console.print("Plugin dependencies (from deps.json):", style=get_heading_style())
    for dep in plugins.load_deps():
        ref = dep.plugin_ref if dep.plugin_ref is not None else "manual — no marketplace source"
        console.print(
            f"  {dep.name} [{dep.kind}] -> {ref}",
            style=get_value_style(),
            markup=False,
            highlight=False,
            soft_wrap=True,
        )


@plugins_app.command("install")
def install_plugins(
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Print the commands without running them."
    ),
) -> None:
    """Install every dependency plugin (marketplace add + install --scope project)."""
    plugins.install_deps(dry_run=dry_run)


@plugins_app.command("add")
def add_marketplaces(
    dry_run: bool = typer.Option(
        False, "--dry-run", help="Print the commands without running them."
    ),
) -> None:
    """Register the dependency marketplaces only (claude plugin marketplace add)."""
    plugins.add_marketplaces(dry_run=dry_run)


def register(app: typer.Typer) -> None:
    """Mount the ``plugins`` group onto the root app (wiring contract)."""
    app.add_typer(plugins_app, name="plugins")


__all__ = ["plugins_app", "register"]
