"""vibe — the self-hosting flow harness CLI (distribution: ``vibe-flow``).

Keep this module import-cheap: ``vibe-hook`` (the per-Edit guard) imports the
``vibe`` package on the hot path, so nothing here may pull in ``typer``,
``rich``, or ``pydantic``.
"""

__version__ = "0.1.0"

__all__ = ["__version__"]
