"""Error types for the flow half of the CLI.

Stdlib-only so it is safe to import on the ``vibe-hook`` hot path. Command
modules signal exit status by raising ``typer.Exit`` at the call site; they do
NOT raise this class — ``VibeError`` is for internal/library-layer failures.
"""


class VibeError(Exception):
    """Base exception for recoverable vibe library-layer failures."""


__all__ = ["VibeError"]
