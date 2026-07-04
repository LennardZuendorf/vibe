"""The ``vibe`` rainbow wordmark shown atop ``vibe --help`` (vibe-cli UI).

Structural checks only — the art is a vendored constant, the coloring is a
horizontal rainbow, and the banner degrades to plain text off a TTY. The
hot-path import purity that keeps this off ``vibe-hook`` is guarded separately
by ``test_hook_importcost.py``.
"""

from __future__ import annotations

import subprocess
import sys

from rich.text import Text

from vibe.ui import banner
from vibe.ui.theme import get_rainbow_palette


def test_palette_has_six_rainbow_stops():
    palette = get_rainbow_palette()
    assert len(palette) == 6
    assert all(c.startswith("#") and len(c) == 7 for c in palette)


def test_banner_art_is_a_block_wordmark():
    assert "█" in banner.BANNER_ART
    assert banner.BANNER_ART.count("\n") >= 4  # multi-line wordmark


def test_render_banner_preserves_glyphs():
    text = banner.render_banner()
    assert isinstance(text, Text)
    assert "█" in text.plain


def test_render_banner_is_rainbow_not_monochrome():
    text = banner.render_banner()
    colors = {str(span.style) for span in text.spans if span.style}
    assert len(colors) > 3  # many distinct column colors => rainbow


def test_render_banner_plain_has_no_color():
    text = banner.render_banner(color=False)
    hues = {str(span.style) for span in text.spans if span.style and "#" in str(span.style)}
    assert not hues


def test_top_banner_gating():
    # Shows on the top-level help / no-args invocation only.
    assert banner.wants_top_banner([])
    assert banner.wants_top_banner(["--help"])
    assert banner.wants_top_banner(["-h"])
    # Never on a real subcommand (would corrupt piped/scripted output).
    assert not banner.wants_top_banner(["doctor"])
    assert not banner.wants_top_banner(["go", "feature.design"])
    assert not banner.wants_top_banner(["doctor", "--help"])


def _run_vibe(*args: str) -> subprocess.CompletedProcess:
    """Invoke ``main()`` in a fresh interpreter with a synthetic ``argv``."""
    code = (
        "import sys;"
        f"sys.argv = ['vibe', *{list(args)!r}];"
        "from vibe.app import main; main()"
    )
    return subprocess.run([sys.executable, "-c", code], capture_output=True, text=True)


def test_help_shows_banner_but_subcommand_help_does_not():
    top = _run_vibe("--help")
    assert "█" in top.stdout  # banner atop top-level help
    sub = _run_vibe("doctor", "--help")
    assert "█" not in sub.stdout  # never before a subcommand's output
