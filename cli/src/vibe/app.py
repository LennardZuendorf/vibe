"""The rich ``vibe`` command-line app (first console_script).

This is the human-facing entry point. It builds the root ``typer`` app with a
callback; individual command modules register themselves via the
``register(app: typer.Typer) -> None`` contract and are wired here by a later
unit. The registry is intentionally EMPTY at this stage — ``vibe --help`` still
runs and exits 0.

The per-Edit guard never loads this module; it goes through ``vibe.hook``
(stdlib-only) instead, so ``typer``/``rich`` startup cost stays off the hot path.
"""

from __future__ import annotations

import typer

from vibe import __version__
from vibe.commands import (
    doctor_cmd,
    flow_cmds,
    init_cmd,
    lifecycle_cmd,
    plugins_cmd,
    rules_cmd,
    spec_cmd,
)

app = typer.Typer(
    name="vibe",
    help="vibe — the self-hosting flow harness.",
    no_args_is_help=True,
    add_completion=False,
    rich_markup_mode="rich",
)


@app.callback()
def main_callback() -> None:
    """vibe — orient, transition, and provision the flow harness.

    Run ``vibe <command> --help`` for details on a command.
    """
    # The callback is load-bearing: it makes this a Typer *group*, so ``--help``
    # works even before any command is registered.


# Each command module registers itself via the ``register(app)`` contract.
for _module in (
    flow_cmds,
    init_cmd,
    lifecycle_cmd,
    doctor_cmd,
    spec_cmd,
    plugins_cmd,
    rules_cmd,
):
    _module.register(app)


def main() -> None:
    """console_script entry point for ``vibe`` (see ``[project.scripts]``)."""
    app()


__all__ = ["app", "main", "__version__"]
