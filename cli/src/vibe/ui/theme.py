"""Design-system styles for the ``vibe`` CLI.

All colors live here as private constants; call sites use the ``get_*_style``
accessors so no command hardcodes a color. Mirrors the ``indexed`` theme
pattern. Rich-only — never imported on the ``vibe-hook`` hot path.
"""

# ---------------------------------------------------------------------------
# Palette
# ---------------------------------------------------------------------------
_ACCENT_COLOR = "#7C3AED"  # single accent (violet) for highlights and labels
_WHITE = "white"
_ERROR_COLOR = "red"
_WARNING_COLOR = "yellow"
_SUCCESS_COLOR = "green"
_DIM_COLOR = "#808080"  # neutral grey, stable across terminal themes

# Six rainbow stops shared with the README wordmark (docs/img/logo.svg) so the
# terminal banner and the SVG read as one brand. Interpolated across columns by
# the banner; used verbatim here as the anchor palette.
_RAINBOW_PALETTE = (
    "#FF4D4D",  # red
    "#FF9F45",  # orange
    "#FFD93D",  # yellow
    "#6BCB77",  # green
    "#4D96FF",  # blue
    "#9B5DE5",  # violet
)

# Text-hierarchy styles
_ACCENT_STYLE = f"bold {_ACCENT_COLOR}"
_TITLE_STYLE = f"bold underline {_ACCENT_COLOR}"
_HEADING_STYLE = f"bold {_WHITE}"
_LABEL_STYLE = f"bold {_ACCENT_COLOR}"
_VALUE_STYLE = f"not bold {_WHITE}"
_DIM_STYLE = f"not bold {_DIM_COLOR}"

# Status styles
_INFO_STYLE = _DIM_COLOR
_SUCCESS_STYLE = f"bold {_SUCCESS_COLOR}"
_ERROR_STYLE = f"bold {_ERROR_COLOR}"
_WARNING_STYLE = f"bold {_WARNING_COLOR}"


def get_accent_style() -> str:
    """Style for accent/emphasis text."""
    return _ACCENT_STYLE


def get_title_style() -> str:
    """Style for panel/card titles."""
    return _TITLE_STYLE


def get_heading_style() -> str:
    """Style for section headings and main titles."""
    return _HEADING_STYLE


def get_label_style() -> str:
    """Style for left-side labels."""
    return _LABEL_STYLE


def get_value_style() -> str:
    """Style for right-side values."""
    return _VALUE_STYLE


def get_dim_style() -> str:
    """Style for dim/secondary helper text."""
    return _DIM_STYLE


def get_info_style() -> str:
    """Style for informational messages."""
    return _INFO_STYLE


def get_success_style() -> str:
    """Style for success messages."""
    return _SUCCESS_STYLE


def get_warning_style() -> str:
    """Style for warning messages."""
    return _WARNING_STYLE


def get_error_style() -> str:
    """Style for error messages."""
    return _ERROR_STYLE


def get_rainbow_palette() -> list[str]:
    """Six rainbow stops matching the README wordmark (docs/img/logo.svg)."""
    return list(_RAINBOW_PALETTE)


def get_card_border_style() -> str:
    """Border style for panels/cards (dim, matching Typer's help chrome)."""
    return "dim"


def get_help_theme_styles() -> dict[str, str]:
    """Rich theme overrides for the Typer help menu."""
    return {"argparse.text": _ACCENT_STYLE}


__all__ = [
    "get_accent_style",
    "get_title_style",
    "get_heading_style",
    "get_label_style",
    "get_value_style",
    "get_dim_style",
    "get_rainbow_palette",
    "get_info_style",
    "get_success_style",
    "get_warning_style",
    "get_error_style",
    "get_card_border_style",
    "get_help_theme_styles",
]
