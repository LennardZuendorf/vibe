"""Orchestrate Claude Code plugin provisioning from the bundled ``deps.json``.

Reads the single source of external skill/subagent dependencies and drives
``claude plugin marketplace add`` + ``claude plugin install <p>@<mkt> --scope
project`` for each dependency that carries a git marketplace source. Degrades
gracefully by repo convention (warn, never hard-fail): when the ``claude``
binary is absent, when ``dry_run`` is requested, or when a subprocess fails
mid-run, it surfaces the manual commands and keeps going instead of crashing.

Marketplace derivation: ``deps.json`` records only a ``source`` per dependency,
so the marketplace name is the last path segment of a git source
(``https://github.com/obra/superpowers`` -> ``superpowers``) and the install
ref is ``<name>@<marketplace>``. A dependency whose source is prose (no git
host, e.g. ``feature-dev``) is not auto-installable; it is surfaced as an
advisory note rather than installed by a bare name.

This is a provisioning (rich-path) module, not a ``vibe-hook`` hot-path module:
importing ``rich`` here is intentional and allowed.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
from dataclasses import dataclass, field
from importlib import resources
from pathlib import Path
from typing import Callable, Optional

from vibe.ui.console import console
from vibe.ui.theme import (
    get_dim_style,
    get_heading_style,
    get_success_style,
    get_warning_style,
)

CLAUDE = "claude"

_OWNER_REPO = re.compile(r"^[\w.-]+/[\w.-]+$")
_GIT_SCHEMES = ("http://", "https://", "git@", "ssh://", "git://")


@dataclass(frozen=True)
class Dep:
    """One external dependency declared in ``deps.json``."""

    name: str
    kind: str
    source: str
    required_by: tuple[str, ...]
    degrade: str

    @property
    def marketplace(self) -> Optional[str]:
        """Marketplace name derived from a git ``source``, else ``None``."""
        return _repo_name(self.source)

    @property
    def installable(self) -> bool:
        """True when the source yields a marketplace we can add + install from."""
        return self.marketplace is not None

    @property
    def plugin_ref(self) -> Optional[str]:
        """The ``<name>@<marketplace>`` install ref, or ``None`` if not installable."""
        mkt = self.marketplace
        return f"{self.name}@{mkt}" if mkt is not None else None


@dataclass
class PluginResult:
    """Outcome of a provisioning pass, foldable into an init/update summary."""

    claude_present: bool
    ran: list[list[str]] = field(default_factory=list)
    failed: list[list[str]] = field(default_factory=list)
    manual: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)


def _repo_name(source: str) -> Optional[str]:
    s = (source or "").strip()
    if not s:
        return None
    if not (s.startswith(_GIT_SCHEMES) or _OWNER_REPO.match(s)):
        return None
    seg = s.rstrip("/").split("/")[-1]
    if seg.endswith(".git"):
        seg = seg[:-4]
    seg = seg.strip()
    return seg or None


def _deps_text(path: Optional[Path]) -> str:
    if path is not None:
        return Path(path).read_text(encoding="utf-8")
    return (resources.files("vibe") / "_assets" / "deps.json").read_text(encoding="utf-8")


def load_deps(path: Optional[Path] = None) -> list[Dep]:
    """Parse ``deps.json`` (bundled by default) into ``Dep`` objects, in order."""
    data = json.loads(_deps_text(path))
    deps: list[Dep] = []
    for entry in data.get("deps", []):
        deps.append(
            Dep(
                name=entry["name"],
                kind=entry.get("kind", ""),
                source=entry.get("source", ""),
                required_by=tuple(entry.get("required_by", [])),
                degrade=entry.get("degrade", ""),
            )
        )
    return deps


def claude_path() -> Optional[str]:
    """Absolute path to the ``claude`` binary on ``PATH``, or ``None`` if absent."""
    return shutil.which(CLAUDE)


def marketplace_add_argv(dep: Dep) -> Optional[list[str]]:
    """Argv for ``claude plugin marketplace add <source>``, or ``None``."""
    if not dep.installable:
        return None
    return [CLAUDE, "plugin", "marketplace", "add", dep.source]


def install_argv(dep: Dep, *, scope: str = "project") -> Optional[list[str]]:
    """Argv for ``claude plugin install <p>@<mkt> --scope <scope>``, or ``None``."""
    ref = dep.plugin_ref
    if ref is None:
        return None
    return [CLAUDE, "plugin", "install", ref, "--scope", scope]


def manual_commands(
    deps: Optional[list[Dep]] = None, *, install: bool = True
) -> list[str]:
    """The exact shell command strings a user would run by hand, in order.

    Includes only installable dependencies. When ``install`` is False, only the
    ``marketplace add`` step is emitted (mirrors ``add_marketplaces``).
    """
    deps = load_deps() if deps is None else deps
    cmds: list[str] = []
    for dep in deps:
        add = marketplace_add_argv(dep)
        if add is None:
            continue
        cmds.append(" ".join(add))
        if install:
            inst = install_argv(dep)
            if inst is not None:
                cmds.append(" ".join(inst))
    return cmds


def _argvs_for(dep: Dep, *, install: bool) -> list[list[str]]:
    argvs: list[list[str]] = []
    add = marketplace_add_argv(dep)
    if add is not None:
        argvs.append(add)
    if install:
        inst = install_argv(dep)
        if inst is not None:
            argvs.append(inst)
    return argvs


def _invoke(run: Callable[..., object], argv: list[str]) -> bool:
    line = " ".join(argv)
    try:
        proc = run(argv, capture_output=True, text=True)
    except (OSError, subprocess.SubprocessError) as exc:
        console.print(f"  failed: {line} ({exc})", style=get_warning_style(), soft_wrap=True)
        return False
    if getattr(proc, "returncode", 0) != 0:
        console.print(f"  failed: {line}", style=get_warning_style(), soft_wrap=True)
        return False
    console.print(f"  ran: {line}", style=get_dim_style(), soft_wrap=True)
    return True


def _print_manual(result: PluginResult, *, dry_run: bool) -> None:
    if dry_run:
        console.print("[dry-run] plugin commands (not executed):", style=get_dim_style())
    else:
        console.print(
            "claude CLI not found on PATH — run these manually:",
            style=get_warning_style(),
        )
    for cmd in result.manual:
        console.print(f"  {cmd}", markup=False, highlight=False, soft_wrap=True)
    _print_notes(result)


def _print_notes(result: PluginResult) -> None:
    for note in result.notes:
        console.print(f"  note: {note}", style=get_dim_style(), markup=False, soft_wrap=True)


def _provision(
    *,
    install: bool,
    deps: Optional[list[Dep]] = None,
    dry_run: bool = False,
    run: Optional[Callable[..., object]] = None,
) -> PluginResult:
    deps = load_deps() if deps is None else deps
    run = run if run is not None else subprocess.run
    present = claude_path() is not None
    result = PluginResult(claude_present=present)

    for dep in deps:
        if not dep.installable:
            result.notes.append(
                f"{dep.name}: no git marketplace source in deps.json; install manually if needed"
            )

    if dry_run or not present:
        result.manual = manual_commands(deps, install=install)
        _print_manual(result, dry_run=dry_run)
        return result

    console.print(
        "Installing plugin dependencies:" if install else "Registering plugin marketplaces:",
        style=get_heading_style(),
    )
    for dep in deps:
        for argv in _argvs_for(dep, install=install):
            result.ran.append(argv)
            if not _invoke(run, argv):
                result.failed.append(argv)

    if result.failed:
        console.print(
            f"{len(result.failed)} plugin command(s) failed — see warnings above",
            style=get_warning_style(),
        )
    else:
        console.print("plugin dependencies provisioned", style=get_success_style())
    _print_notes(result)
    return result


def install_deps(
    deps: Optional[list[Dep]] = None,
    *,
    dry_run: bool = False,
    run: Optional[Callable[..., object]] = None,
) -> PluginResult:
    """Add each marketplace and install each dependency plugin (``--scope project``).

    Graceful: ``claude`` absent or ``dry_run`` -> emit manual commands, no
    invocation; a failing command -> warn and continue to the next.
    """
    return _provision(install=True, deps=deps, dry_run=dry_run, run=run)


def add_marketplaces(
    deps: Optional[list[Dep]] = None,
    *,
    dry_run: bool = False,
    run: Optional[Callable[..., object]] = None,
) -> PluginResult:
    """Register each dependency's marketplace only (``marketplace add`` step)."""
    return _provision(install=False, deps=deps, dry_run=dry_run, run=run)


__all__ = [
    "CLAUDE",
    "Dep",
    "PluginResult",
    "add_marketplaces",
    "claude_path",
    "install_argv",
    "install_deps",
    "load_deps",
    "manual_commands",
    "marketplace_add_argv",
]
