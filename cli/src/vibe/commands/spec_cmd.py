"""``vibe spec`` wrapper + ``vibe check`` / ``vibe orders`` inspection (vibe-cli/15).

The spec framework stays standalone bash — this module only *wraps* it. ``vibe
spec validate`` and ``vibe spec setup`` shell to the spec skill's scripts,
preferring the target project's provisioned copy
(``<root>/.claude/skills/spec/scripts/<name>``) and falling back to the bundled
``_assets/skills/spec/scripts/<name>``; neither needs the flow runtime (no
cursor, no hooks). The subprocess output is streamed through verbatim so the
result is byte-identical to running the script directly, and its exit code
propagates.

``vibe check <path>`` and ``vibe orders`` surface the flow oracles for humans:
:func:`vibe.policy.decide` and :func:`vibe.orders.resolve`. They read the same
stdlib layer the ``vibe-hook`` guard uses, so the human verdict cannot drift from
the enforced one.

Exposes the ``register(app)`` wiring contract so ``app.py`` (unit 16) mounts the
commands without this module importing ``app.py``.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import typer

from vibe import cursor, orders, policy
from vibe.errors import VibeError
from vibe.ui import theme
from vibe.ui.console import console

_BUNDLED_SPEC = Path(__file__).resolve().parents[1] / "_assets" / "skills" / "spec"

_ROOT_OPTION = typer.Option(
    None,
    "--root",
    help="Project root to operate on (default: current directory).",
    show_default=False,
    exists=False,
    file_okay=False,
    dir_okay=True,
)

spec_app = typer.Typer(
    name="spec",
    help="Wrap the standalone spec framework (validate / setup).",
    no_args_is_help=True,
    add_completion=False,
)


def _resolve_script(root: Path, name: str) -> Path:
    """Locate spec script ``name`` — target's ``.claude`` copy first, else bundled.

    Prefers the project-provisioned ``<root>/.claude/skills/spec/scripts/<name>``
    so an updated target overrides the shipped copy; falls back to the bundled
    ``_assets/skills/spec/scripts/<name>`` when the target has none.
    """
    target = root / ".claude" / "skills" / "spec" / "scripts" / name
    if target.is_file():
        return target
    return _BUNDLED_SPEC / "scripts" / name


def _run_spec_script(root: Path | None, name: str) -> None:
    """Shell to spec script ``name`` in ``root``, stream output, propagate exit.

    Requires ``bash`` on ``PATH`` (the spec half is bash and standalone); a
    missing ``bash`` warns and exits non-zero rather than crashing. Output is
    written through verbatim so the result stays byte-identical to invoking the
    script directly.
    """
    target_root = (root or Path.cwd()).resolve()

    if shutil.which("bash") is None:
        console.print(
            f"[{theme.get_error_style()}]spec:[/] bash is required to run the "
            "spec scripts but was not found on PATH."
        )
        raise typer.Exit(127)

    script = _resolve_script(target_root, name)
    proc = subprocess.run(
        ["bash", str(script)],
        cwd=str(target_root),
        capture_output=True,
        text=True,
    )
    sys.stdout.write(proc.stdout)
    sys.stdout.flush()
    sys.stderr.write(proc.stderr)
    sys.stderr.flush()
    if proc.returncode != 0:
        raise typer.Exit(proc.returncode)


@spec_app.command("validate")
def validate_command(root: Path = _ROOT_OPTION) -> None:
    """Validate the target's ``.spec/`` via the spec skill's ``validate.sh``."""
    _run_spec_script(root, "validate.sh")


@spec_app.command("setup")
def setup_command(root: Path = _ROOT_OPTION) -> None:
    """Scaffold the target's ``.spec/`` via the spec skill's ``setup.sh``."""
    _run_spec_script(root, "setup.sh")


def _verdict_style(verdict: str) -> str:
    """Rich style for a ``decide`` verdict by its ``allow``/``warn``/``block`` tag."""
    if verdict.startswith("block:"):
        return theme.get_error_style()
    if verdict.startswith("warn:"):
        return theme.get_warning_style()
    return theme.get_success_style()


def check_command(
    path: str = typer.Argument(
        ...,
        help="Path to test the write policy for (as the guard would see it).",
    ),
    state: str = typer.Option(
        None,
        "--state",
        help="Evaluate as if in this state (default: resolve from the cursor).",
        show_default=False,
    ),
    root: Path = _ROOT_OPTION,
) -> None:
    """Print the write-policy verdict for ``path`` — the same oracle the guard uses."""
    target_root = (root or Path.cwd()).resolve()
    verdict = policy.decide(path, state, project_root=target_root)
    console.print(
        verdict,
        style=_verdict_style(verdict),
        markup=False,
        highlight=False,
        soft_wrap=True,
    )


def orders_command(
    state: str = typer.Option(
        None,
        "--state",
        help="Resolve orders for this state (default: resolve from the cursor).",
        show_default=False,
    ),
    feature: str = typer.Option(
        None,
        "--feature",
        "-f",
        help="Feature to interpolate into the orders (default: from the cursor).",
        show_default=False,
    ),
    root: Path = _ROOT_OPTION,
) -> None:
    """Print the per-turn orders for the current (or given) state."""
    target_root = (root or Path.cwd()).resolve()
    if state:
        if feature is None:
            # Mirror orders.sh: the cursor feature is interpolated regardless of
            # the state arg, so an explicit --state still names <feature>.
            try:
                cur = cursor.read(cursor.path_for(target_root))
            except VibeError:
                cur = None
            feature = cur.feature if cur is not None else None
        text = orders.resolve(state, feature)
    else:
        text = orders.resolve_current(target_root)
    console.print(text, markup=False, highlight=False, soft_wrap=True)


def register(app: typer.Typer) -> None:
    """Mount ``spec`` group + ``check`` / ``orders`` onto ``app`` (wiring contract)."""
    app.add_typer(spec_app, name="spec")
    app.command("check")(check_command)
    app.command("orders")(orders_command)


__all__ = [
    "spec_app",
    "validate_command",
    "setup_command",
    "check_command",
    "orders_command",
    "register",
]
