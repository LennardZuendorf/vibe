"""The ``vibe-hook`` entry point (second console_script) — the per-turn hooks.

This is the HOT PATH: Claude Code's ``PreToolUse`` guard fires this once per
Edit, dozens of times in an implementation turn, so its startup cost is the
whole perf budget. It therefore imports STDLIB ONLY (``argparse``, ``json``,
``os``, ``re``, ``shutil``, ``subprocess``, ``sys``) plus the stdlib-only
``vibe`` flow layer (``policy``/``orders``/``cursor``/``machine``) — never
``typer``, ``rich``, or ``pydantic``. Import-purity is pinned by a fresh-process
``sys.modules`` test (``test_hook_importcost.py``), the mechanism Decision 3
designates for this unit; an in-module assert is deliberately avoided because it
would false-positive whenever a process imports both ``vibe.app`` and this
module.

Three subcommands mirror the three bash adapter hooks verbatim:

* ``inject`` (UserPromptSubmit) — print the current state's orders (D12).
* ``guard`` (PreToolUse) — read the target path from stdin JSON, ask
  :mod:`vibe.policy`, and **exit 2** on a hard-block verdict (reason to stderr);
  a warn goes to stderr with exit 0; allow is silent exit 0.
* ``gate`` (Stop) — warn-only end-of-turn smells; ALWAYS exit 0.

The project is self-located via ``CLAUDE_PROJECT_DIR`` then an upward
``.spec``/``.git`` marker walk — never ``CLAUDE_PLUGIN_ROOT`` (settings.json
hooks do not receive it) and never the script's own path, so a symlinked
invocation resolves identically (the standing path-parity lesson).

Kept deliberately separate from ``vibe.app`` so the rich app never loads here.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

from vibe import cursor, machine, orders, policy
from vibe.errors import VibeError

_SUBCOMMANDS = ("inject", "guard", "gate")

_BLOCK_PREFIX = "block:"
_WARN_PREFIX = "warn:"

_SRC_RE = re.compile(r"(^|/)src/", re.MULTILINE)
_TESTS_RE = re.compile(r"(^|/)tests?/", re.MULTILINE)


def find_project_root(start: str | os.PathLike[str] | None = None) -> Path:
    """Locate the project root for a hook invocation.

    Ports the bash adapters' ``${CLAUDE_PROJECT_DIR:-…}`` plus ``orders.sh``'s
    ``find_repo_root`` marker walk: ``CLAUDE_PROJECT_DIR`` wins verbatim when set;
    otherwise walk upward from ``start`` (the CWD by default) for the first
    ancestor containing a ``.spec`` directory or a ``.git`` entry. ``CLAUDE_PLUGIN_ROOT``
    is intentionally ignored — settings.json hooks never receive it. Resolution
    never consults this module's own path, so a symlinked entry point resolves
    identically. Falls back to ``start`` when no marker is found.
    """
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env:
        return Path(env)
    base = (Path(start) if start is not None else Path.cwd()).resolve()
    for candidate in (base, *base.parents):
        if (candidate / ".spec").is_dir() or (candidate / ".git").exists():
            return candidate
    return base


def _skills_dir_for(root: Path) -> Path | None:
    """Prefer the target's provisioned ``.claude/skills`` for order blocks.

    Returns the target's ``.claude/skills`` when it carries a ``vibe/SKILL.md``
    (so a project may customise its orders), else ``None`` — which lets
    :func:`vibe.orders.resolve_current` fall back to the bundled ``_assets`` copy
    that ships with the package and is guaranteed present.
    """
    target = root / ".claude" / "skills"
    if (target / "vibe" / "SKILL.md").is_file():
        return target
    return None


def _read_stdin_path() -> str | None:
    """Extract the edited path from the PreToolUse stdin JSON, else ``None``.

    Mirrors ``pre-tool-use-guard.sh``'s
    ``.tool_input.file_path // .tool_input.notebook_path``: Edit/Write carry
    ``tool_input.file_path``; NotebookEdit carries ``tool_input.notebook_path``
    (a bare top-level ``notebook_path`` is accepted as a final fallback). Empty
    or unparseable stdin degrades to ``None`` (the caller then exits 0), never a
    session-ending failure.
    """
    try:
        raw = sys.stdin.read()
    except (OSError, ValueError):
        return None
    if not raw.strip():
        return None
    try:
        data = json.loads(raw)
    except (json.JSONDecodeError, ValueError):
        return None
    if not isinstance(data, dict):
        return None
    tool_input = data.get("tool_input")
    if isinstance(tool_input, dict):
        candidate = tool_input.get("file_path") or tool_input.get("notebook_path")
        if candidate:
            return str(candidate)
    top = data.get("notebook_path")
    if top:
        return str(top)
    return None


def _relativize(path: str, root: Path) -> str:
    """Strip a leading ``root/`` so policy's anchored globs match.

    Ports the guard's ``case "$PATH_IN" in "$ROOT"/*) …`` rewrite. Only strips
    when the path is genuinely under ``root``; the policy also matches ``*/``
    forms, so an unstripped absolute path still resolves correctly.
    """
    prefix = f"{root}{os.sep}"
    if path.startswith(prefix):
        return path[len(prefix):]
    return path


def _inject(root: Path) -> int:
    """Emit the current state's orders (UserPromptSubmit). Never fails."""
    try:
        text = orders.resolve_current(root, skills_dir=_skills_dir_for(root))
    except Exception:  # noqa: BLE001 - graceful degrade: never break a turn
        return 0
    print(text)
    return 0


def _guard(root: Path) -> int:
    """Enforce the write policy for a single edit (PreToolUse).

    Reads the target path from stdin, asks :func:`vibe.policy.decide`, and
    translates the verdict to the guard's exit convention: ``block`` → reason to
    stderr, **exit 2**; ``warn`` → reason to stderr, exit 0; ``allow`` → silent
    exit 0. Any resolution failure degrades to allow (exit 0).
    """
    path = _read_stdin_path()
    if not path:
        return 0

    rel = _relativize(path, root)
    try:
        verdict = policy.decide(rel, project_root=root)
    except Exception:  # noqa: BLE001 - graceful degrade to allow, matching bash `|| echo allow`
        return 0

    if verdict.startswith(_BLOCK_PREFIX):
        reason = verdict[len(_BLOCK_PREFIX):]
        print(f"vibe-guard: BLOCKED — {reason}", file=sys.stderr)
        print(
            "vibe-guard: transition with set-state.sh, or amend within the "
            "current state's write rules.",
            file=sys.stderr,
        )
        return 2
    if verdict.startswith(_WARN_PREFIX):
        reason = verdict[len(_WARN_PREFIX):]
        print(f"vibe-guard: warn — {reason}", file=sys.stderr)
        return 0
    return 0


def _git_changed(root: Path) -> str:
    """Return ``git status --porcelain`` for ``root``, or ``""`` when unavailable.

    Ports ``stop-gate.sh``'s ``git_changed``: no ``git`` on ``PATH`` or a
    non-work-tree degrades to an empty string (the affected smell is skipped),
    never a failure.
    """
    if shutil.which("git") is None:
        return ""
    try:
        inside = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--is-inside-work-tree"],
            capture_output=True,
            text=True,
        )
        if inside.returncode != 0:
            return ""
        status = subprocess.run(
            ["git", "-C", str(root), "status", "--porcelain"],
            capture_output=True,
            text=True,
        )
        if status.returncode != 0:
            return ""
        return status.stdout
    except (OSError, ValueError):
        return ""


def _gate(root: Path) -> int:
    """Warn-only end-of-turn smell checks (Stop). ALWAYS exits 0.

    Ports ``stop-gate.sh``'s three predicates verbatim (each prints to stderr and
    the hook still exits 0 — "earn the teeth" before any predicate blocks):
    src-without-tests in an impl/fix state, a verify-state evidence reminder, and
    a still-in-state advance nudge naming the legal ``next`` states.
    """
    state = cursor.current_state(cursor.path_for(root))
    try:
        next_states = machine.next_of(state)
    except VibeError:
        next_states = []
    next_str = ", ".join(next_states)

    def warn(message: str) -> None:
        print(f"vibe-gate: {message}", file=sys.stderr)

    if state in ("feature.impl", "quick.fix"):
        changed = _git_changed(root)
        if changed and _SRC_RE.search(changed) and not _TESTS_RE.search(changed):
            warn(
                f"in {state}, src changed with no test changes — TDD expects a "
                "reproducing/covering test. (warn-only)"
            )

    if state in ("feature.verify", "quick.verify"):
        warn(
            f"in {state} — confirm real evidence per unit ID and a code review "
            "before shipping. (warn-only)"
        )

    if state != "idle" and next_str:
        warn(
            f"still in {state} — when this phase's exit is met, advance with "
            f"set-state.sh (next: {next_str}). (warn-only)"
        )

    return 0


def build_parser() -> argparse.ArgumentParser:
    """Build the ``vibe-hook`` argument parser (inject|guard|gate)."""
    parser = argparse.ArgumentParser(
        prog="vibe-hook",
        description="Per-turn vibe flow hooks (stdlib-only hot path).",
    )
    sub = parser.add_subparsers(dest="command", metavar="{inject,guard,gate}")
    sub.add_parser("inject", help="emit per-turn orders (UserPromptSubmit)")
    sub.add_parser("guard", help="write-policy guard; exit 2 on block (PreToolUse)")
    sub.add_parser("gate", help="warn-only end-of-turn gate (Stop)")
    return parser


def main(argv: list[str] | None = None) -> int:
    """console_script entry point for ``vibe-hook``.

    Parses the subcommand, self-locates the project, and dispatches to the
    matching hook. Returns the process exit code (guard returns 2 on a hard
    block; every other path returns 0).
    """
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command is None:
        parser.print_help(sys.stderr)
        return 0
    root = find_project_root()
    dispatch = {"inject": _inject, "guard": _guard, "gate": _gate}
    return dispatch[args.command](root)


if __name__ == "__main__":  # pragma: no cover - direct invocation convenience
    sys.exit(main())
