"""Unit 7 — hot-path import purity for ``vibe.hook`` (Decision 3).

The ``PreToolUse`` guard runs ``vibe-hook`` once per Edit, so importing
:mod:`vibe.hook` must NOT pull in ``typer``, ``rich``, or ``pydantic``. This is
checked in a FRESH interpreter: an in-process ``sys.modules`` probe would be
polluted by sibling tests that import the rich ``typer`` app.
"""

from __future__ import annotations

import subprocess
import sys

_HEAVY = ("typer", "rich", "pydantic")


def test_importing_hook_loads_no_heavy_deps():
    probe = (
        "import sys\n"
        "import vibe.hook\n"
        f"heavy = {_HEAVY!r}\n"
        "leaked = sorted(m for m in heavy if m in sys.modules)\n"
        "assert not leaked, 'hot-path import leaked: ' + ', '.join(leaked)\n"
        "print('ok')\n"
    )
    proc = subprocess.run(
        [sys.executable, "-c", probe],
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, proc.stderr
    assert proc.stdout.strip() == "ok"
