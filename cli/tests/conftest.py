"""Shared pytest fixtures for the vibe CLI suite.

Collection-time discipline: this module imports NO ``vibe`` unit module. It
depends only on the stdlib and pytest, and resolves every path from this file's
own location so fixtures work regardless of install layout.

Layout anchors (this file is ``cli/tests/conftest.py``)::

    _TESTS_DIR    = cli/tests
    _CLI_DIR      = cli
    _REPO_ROOT    = <repo root>        (parent of cli/)
    _ASSETS_DIR   = cli/src/vibe/_assets
    _FLOW_SCRIPTS = <repo root>/flow/scripts
"""

from __future__ import annotations

import json
import shutil
import subprocess
from collections import namedtuple
from pathlib import Path

import pytest

_TESTS_DIR = Path(__file__).resolve().parent
_CLI_DIR = _TESTS_DIR.parent
_REPO_ROOT = _CLI_DIR.parent
_ASSETS_DIR = _CLI_DIR / "src" / "vibe" / "_assets"
_FLOW_SCRIPTS = _REPO_ROOT / "flow" / "scripts"

# Result of running a bundled bash script: stdout/exit are the contract the
# spec names; stderr rides along for guard-reason assertions in later units.
BashResult = namedtuple("BashResult", ["stdout", "stderr", "returncode"])


@pytest.fixture
def home_sandbox(tmp_path, monkeypatch):
    """Redirect ``Path.home()`` and ``$HOME`` to a throwaway temp dir.

    Provisioning code writes into ``HOME``; this keeps every test off the dev
    tree. ``monkeypatch`` restores both after the test.
    """
    home = tmp_path / "home"
    home.mkdir()
    monkeypatch.setattr(Path, "home", staticmethod(lambda: home))
    monkeypatch.setenv("HOME", str(home))
    return home


@pytest.fixture
def target_project(tmp_path):
    """Return a bare (empty) temp directory the provisioning tests write into."""
    target = tmp_path / "target"
    target.mkdir()
    return target


@pytest.fixture
def bash_ref():
    """Run a named script from the repo's ``flow/scripts/`` for parity tests.

    Returns a ``run(script_name, *args, cwd=None, input=None) -> BashResult``
    helper. Skips the whole test with a clear message when ``bash`` or ``jq``
    is unavailable (the parity origins depend on both).
    """
    if shutil.which("bash") is None or shutil.which("jq") is None:
        pytest.skip("bash and jq are required for parity/bash-reference tests")

    def run(script_name, *args, cwd=None, input=None) -> BashResult:
        script = _FLOW_SCRIPTS / script_name
        if not script.exists():
            pytest.skip(f"bash reference script not found: {script}")
        proc = subprocess.run(
            ["bash", str(script), *args],
            capture_output=True,
            text=True,
            cwd=str(cwd) if cwd is not None else None,
            input=input,
        )
        return BashResult(proc.stdout, proc.stderr, proc.returncode)

    return run


@pytest.fixture
def sample_lessons() -> str:
    """A small ``.spec/lessons.md``-style document: 6 entries, one pinned.

    Faithful to the real format (``### Title`` / ``**Rule:**`` / ``**Date:**`` /
    optional ``**Pinned-by:**``) so rules-digest tests exercise the pinned-first
    ordering and the cap-at-5 selection.
    """
    return (
        "# Lessons\n"
        "\n"
        "Mistakes made and rules to prevent repeating them.\n"
        "\n"
        "### Warn-first validators, then migrate\n"
        "**Pattern:** New checks shipped as errors blocked the harness.\n"
        "**Rule:** Ship structural validators warn-first; promote to error only "
        "after live specs are migrated.\n"
        "**Tags:** spec, validate\n"
        "**Date:** 2026-06-06\n"
        "\n"
        "### Single-source the per-turn orders\n"
        "**Pattern:** Orders lived in both the machine and the skill and drifted.\n"
        "**Rule:** Author orders once, in the linked skill, as an extractable "
        "block; carry inject:null in the machine.\n"
        "**Tags:** vibe-flow, d12\n"
        "**Date:** 2026-06-18\n"
        "\n"
        "### Preserve per-project runtime state across a re-copy\n"
        "**Pattern:** Re-install reset a live cursor to idle while claiming "
        "idempotence.\n"
        "**Rule:** Snapshot the cursor before a managed-file refresh and restore "
        "it after; seed only when absent.\n"
        "**Tags:** install, idempotency, cursor\n"
        "**Pinned-by:** vibe-cli\n"
        "**Date:** 2026-06-20\n"
        "\n"
        "### Uninstall must be surgical\n"
        "**Pattern:** A blanket rm of a shared dir destroyed co-located user "
        "files.\n"
        "**Rule:** Invert the install per file; prune only emptied dirs; pair "
        "with a discriminating test.\n"
        "**Tags:** uninstall, surgical\n"
        "**Date:** 2026-06-24\n"
        "\n"
        "### Scripts self-locate by marker search\n"
        "**Pattern:** Fixed ``..`` hop counts broke under compat symlinks.\n"
        "**Rule:** Walk up for a ``.spec``/``.git`` marker; pin with a "
        "path-parity test.\n"
        "**Tags:** scripts, self-location\n"
        "**Date:** 2026-06-28\n"
        "\n"
        "### Offer the superpower, then self-suffice\n"
        "**Pattern:** Skills either blocked on an executor or skipped the offer.\n"
        "**Rule:** Offer the optimal executor first; self-execute from the "
        "constraint docs if declined or unavailable.\n"
        "**Tags:** skills, design\n"
        "**Date:** 2026-07-01\n"
    )


@pytest.fixture
def machine_data() -> dict:
    """The bundled ``_assets/state-machine.json`` parsed as a dict."""
    return json.loads((_ASSETS_DIR / "state-machine.json").read_text())
