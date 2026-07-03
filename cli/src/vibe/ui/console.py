"""Shared Rich console for the ``vibe`` CLI.

A single ``Console`` instance backs all rich output so that Live/Progress/Status
displays never conflict with one another. Non-interactive environments (piped
output, CI, ``CliRunner``) degrade gracefully via ``Console.is_terminal``
auto-detection. This module is rich-only and MUST NOT be imported on the
``vibe-hook`` hot path.
"""

from rich.console import Console

# Single shared console — ALL rich CLI output goes through this instance.
# Uses stdout (default) so Typer's CliRunner captures it in tests.
console = Console()


def is_interactive() -> bool:
    """Return True when the shared console is attached to a real terminal."""
    return console.is_terminal


__all__ = ["console", "is_interactive"]
