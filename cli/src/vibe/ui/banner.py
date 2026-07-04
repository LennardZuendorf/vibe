"""The ``vibe`` rainbow wordmark shown atop ``vibe --help`` (vibe-cli UI).

Mirrors the ``indexed`` project's ``banner.py`` — a solid-block wordmark painted
with a gradient — with two deliberate departures:

1. **Vendored art.** The ``vibe`` block letters are a module constant, generated
   once with the ``art`` library (``tarty1`` font). The flow half ships zero
   runtime deps beyond ``typer``/``rich``; adding ``art`` just to redraw a fixed
   word is not worth a new dependency (and its output is deterministic anyway).
2. **Horizontal rainbow.** Colour runs left-to-right per column (over ~6 rows a
   top-to-bottom gradient shows almost no bands) across the six stops of the
   README wordmark (``docs/img/logo.svg``), so terminal and SVG match.

Rich-only. MUST NOT be imported on the ``vibe-hook`` hot path — the per-edit
guard's import purity is pinned by ``tests/test_hook_importcost.py``.
"""

from __future__ import annotations

from rich.text import Text

from vibe.ui.theme import get_dim_style, get_rainbow_palette

# Vendored ``tarty1`` rendering of "vibe" (art.text2art("vibe", "tarty1")).
BANNER_ART = (
    "██╗░░░██╗██╗██████╗░███████╗\n"
    "██║░░░██║██║██╔══██╗██╔════╝\n"
    "╚██╗░██╔╝██║██████╦╝█████╗░░\n"
    "░╚████╔╝░██║██╔══██╗██╔══╝░░\n"
    "░░╚██╔╝░░██║██████╦╝███████╗\n"
    "░░░╚═╝░░░╚═╝╚═════╝░╚══════╝"
)

# Glyphs that draw the letterforms (painted with the rainbow). ``░`` is the
# light-shade filler between/around letters and is rendered dim, not coloured.
_INK = set("█╗║╝╔╚═╦")
_SHADE = "░"


def _hex_to_rgb(color: str) -> tuple[int, int, int]:
    h = color.lstrip("#")
    return int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)


def _rgb_to_hex(rgb: tuple[int, int, int]) -> str:
    return "#{:02X}{:02X}{:02X}".format(*rgb)


def _column_colors(width: int) -> list[str]:
    """Interpolate the rainbow palette into one hex colour per column."""
    stops = [_hex_to_rgb(c) for c in get_rainbow_palette()]
    if width <= 1:
        return [_rgb_to_hex(stops[0])]
    segments = len(stops) - 1
    colors: list[str] = []
    for col in range(width):
        pos = col / (width - 1) * segments  # 0 .. segments
        idx = min(int(pos), segments - 1)
        frac = pos - idx
        a, b = stops[idx], stops[idx + 1]
        rgb = (
            round(a[0] + (b[0] - a[0]) * frac),
            round(a[1] + (b[1] - a[1]) * frac),
            round(a[2] + (b[2] - a[2]) * frac),
        )
        colors.append(_rgb_to_hex(rgb))
    return colors


def render_banner(color: bool = True) -> Text:
    """Return the ``vibe`` wordmark as a Rich ``Text``.

    ``color=True`` paints each letter glyph with a horizontal rainbow (per
    column) and dims the ``░`` shading. ``color=False`` returns the plain art
    for non-interactive output (pipes, CI) so no ANSI leaks into captured text.
    """
    lines = BANNER_ART.split("\n")
    width = max((len(line) for line in lines), default=0)
    col_colors = _column_colors(width) if color else []
    text = Text()
    for row, line in enumerate(lines):
        for col, char in enumerate(line):
            if not color or char == " ":
                text.append(char)
            elif char in _INK:
                text.append(char, style=col_colors[col])
            elif char == _SHADE:
                text.append(char, style=get_dim_style())
            else:
                text.append(char)
        if row < len(lines) - 1:
            text.append("\n")
    return text


def wants_top_banner(argv: list[str]) -> bool:
    """True only for the top-level ``vibe`` / ``vibe --help`` invocation.

    A subcommand (even ``vibe doctor --help``) returns False so the banner never
    prepends to piped or scripted command output.
    """
    if not argv:
        return True
    return argv[0] in ("-h", "--help")


def print_banner(console, *, color: bool = True) -> None:
    """Print the wordmark with a blank line above and below."""
    console.print()
    console.print(render_banner(color=color))
    console.print()


__all__ = ["BANNER_ART", "render_banner", "wants_top_banner", "print_banner"]
